//! 真实调度闭环（P0-1）：周期性扫描 `jobs` 表中 `pending` 作业，原子认领 → 配额预留
//! → GPU 分配 → 置 `running` → 模拟执行 → 释放 GPU/配额 → 置 `completed`。
//!
//! # 设计动机
//!
//! 既有 [`crate::services::priority_scheduler`] 是纯内存态 mock，从未在进程内接线，
//! 且执行体仅是 `tokio::time::sleep`，与 DB 的 `jobs` 表完全脱钩——这就是"有 mock
//! 无调度闭环"的根因。本模块直接消费 DB 中真实作业，是生产进程唯一启动的调度闭环。
//!
//! # 关键不变量
//!
//! - **认领原子性**：`UPDATE jobs SET status='running' WHERE id=? AND status='pending'`，
//!   `rows_affected==0` 即被其他 worker 认领或已非 pending，天然防并发重复派发
//!   （单 worker 串行 + 语句级原子双重保障）。
//! - **预留失败回退**：配额或 GPU 不足时，已预留的资源被释放，作业回退 `pending`，
//!   下一轮重试（背压语义，不丢弃作业）。
//! - **崩溃恢复（reconcile）**：启动期扫描 `running` 作业——仍持 active GPU 分配则
//!   重新挂接完成逻辑（避免重复分配）；无 active 分配且超过宽限期则回退 `pending`。
//! - **执行体为模拟**：K8s 真实提交未接入，用 `simulated_run_ms` 短时睡眠模拟一次运行，
//!   便于观测闭环；真实执行接入后仅需替换 `complete_running_job` 前的等待段。

use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use tokio::task::JoinHandle;

use crate::config::Config;
use crate::db::DatabasePool;
use crate::error::AppResult;
use crate::models::job::{self, Job};
use crate::services::gpu;
use crate::services::quota::{self, QuotaRequest};

/// 调度闭环运行配置。
#[derive(Debug, Clone)]
pub struct DispatchConfig {
    pub enabled: bool,
    /// 轮询间隔（毫秒）。
    pub interval_ms: u64,
    /// 模拟执行时长（毫秒）。
    pub simulated_run_ms: u64,
    /// 单轮最多认领数。
    pub batch_size: usize,
    /// 同时 running 的最大作业数（并发上限）。
    pub max_concurrent: usize,
    /// 孤儿 running 作业回退宽限（秒）。
    pub reconcile_grace_seconds: i64,
}

impl DispatchConfig {
    /// 从全局 [`Config`] 派生。
    pub fn from_config(cfg: &Config) -> Self {
        Self {
            enabled: cfg.dispatch_enabled,
            interval_ms: cfg.dispatch_interval_ms,
            simulated_run_ms: cfg.dispatch_simulated_run_ms,
            batch_size: cfg.dispatch_batch_size,
            max_concurrent: cfg.dispatch_max_concurrent,
            reconcile_grace_seconds: cfg.dispatch_reconcile_grace_seconds,
        }
    }
}

/// 启动后台调度闭环，返回 `JoinHandle`（优雅关闭时 `abort`）。
pub fn start_dispatch_loop(pool: DatabasePool, config: DispatchConfig) -> JoinHandle<()> {
    tokio::spawn(async move {
        tracing::info!(
            interval_ms = config.interval_ms,
            simulated_run_ms = config.simulated_run_ms,
            max_concurrent = config.max_concurrent,
            "dispatch loop started"
        );

        // 崩溃恢复：先 reconcile 孤儿 / 遗留 running 作业。
        if let Err(e) = reconcile(&pool, &config).await {
            tracing::warn!(error = %e, "dispatch reconcile failed");
        }

        let mut ticker =
            tokio::time::interval(Duration::from_millis(config.interval_ms.max(100)));
        // `interval` 首 tick 立即触发，先吞掉一次以避免启动即密集派发。
        ticker.tick().await;
        loop {
            ticker.tick().await;
            if let Err(e) = dispatch_tick(&pool, &config).await {
                tracing::warn!(error = %e, "dispatch tick failed");
            }
        }
    })
}

/// 单轮调度：受并发上限约束地认领并派发 pending 作业。
async fn dispatch_tick(pool: &DatabasePool, config: &DispatchConfig) -> AppResult<()> {
    // 1) 当前 running 数量（控制并发上限）。
    let running: i64 = crate::with_db!(
        pool,
        "SELECT COUNT(*) FROM jobs WHERE status = 'running' AND deleted_at IS NULL",
        |db_s, db_e| crate::db::query_scalar_db(db_e, db_s)
            .fetch_one(db_e)
            .await?
    );
    let running = running.max(0) as usize;
    if running >= config.max_concurrent {
        return Ok(());
    }
    let budget = (config.max_concurrent - running).min(config.batch_size).max(1);

    // 2) 拉取 pending 作业，按优先级（高→低）、提交时间（早→晚）排序。
    let jobs: Vec<Job> = crate::with_db!(
        pool,
        "SELECT * FROM jobs WHERE status = 'pending' AND deleted_at IS NULL \
         ORDER BY priority DESC, id ASC LIMIT ?",
        |db_s, db_e| {
            let limit: i64 = budget as i64;
            crate::db::query_as_db(db_e, db_s)
                .bind(limit)
                .fetch_all(db_e)
                .await?
        }
    );
    if jobs.is_empty() {
        return Ok(());
    }

    for job in jobs {
        if let Err(e) = try_dispatch_one(pool, &job, config).await {
            tracing::warn!(
                job_id = job.id,
                error = %e,
                "dispatch of job failed; will retry next tick"
            );
        }
    }
    Ok(())
}

/// 尝试派发单个作业：原子认领 → 配额预留 → GPU 分配 → 置 running → 挂接模拟完成任务。
///
/// 资源不足等可重试失败会回退为 `pending` 并返回 `Ok(())`（未派发）；真正的硬错误
/// （如 DB 故障）返回 `Err`。认领成功后、所有资源预留成功前，作业短暂处于 `running`
/// 半状态——若此窗口内进程崩溃，[`reconcile`] 会安全回退/重挂接。
async fn try_dispatch_one(pool: &DatabasePool, job: &Job, config: &DispatchConfig) -> AppResult<()> {
    // 原子认领 pending→running，仅当仍是 pending。
    let now = Utc::now();
    let claimed = crate::with_db!(
        pool,
        "UPDATE jobs SET status = 'running', start_time = ?, updated_at = ? \
         WHERE id = ? AND status = 'pending' AND deleted_at IS NULL",
        |db_s, db_e| {
            sqlx::query(db_s)
                .bind(now)
                .bind(now)
                .bind(job.id)
                .execute(db_e)
                .await?
                .rows_affected()
        }
    );
    if claimed == 0 {
        return Ok(()); // 已被其他 worker 认领或不再是 pending
    }

    // 观测入队到调度的等待延迟（秒）。
    let age = (Utc::now() - job.created_at).num_milliseconds().max(0) as f64 / 1000.0;
    crate::metrics::scheduler_metrics()
        .schedule_latency_seconds
        .observe(age);

    // 配额预留（若租户存在 active 配额；无配额视为无限）。
    let quota_req = QuotaRequest {
        gpu: job.gpus,
        cpu: job.cpus as f64,
        memory_gb: job.memory as f64,
        storage_gb: 0.0,
    };
    let partition = if job.partition_id > 0 {
        Some(job.partition_id)
    } else {
        None
    };
    let quota_id = match quota::find_quota_for_scope(pool, job.tenant_id, partition).await? {
        Some(q) => match quota::allocate_resources(pool, q.id, quota_req).await {
            Ok(_) => Some(q.id),
            Err(e) => {
                tracing::debug!(
                    job_id = job.id,
                    tenant_id = job.tenant_id,
                    error = %e,
                    "quota insufficient; revert to pending"
                );
                crate::metrics::scheduler_metrics()
                    .quota_rejected_total
                    .with_label_values(&[&job.tenant_id.to_string(), "combined"])
                    .inc();
                revert_to_pending(pool, job.id).await?;
                return Ok(());
            }
        },
        None => None,
    };

    // GPU 分配（仅当作业请求 GPU）。逐卡分配，失败则释放已分配部分并回退。
    if job.gpus > 0 {
        let mut alloc_ok = true;
        for _ in 0..job.gpus {
            match gpu::allocate_gpu(
                pool,
                gpu::AllocateGpuInput {
                    job_id: job.id,
                    tenant_id: job.tenant_id,
                    user_id: job.user_id,
                    fraction: if job.gpu_fraction > 0.0 {
                        job.gpu_fraction
                    } else {
                        1.0
                    },
                    memory_gb: job.gpu_memory_gb,
                    vendor: job.gpu_vendor.clone(),
                },
            )
            .await
            {
                Ok(_) => {}
                Err(e) => {
                    alloc_ok = false;
                    tracing::debug!(
                        job_id = job.id,
                        error = %e,
                        "gpu allocation failed; revert"
                    );
                    crate::metrics::scheduler_metrics()
                        .gpu_allocation_failed_total
                        .with_label_values(&["no_capacity"])
                        .inc();
                    break;
                }
            }
        }
        if !alloc_ok {
            // 释放已分配的 GPU + 配额，回退 pending。
            let _ = release_gpu_for_job(pool, job.id).await;
            if let Some(qid) = quota_id {
                let _ = quota::release_resources(pool, qid, quota_req).await;
            }
            revert_to_pending(pool, job.id).await?;
            return Ok(());
        }
    }

    // 全部预留成功 → 挂接模拟执行任务。
    let pool2 = pool.clone();
    let job_id = job.id;
    let sim_ms = config.simulated_run_ms;
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(sim_ms)).await;
        if let Err(e) = complete_running_job(&pool2, job_id).await {
            tracing::warn!(job_id, error = %e, "complete_running_job failed");
        }
    });

    Ok(())
}

/// 回退作业为 pending（清除 start_time），仅当当前仍为 running。
async fn revert_to_pending(pool: &DatabasePool, job_id: i64) -> AppResult<()> {
    crate::with_db!(
        pool,
        "UPDATE jobs SET status = 'pending', start_time = NULL, updated_at = ? \
         WHERE id = ? AND status = 'running' AND deleted_at IS NULL",
        |db_s, db_e| {
            sqlx::query(db_s)
                .bind(Utc::now())
                .bind(job_id)
                .execute(db_e)
                .await?
                .rows_affected()
        }
    );
    Ok(())
}

/// 释放某作业的全部 active GPU 分配。
async fn release_gpu_for_job(pool: &DatabasePool, job_id: i64) -> AppResult<()> {
    let alloc_ids: Vec<i64> = crate::with_db!(
        pool,
        "SELECT id FROM gpu_allocations WHERE job_id = ? AND status = 'active' AND deleted_at IS NULL",
        |db_s, db_e| crate::db::query_scalar_db(db_e, db_s)
            .bind(job_id)
            .fetch_all(db_e)
            .await?
    );
    for aid in alloc_ids {
        if let Err(e) = gpu::release_gpu(pool, aid).await {
            tracing::warn!(allocation_id = aid, error = %e, "failed to release gpu allocation");
        }
    }
    Ok(())
}

/// 完成 running 作业：释放 GPU + 配额 → 置 `completed`（progress=100）。
///
/// 同时被模拟执行任务（睡眠后）与崩溃恢复 [`reconcile`] 复用，确保释放逻辑单一来源。
async fn complete_running_job(pool: &DatabasePool, job_id: i64) -> AppResult<()> {
    // 取作业以确定租户 / 分区 / 资源量（用于释放配额）。
    let job = job::get_by_id(pool, job_id, false)
        .await?
        .ok_or_else(|| crate::error::AppError::internal("job disappeared during completion"))?;

    // 释放配额（若租户有 active 配额）。
    let partition = if job.partition_id > 0 {
        Some(job.partition_id)
    } else {
        None
    };
    if let Some(q) = quota::find_quota_for_scope(pool, job.tenant_id, partition).await? {
        let req = QuotaRequest {
            gpu: job.gpus,
            cpu: job.cpus as f64,
            memory_gb: job.memory as f64,
            storage_gb: 0.0,
        };
        let _ = quota::release_resources(pool, q.id, req).await;
    }

    // 释放 GPU。
    release_gpu_for_job(pool, job_id).await?;

    // 置 completed。
    let now = Utc::now();
    crate::with_db!(
        pool,
        "UPDATE jobs SET status = 'completed', progress = 100, end_time = ?, updated_at = ? \
         WHERE id = ? AND status = 'running' AND deleted_at IS NULL",
        |db_s, db_e| {
            sqlx::query(db_s)
                .bind(now)
                .bind(now)
                .bind(job_id)
                .execute(db_e)
                .await?
                .rows_affected()
        }
    );
    crate::metrics::scheduler_metrics().completed_total.inc();
    Ok(())
}

/// 崩溃恢复：扫描 running 作业。
///
/// - 仍持有 active GPU 分配 → 重新挂接完成逻辑（释放资源并置 completed），避免孤儿 running。
/// - 无 active GPU 分配 → 视为孤儿，超过宽限期才回退 pending 以便重新调度
///   （避免误伤刚派发、尚在模拟执行中的作业）。
async fn reconcile(pool: &DatabasePool, config: &DispatchConfig) -> AppResult<()> {
    let running_jobs: Vec<Job> = crate::with_db!(
        pool,
        "SELECT * FROM jobs WHERE status = 'running' AND deleted_at IS NULL",
        |db_s, db_e| crate::db::query_as_db(db_e, db_s).fetch_all(db_e).await?
    );
    let grace = ChronoDuration::seconds(config.reconcile_grace_seconds);
    let now = Utc::now();
    for job in running_jobs {
        let cnt: i64 = crate::with_db!(
            pool,
            "SELECT COUNT(*) FROM gpu_allocations \
             WHERE job_id = ? AND status = 'active' AND deleted_at IS NULL",
            |db_s, db_e| crate::db::query_scalar_db(db_e, db_s)
                .bind(job.id)
                .fetch_one(db_e)
                .await?
        );
        let has_alloc = cnt > 0;
        if has_alloc {
            // 重新挂接完成（视为已运行，不再睡眠）。
            let pool2 = pool.clone();
            let jid = job.id;
            tokio::spawn(async move {
                if let Err(e) = complete_running_job(&pool2, jid).await {
                    tracing::warn!(job_id = jid, error = %e, "reconcile complete_running_job failed");
                }
            });
        } else {
            // 孤儿：超过宽限期才回退。
            let stale = job
                .start_time
                .map(|st| now - st > grace)
                .unwrap_or(true);
            if stale {
                tracing::info!(
                    job_id = job.id,
                    "reconcile: orphaned running job without gpu allocation -> revert to pending"
                );
                revert_to_pending(pool, job.id).await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DatabasePool;

    async fn setup_pool() -> DatabasePool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect sqlite memory");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run migrations");
        DatabasePool::Sqlite(pool)
    }

    fn dispatch_config() -> DispatchConfig {
        DispatchConfig {
            enabled: true,
            interval_ms: 1000,
            simulated_run_ms: 50,
            batch_size: 10,
            max_concurrent: 10,
            reconcile_grace_seconds: 300,
        }
    }

    #[tokio::test]
    async fn test_dispatch_pending_job_completes_and_releases() {
        let pool = setup_pool().await;

        // 播种：GPU 设备（available，足够显存）、租户配额、pending 作业（请求 1 GPU）。
        gpu::create_gpu_device(
            &pool,
            gpu::CreateGpuDeviceInput {
                cluster_id: 1,
                node_name: "node-1".into(),
                vendor: "nvidia".into(),
                model: "A100".into(),
                index: 0,
                total_memory_gb: 80,
                allocatable_memory_gb: 80,
                used_memory_gb: 0,
                mig_enabled: false,
                mig_profiles: "[]".into(),
                driver_version: "".into(),
                cuda_version: "".into(),
                status: Some("available".into()),
                utilization: 0.0,
                temperature: 0,
                power_draw: 0,
                details: "".into(),
            },
        )
        .await
        .expect("create gpu device");

        quota::create_quota(
            &pool,
            quota::CreateQuotaInput {
                name: "tenant-q".into(),
                description: "".into(),
                tenant_id: 1,
                partition_id: None,
                gpu_limit: 4,
                cpu_limit: 100.0,
                memory_limit_gb: 100.0,
                storage_limit_gb: 100.0,
                status: Some("active".into()),
            },
        )
        .await
        .expect("create quota");

        let created = crate::services::job::create_job(
            &pool,
            crate::services::job::CreateJobInput {
                name: "train-1".into(),
                description: "".into(),
                kind: "training".into(),
                priority: 1,
                gpus: 1,
                cpus: 0,
                memory: 0,
                duration: 0,
                cluster_id: 1,
            },
            crate::services::job::Actor {
                tenant_id: 1,
                user_id: 1,
                is_admin: true,
            },
        )
        .await
        .expect("create job");

        // 单轮派发：应认领并置 running，随后模拟执行并 completed。
        dispatch_tick(&pool, &dispatch_config()).await.expect("dispatch tick");

        // 轮询直到作业完成（模拟执行 50ms）。
        let mut completed = false;
        for _ in 0..100 {
            let j = crate::services::job::get_job_unscoped(&pool, created.id).await.expect("get job");
            if j.status == job::status::COMPLETED {
                completed = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        assert!(completed, "job should reach completed after dispatch");

        // GPU 分配应已释放（无 active 分配）。
        let active: i64 = crate::with_db!(
            &pool,
            "SELECT COUNT(*) FROM gpu_allocations WHERE job_id = ? AND status = 'active' AND deleted_at IS NULL",
            |db_s, db_e| crate::db::query_scalar_db(db_e, db_s)
                .bind(created.id)
                .fetch_one(db_e)
                .await
        )
        .expect("count active allocations");
        assert_eq!(active, 0, "gpu allocation must be released after completion");

        // 配额应已释放（gpu_used 回到 0）。
        let q = quota::find_quota_for_scope(&pool, 1, None)
            .await
            .expect("find quota")
            .expect("some quota");
        assert_eq!(q.gpu_used, 0, "quota must be released after completion");
    }

    #[tokio::test]
    async fn test_dispatch_without_quota_or_gpu_still_runs_cpu_only() {
        let pool = setup_pool().await;
        // 不播种 GPU 设备、不播种配额（无限额）。
        let created = crate::services::job::create_job(
            &pool,
            crate::services::job::CreateJobInput {
                name: "cpu-job".into(),
                description: "".into(),
                kind: "batch".into(),
                priority: 0,
                gpus: 0,
                cpus: 0,
                memory: 0,
                duration: 0,
                cluster_id: 1,
            },
            crate::services::job::Actor {
                tenant_id: 2,
                user_id: 1,
                is_admin: true,
            },
        )
        .await
        .expect("create job");

        dispatch_tick(&pool, &dispatch_config()).await.expect("dispatch tick");

        let mut completed = false;
        for _ in 0..100 {
            let j = crate::services::job::get_job_unscoped(&pool, created.id).await.expect("get job");
            if j.status == job::status::COMPLETED {
                completed = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        assert!(
            completed,
            "cpu-only job with no quota/gpu should still be dispatched and completed"
        );
    }
}
