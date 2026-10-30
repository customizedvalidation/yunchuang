//! 真实调度闭环（P0-1）：周期性扫描 `jobs` 表中 `pending` 作业，原子认领 → 配额预留
//! → GPU 分配 → 置 `running` → 模拟执行 → 释放 GPU/配额 → 置 `completed`。
//!
//! 增量能力：P1-1 Backfill 回填 / P1-2 多因子评分 / P1-3 弹性配额借用 /
//! P2-1 Gang all-or-nothing 准入 / **P2-2 拓扑感知放置（消费 `jobs.topology_affinity`）**。
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

use std::cmp::Ordering;
use std::collections::HashMap;
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
    // ---- P1 利用率与公平 ----
    /// P1-1 Backfill 回填：利用空闲容量派发小作业。
    pub backfill_enabled: bool,
    /// Backfill 额外扫描候选数。
    pub backfill_window: usize,
    /// P1-2 启用以多因子动态评分替代静态优先级排序。
    pub scoring_enabled: bool,
    /// 静态优先级权重。
    pub priority_weight: f64,
    /// 等待时间奖励权重。
    pub age_weight: f64,
    /// 等待时间半衰期（小时）。
    pub age_half_life_hours: f64,
    /// 公平份额奖励权重。
    pub fairshare_weight: f64,
    /// 作业规模惩罚权重。
    pub size_weight: f64,
    /// P2-1 Gang 调度：启用后 Gang 作业按 min_available 副本做 all-or-nothing 原子准入；
    /// 关闭时 Gang 作业按 replicas=1（普通作业）处理。
    pub gang_enabled: bool,
    /// P2-2 拓扑感知：消费 jobs.topology_affinity，把作业全部 GPU 副本共置到同一节点
    /// （node/rack/switch 级别）或钉到指定节点（node:<name> / 裸节点名）。默认开启。
    pub topology_enabled: bool,
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
            backfill_enabled: cfg.dispatch_backfill_enabled,
            backfill_window: cfg.dispatch_backfill_window,
            scoring_enabled: cfg.dispatch_scoring_enabled,
            priority_weight: cfg.dispatch_priority_weight,
            age_weight: cfg.dispatch_age_weight,
            age_half_life_hours: cfg.dispatch_age_half_life_hours,
            fairshare_weight: cfg.dispatch_fairshare_weight,
            size_weight: cfg.dispatch_size_weight,
            gang_enabled: cfg.dispatch_gang_enabled,
            topology_enabled: cfg.dispatch_topology_enabled,
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
///
/// 两阶段：
/// 1. **优先级主路径**：按动态评分（P1-2，默认开启）或静态优先级排序，取前 `budget` 个尝试派发。
/// 2. **Backfill 回填**（P1-1，默认开启）：剩余候选按"规模升序"排序（小作业优先），
///    在不阻塞高优先级作业的前提下，利用空闲并发槽与资源余量派发能塞进去的小作业，
///    提升集群利用率。被计入 `backfilled_total` 指标。
///
/// 通过本地 `dispatched` 计数严格约束单轮实际 running 数不超过 `max_concurrent`。
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
    let remaining_capacity = config.max_concurrent - running;

    // 2) 拉取候选 pending 作业（主路径 budget + backfill 额外窗口）。
    let fetch_limit = (remaining_capacity
        + if config.backfill_enabled {
            config.backfill_window
        } else {
            0
        }) as i64;
    let candidates: Vec<Job> = crate::with_db!(
        pool,
        "SELECT * FROM jobs WHERE status = 'pending' AND deleted_at IS NULL \
         ORDER BY id ASC LIMIT ?",
        |db_s, db_e| {
            crate::db::query_as_db(db_e, db_s)
                .bind(fetch_limit)
                .fetch_all(db_e)
                .await?
        }
    );
    if candidates.is_empty() {
        return Ok(());
    }

    let now = Utc::now();
    // 租户用量比例（fairshare 输入），按候选涉及的租户一次性计算并缓存。
    let tenant_ratio = tenant_usage_ratios(pool, &candidates).await?;
    // 动态评分（P1-2）。
    let scores = dispatch_scores(&candidates, &tenant_ratio, config, now);

    // 3) 主路径：按评分（或静态优先级）降序排序。
    let mut ordered: Vec<&Job> = candidates.iter().collect();
    if config.scoring_enabled {
        ordered.sort_by(|a, b| {
            scores
                .get(&b.id)
                .copied()
                .unwrap_or(0.0)
                .partial_cmp(&scores.get(&a.id).copied().unwrap_or(0.0))
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
    } else {
        ordered.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));
    }
    let budget = remaining_capacity.min(config.batch_size).max(1);

    let mut dispatched = 0usize;
    for job in ordered.iter().take(budget) {
        if dispatched >= remaining_capacity {
            break;
        }
        if try_dispatch_one(pool, job, config).await? {
            dispatched += 1;
        }
    }

    // 4) Backfill：剩余候选按规模升序（小作业优先）尝试派发，利用空闲槽。
    if config.backfill_enabled {
        let mut backfill: Vec<&Job> = ordered.into_iter().skip(budget).collect();
        backfill.sort_by(|a, b| {
            job_size(a, gang_unit(a, config))
                .partial_cmp(&job_size(b, gang_unit(b, config)))
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.created_at.cmp(&b.created_at))
                .then_with(|| a.id.cmp(&b.id))
        });
        for job in backfill {
            if dispatched >= remaining_capacity {
                break;
            }
            if try_dispatch_one(pool, job, config).await? {
                dispatched += 1;
                crate::metrics::scheduler_metrics().backfilled_total.inc();
            }
        }
    }

    Ok(())
}

/// P2-1 Gang 调度的准入副本数（unit）。
///
/// - Gang 关闭，或作业非 Gang（`gang_replicas <= 1`）→ 1（按普通单副本作业处理）。
/// - Gang 开启且 `gang_replicas > 1` → `gang_min_available.max(1)`：
///   调度按该副本数的资源量做 all-or-nothing 原子预留（对标 Volcano PodGroup
///   minMember / Borg gang 语义）。
fn gang_unit(job: &Job, config: &DispatchConfig) -> i64 {
    if config.gang_enabled && job.gang_replicas > 1 {
        job.gang_min_available.max(1)
    } else {
        1
    }
}

/// 作业规模归一化（用于 Backfill 优先派发小作业 + 评分惩罚）。
///
/// Gang 作业按准入副本数 `unit` 放大，保证大资源 Gang 作业正确参与排序/惩罚。
fn job_size(job: &Job, unit: i64) -> f64 {
    let u = unit.max(1) as f64;
    u * (job.gpus as f64 + job.cpus as f64 / 8.0 + job.memory as f64 / 64.0)
}

/// 计算各租户的配额用量比例（used / limit，clamp 到 [0,1]）；无配额（无限额）视为 0。
async fn tenant_usage_ratios(
    pool: &DatabasePool,
    candidates: &[Job],
) -> AppResult<HashMap<i64, f64>> {
    let mut ratios = HashMap::new();
    for job in candidates {
        if ratios.contains_key(&job.tenant_id) {
            continue;
        }
        let partition = if job.partition_id > 0 {
            Some(job.partition_id)
        } else {
            None
        };
        let ratio = match quota::find_quota_for_scope(pool, job.tenant_id, partition).await? {
            Some(q) => {
                let r = |used: f64, limit: f64| {
                    if limit <= 0.0 {
                        0.0
                    } else {
                        (used / limit).clamp(0.0, 1.0)
                    }
                };
                // 取四类资源中用量最高者作为该租户整体"拥挤度"。
                let gpu = r(q.gpu_used as f64, q.gpu_limit as f64);
                let cpu = r(q.cpu_used, q.cpu_limit);
                let mem = r(q.memory_used_gb, q.memory_limit_gb);
                let sto = r(q.storage_used_gb, q.storage_limit_gb);
                gpu.max(cpu).max(mem).max(sto)
            }
            None => 0.0,
        };
        ratios.insert(job.tenant_id, ratio);
    }
    Ok(ratios)
}

/// P1-2 多因子动态调度评分。
///
/// score = 优先级权重·priority
///       + 等待时间奖励（半衰期收敛：等待越久边际加成越小，防止无限积压 / 饿死）
///       + 公平份额奖励（租户越空闲得分越高）
///       - 规模惩罚（大作业降分，利于小作业回填装箱）。
fn dispatch_scores(
    candidates: &[Job],
    tenant_ratio: &HashMap<i64, f64>,
    config: &DispatchConfig,
    now: chrono::DateTime<Utc>,
) -> HashMap<i64, f64> {
    let mut scores = HashMap::new();
    for job in candidates {
        let priority_term = config.priority_weight * job.priority as f64;

        let age_hours = (now - job.created_at).num_seconds().max(0) as f64 / 3600.0;
        // 半衰期：age_bonus 随等待时间逼近 age_weight，边际增量每 half_life 减半。
        let age_bonus = if config.age_half_life_hours > 0.0 {
            config.age_weight
                * (1.0 - 0.5f64.powf(age_hours / config.age_half_life_hours))
        } else {
            config.age_weight * age_hours
        };

        let ratio = tenant_ratio.get(&job.tenant_id).copied().unwrap_or(0.0);
        let fairshare_bonus = config.fairshare_weight * (1.0 - ratio);

        let size_penalty = config.size_weight * job_size(job, gang_unit(job, config));

        let score = priority_term + age_bonus + fairshare_bonus - size_penalty;
        scores.insert(job.id, score);
    }
    scores
}

/// 尝试派发单个作业：原子认领 → 配额预留（含弹性借用）→ GPU 分配 → 置 running →
/// 挂接模拟完成任务。
///
/// 返回 `true` 表示已成功认领并预留资源、作业进入 running；返回 `false` 表示本轮未派发
/// （已被他人认领、或资源不足已回退 pending，下一轮重试）。真正的硬错误（如 DB 故障）
/// 返回 `Err`。认领成功后、所有资源预留成功前，作业短暂处于 `running` 半状态——若此窗口
/// 内进程崩溃，[`reconcile`] 会安全回退/重挂接。
async fn try_dispatch_one(pool: &DatabasePool, job: &Job, config: &DispatchConfig) -> AppResult<bool> {
    // P2-1 Gang：准入副本数（非 Gang 或开关关闭时为 1，行为与既有路径完全一致）。
    let unit = gang_unit(job, config);
    let is_gang = unit > 1;

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
        return Ok(false); // 已被其他 worker 认领或不再是 pending
    }

    // 观测入队到调度的等待延迟（秒）。
    let age = (Utc::now() - job.created_at).num_milliseconds().max(0) as f64 / 1000.0;
    crate::metrics::scheduler_metrics()
        .schedule_latency_seconds
        .observe(age);

    // 配额预留（若租户存在 active 配额；无配额视为无限）。
    // Gang 作业：请求量 = min_available 副本 × 各资源（all-or-nothing 原子预留）。
    let quota_req = QuotaRequest {
        gpu: job.gpus * unit,
        cpu: job.cpus as f64 * unit as f64,
        memory_gb: job.memory as f64 * unit as f64,
        storage_gb: 0.0,
    };
    let partition = if job.partition_id > 0 {
        Some(job.partition_id)
    } else {
        None
    };
    let quota_id = match quota::find_quota_for_scope(pool, job.tenant_id, partition).await? {
        Some(q) => match quota::allocate_resources_with_borrow(pool, q.id, quota_req).await {
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
                if is_gang {
                    // Gang 准入失败（任一副本资源不足即整体失败）。
                    crate::metrics::scheduler_metrics().gang_blocked_total.inc();
                }
                revert_to_pending(pool, job.id).await?;
                return Ok(false);
            }
        },
        None => None,
    };

    // GPU 分配（P2-2 拓扑感知：topology_affinity 约束下共置 / 钉节点放置）。
    // Gang 作业：共分配 min_available 副本 × gpus 张卡，任一张不足即整体回退。
    if job.gpus > 0 && !allocate_gpus_for_job(pool, job, config, unit).await? {
        // 释放已分配的 GPU + 本轮配额，回退 pending。
        let _ = release_gpu_for_job(pool, job.id).await;
        if let Some(qid) = quota_id {
            let _ = quota::release_resources(pool, qid, quota_req).await;
        }
        if is_gang {
            // Gang 准入失败（任一副本 GPU 不足即整体失败）。
            crate::metrics::scheduler_metrics().gang_blocked_total.inc();
        }
        revert_to_pending(pool, job.id).await?;
        return Ok(false);
    }

    // P2-1 Gang：min_available 副本资源全部原子预留成功 → 落实 gang_fulfilled 并计指标。
    if is_gang {
        crate::with_db!(
            pool,
            "UPDATE jobs SET gang_fulfilled = ?, updated_at = ? \
             WHERE id = ? AND deleted_at IS NULL",
            |db_s, db_e| {
                sqlx::query(db_s)
                    .bind(unit)
                    .bind(Utc::now())
                    .bind(job.id)
                    .execute(db_e)
                    .await?
                    .rows_affected()
            }
        );
        crate::metrics::scheduler_metrics().gang_admitted_total.inc();
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

    Ok(true)
}

// ---------------------------------------------------------------------------
// P2-2 拓扑感知放置
// ---------------------------------------------------------------------------

/// P2-2 由 `jobs.topology_affinity` 解析出的放置模式。
#[derive(Debug, Clone, PartialEq)]
enum TopoPlacement {
    /// 无约束（空串 / cluster / 关闭开关）——既有行为。
    Unconstrained,
    /// 共置：全部 GPU 副本（× Gang unit）放在同一节点。
    /// `node` 为显式语义；`rack` / `switch` 因设备模型无 rack/switch 层级字段，
    /// 退化为 node 级共置（严格满足约束、放置更紧；不满足即回退 pending 背压，
    /// 绝不违反亲和）。
    CoLocate,
    /// 钉住指定节点（`node:<name>` 前缀或非关键字裸值）。
    Pinned(String),
}

/// 解析作业的拓扑放置模式。
///
/// 格式判别规则：先 trim；关键字比较大小写不敏感（`Node`/`CLUSTER` 等均识别），
/// 但钉扎目标**保留原始大小写**（节点名大小写敏感）：
/// - `""` / `cluster` → 无约束；
/// - `node` / `rack` / `switch` → 同节点共置；
/// - `node:<name>` → 剥前缀钉住 `<name>`（前缀形式大小写不敏感：`Node:x` 同样识别，
///   对 lower 剥前缀后按长度映射回 raw 切片——`to_ascii_lowercase` 仅改 ASCII
///   大小写、字节长不变，映射安全；目标 `<name>` 仍保留原始大小写）；
/// - 其余非关键字裸值 → 视为节点名钉住。
fn topology_placement(job: &Job, config: &DispatchConfig) -> TopoPlacement {
    if !config.topology_enabled {
        return TopoPlacement::Unconstrained;
    }
    let raw = job.topology_affinity.trim();
    let lower = raw.to_ascii_lowercase();
    match lower.as_str() {
        "" | "cluster" => TopoPlacement::Unconstrained,
        "node" | "rack" | "switch" => TopoPlacement::CoLocate,
        _ => {
            // `node:<name>` 显式钉扎形式（前缀大小写不敏感）；其余裸值视为节点名。
            let name = if let Some(lower_name) = lower.strip_prefix("node:") {
                let raw_name = &raw[raw.len() - lower_name.len()..];
                raw_name.trim()
            } else {
                raw
            };
            // 形如 "node:"（空名）回退为整串裸值，不产生空钉扎。
            let name = if name.is_empty() { raw } else { name };
            TopoPlacement::Pinned(name.to_string())
        }
    }
}

/// 按节点分组预选放置节点（P2-2 共置 / 钉扎的前置检查）。
///
/// 统计每个节点上满足作业单卡需求（vendor / 显存 / fraction 容量）的"可用卡数"，
/// 返回可用卡数 ≥ `total_cards` 的节点中**空闲卡数最小**者（best-fit，减少碎片；
/// 并列取节点名稳定序）。无满足节点返回 `None`——这是正确背压（作业回退 pending
/// 下一轮重试），而非"首卡落在卡数不足节点 → 确定性反复回退"的 livelock。
///
/// 钉扎模式只统计指定节点；节点不存在 / 卡数不足 → `None`。
/// 注意：按"每卡一设备"保守计数（fraction < 1 的同卡多份额不计入），宁可少算不超卖。
async fn select_topology_node(
    pool: &DatabasePool,
    job: &Job,
    placement: &TopoPlacement,
    total_cards: i64,
) -> AppResult<Option<String>> {
    let node_filter: Option<String> = match placement {
        TopoPlacement::Pinned(n) => Some(n.clone()),
        _ => None,
    };
    let vendor = job.gpu_vendor.trim();
    let fraction = if job.gpu_fraction > 0.0 {
        job.gpu_fraction
    } else {
        1.0
    };
    let memory_gb = job.gpu_memory_gb;

    // 1) 候选设备（与 gpu::allocate_gpu 的候选口径一致：available/allocated）。
    let devices: Vec<crate::models::gpu_device::GpuDevice> = match &node_filter {
        Some(node) => {
            crate::with_db!(
                pool,
                "SELECT * FROM gpu_devices \
                 WHERE deleted_at IS NULL AND (status = 'available' OR status = 'allocated') \
                   AND node_name = ?",
                |db_s, db_e| crate::db::query_as_db(db_e, db_s)
                    .bind(node)
                    .fetch_all(db_e)
                    .await?
            )
        }
        None => {
            crate::with_db!(
                pool,
                "SELECT * FROM gpu_devices \
                 WHERE deleted_at IS NULL AND (status = 'available' OR status = 'allocated')",
                |db_s, db_e| crate::db::query_as_db(db_e, db_s).fetch_all(db_e).await?
            )
        }
    };

    // 2) 各设备 active fraction 占用（一次聚合，消除 N 次 SUM）。
    let frac_rows: Vec<(i64, f64)> = crate::with_db!(
        pool,
        "SELECT device_id, COALESCE(SUM(fraction), 0.0) FROM gpu_allocations \
         WHERE status = 'active' AND deleted_at IS NULL GROUP BY device_id",
        |db_s, db_e| crate::db::query_as_db(db_e, db_s).fetch_all(db_e).await?
    );
    let frac_map: std::collections::HashMap<i64, f64> = frac_rows.into_iter().collect();

    // 3) 按节点统计满足单卡需求的可用卡数。
    let mut per_node: HashMap<String, i64> = HashMap::new();
    for d in &devices {
        if !vendor.is_empty() && d.vendor != vendor {
            continue;
        }
        if d.allocatable_memory_gb - d.used_memory_gb < memory_gb {
            continue;
        }
        let used = frac_map.get(&d.id).copied().unwrap_or(0.0);
        if (1.0 - used) + 0.001 < fraction {
            continue;
        }
        *per_node.entry(d.node_name.clone()).or_insert(0) += 1;
    }

    // 4) best-fit：卡数足够的节点里选空闲卡数最小者（并列取名字序，保证确定性）。
    let mut candidates: Vec<(String, i64)> = per_node
        .into_iter()
        .filter(|(_, cards)| *cards >= total_cards)
        .collect();
    candidates.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    Ok(candidates.into_iter().next().map(|(node, _)| node))
}

/// GPU 逐卡分配（P2-2 拓扑感知：topology_affinity 约束下共置 / 钉节点放置）。
///
/// - **预选节点**（约束生效时）：分配任何卡之前，先按节点分组筛出"空闲卡数 ≥
///   total_cards"的节点并 best-fit 选一个，再把全部卡钉在该节点逐卡分配——
///   避免首卡落在卡数不足节点导致的确定性反复回退（livelock）。
/// - `Pinned(node)`：只在该节点预选 / 分配（节点不存在或卡数不足 → blocked）。
/// - `CoLocate` 且总卡数 > 1：全集群预选；单卡作业无额外约束（单卡天然满足）。
/// - 约束关闭时行为与既有路径完全一致（不预选、不钉节点）。
///
/// 返回 `true` 成功（约束真实生效时计 `topology_matched_total`）；`false` 失败——
/// 内部已释放全部部分分配并计 `topology_blocked_total`，由调用方回退 pending。
async fn allocate_gpus_for_job(
    pool: &DatabasePool,
    job: &Job,
    config: &DispatchConfig,
    unit: i64,
) -> AppResult<bool> {
    let total_cards = job.gpus * unit;
    let placement = topology_placement(job, config);
    // 拓扑约束是否真实生效（用于 matched / blocked 指标与预选）。
    let constraint_active = match &placement {
        TopoPlacement::Pinned(_) => true,
        TopoPlacement::CoLocate => total_cards > 1,
        TopoPlacement::Unconstrained => false,
    };

    // 预选节点：无满足节点即正确背压（blocked 指标 + 回退），不产生任何部分分配。
    let pinned_node: Option<String> = if constraint_active {
        match select_topology_node(pool, job, &placement, total_cards).await? {
            Some(node) => Some(node),
            None => {
                tracing::debug!(
                    job_id = job.id,
                    affinity = %job.topology_affinity,
                    total_cards,
                    "no node with enough free cards for topology constraint"
                );
                crate::metrics::scheduler_metrics().topology_blocked_total.inc();
                return Ok(false);
            }
        }
    } else {
        None
    };

    let node_name = pinned_node.unwrap_or_default();
    for _ in 0..total_cards {
        let res = gpu::allocate_gpu(
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
                node_name: node_name.clone(),
            },
        )
        .await;
        match res {
            Ok(_) => {}
            Err(e) => {
                tracing::debug!(
                    job_id = job.id,
                    affinity = %job.topology_affinity,
                    error = %e,
                    "gpu allocation failed under topology constraint; revert"
                );
                crate::metrics::scheduler_metrics()
                    .gpu_allocation_failed_total
                    .with_label_values(&["no_capacity"])
                    .inc();
                if constraint_active {
                    crate::metrics::scheduler_metrics().topology_blocked_total.inc();
                }
                // 释放已分配部分，交由调用方回退 pending。
                let _ = release_gpu_for_job(pool, job.id).await;
                return Ok(false);
            }
        }
    }

    if constraint_active {
        crate::metrics::scheduler_metrics().topology_matched_total.inc();
    }
    Ok(true)
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
        // P2-1 Gang：释放量必须与预留量一致——Gang 准入时按 min_available 副本
        // 预留（落库在 gang_fulfilled），非 Gang 作业 gang_fulfilled=0 → 按 1 副本释放。
        let unit = job.gang_fulfilled.max(1);
        let req = QuotaRequest {
            gpu: job.gpus * unit,
            cpu: job.cpus as f64 * unit as f64,
            memory_gb: job.memory as f64 * unit as f64,
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
        // 内存库必须单连接：多连接下每个连接各自一份私有 :memory: 库，
        // 迁移与后续查询可能落在不同连接上（偶发 no such table）。
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
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
            backfill_enabled: true,
            backfill_window: 50,
            scoring_enabled: true,
            priority_weight: 10.0,
            age_weight: 1.0,
            age_half_life_hours: 24.0,
            fairshare_weight: 5.0,
            size_weight: 0.5,
            gang_enabled: true,
            topology_enabled: true,
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
                ..Default::default()
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
                gang_replicas: 1,
                gang_min_available: 1,
                topology_affinity: String::new(),
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
                gang_replicas: 1,
                gang_min_available: 1,
                topology_affinity: String::new(),
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

    #[tokio::test]
    async fn test_backfill_dispatches_small_job_when_big_job_blocked() {
        let pool = setup_pool().await;
        // 大作业：高优先级（合法范围 [0,3]，越界会被 create_job 归 0）、请求 GPU，
        // 但集群无任何 GPU 设备 → 必然无法调度（回退 pending）。
        let big = crate::services::job::create_job(
            &pool,
            crate::services::job::CreateJobInput {
                name: "big".into(),
                description: "".into(),
                kind: "training".into(),
                priority: 3,
                gpus: 1,
                cpus: 0,
                memory: 0,
                duration: 0,
                cluster_id: 1,
                gang_replicas: 1,
                gang_min_available: 1,
                topology_affinity: String::new(),
            },
            crate::services::job::Actor {
                tenant_id: 1,
                user_id: 1,
                is_admin: true,
            },
        )
        .await
        .expect("create big job");

        // 小作业：低优先级、纯 CPU（无 GPU、无配额）→ 应被 backfill 派发完成。
        let small = crate::services::job::create_job(
            &pool,
            crate::services::job::CreateJobInput {
                name: "small".into(),
                description: "".into(),
                kind: "batch".into(),
                priority: 0,
                gpus: 0,
                cpus: 0,
                memory: 0,
                duration: 0,
                cluster_id: 1,
                gang_replicas: 1,
                gang_min_available: 1,
                topology_affinity: String::new(),
            },
            crate::services::job::Actor {
                tenant_id: 1,
                user_id: 1,
                is_admin: true,
            },
        )
        .await
        .expect("create small job");

        // 主路径 batch_size=1 只取评分最高的 big；small 走 backfill。
        let mut cfg = dispatch_config();
        cfg.batch_size = 1;
        cfg.backfill_enabled = true;
        cfg.scoring_enabled = true;
        cfg.max_concurrent = 10;
        dispatch_tick(&pool, &cfg).await.expect("dispatch tick");

        let mut small_done = false;
        for _ in 0..100 {
            let j = crate::services::job::get_job_unscoped(&pool, small.id)
                .await
                .expect("get small");
            if j.status == job::status::COMPLETED {
                small_done = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        assert!(
            small_done,
            "small job should be backfilled and completed"
        );

        // 大作业因无 GPU 资源而保持 pending（回退），未被错误完成。
        let big_j = crate::services::job::get_job_unscoped(&pool, big.id)
            .await
            .expect("get big");
        assert_eq!(
            big_j.status,
            job::status::PENDING,
            "big job must remain pending (no GPU capacity)"
        );

        // backfill 指标应至少 +1。
        let bf = crate::metrics::scheduler_metrics().backfilled_total.get();
        assert!(bf >= 1, "backfilled_total should be incremented, got {bf}");
    }

    /// 播种一张 available 的 GPU 设备（简化测试样板）。
    async fn seed_gpu_device(pool: &DatabasePool, node: &str, index: i64) {
        gpu::create_gpu_device(
            pool,
            gpu::CreateGpuDeviceInput {
                cluster_id: 1,
                node_name: node.into(),
                vendor: "nvidia".into(),
                model: "A100".into(),
                index,
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
    }

    #[tokio::test]
    async fn test_gang_job_blocked_when_resources_insufficient() {
        let pool = setup_pool().await;

        // 仅播种 1 张 GPU 设备 + 租户配额。
        seed_gpu_device(&pool, "node-1", 0).await;
        quota::create_quota(
            &pool,
            quota::CreateQuotaInput {
                name: "gang-q".into(),
                description: "".into(),
                tenant_id: 1,
                partition_id: None,
                gpu_limit: 4,
                cpu_limit: 100.0,
                memory_limit_gb: 100.0,
                storage_limit_gb: 100.0,
                status: Some("active".into()),
                ..Default::default()
            },
        )
        .await
        .expect("create quota");

        // Gang 作业：2 副本 × 1 GPU = 需 2 张卡 min_available，但集群只有 1 张 → 整体阻塞。
        let created = crate::services::job::create_job(
            &pool,
            crate::services::job::CreateJobInput {
                name: "gang-blocked".into(),
                description: "".into(),
                kind: "training".into(),
                priority: 2,
                gpus: 1,
                cpus: 0,
                memory: 0,
                duration: 0,
                cluster_id: 1,
                gang_replicas: 2,
                gang_min_available: 2,
                topology_affinity: String::new(),
            },
            crate::services::job::Actor {
                tenant_id: 1,
                user_id: 1,
                is_admin: true,
            },
        )
        .await
        .expect("create gang job");

        dispatch_tick(&pool, &dispatch_config()).await.expect("dispatch tick");

        // 作业必须仍为 pending（all-or-nothing：任一副本资源不足即整体回退）。
        let j = crate::services::job::get_job_unscoped(&pool, created.id)
            .await
            .expect("get gang job");
        assert_eq!(
            j.status,
            job::status::PENDING,
            "gang job must remain pending when min_available replicas cannot be reserved"
        );
        assert_eq!(j.gang_fulfilled, 0, "blocked gang job must not record gang_fulfilled");

        // 回退后不得残留 active GPU 分配（整体回退清理）。
        let active: i64 = crate::with_db!(
            &pool,
            "SELECT COUNT(*) FROM gpu_allocations WHERE job_id = ? AND status = 'active' AND deleted_at IS NULL",
            |db_s, db_e| crate::db::query_scalar_db(db_e, db_s)
                .bind(created.id)
                .fetch_one(db_e)
                .await
        )
        .expect("count active allocations");
        assert_eq!(active, 0, "blocked gang job must release partially allocated gpus");

        // gang_blocked_total 应至少 +1。
        let blocked = crate::metrics::scheduler_metrics().gang_blocked_total.get();
        assert!(blocked >= 1, "gang_blocked_total should be incremented, got {blocked}");
    }

    #[tokio::test]
    async fn test_gang_job_admitted_when_min_available_satisfied() {
        let pool = setup_pool().await;

        // 播种 2 张 GPU 设备 + 足量配额 → 2 副本 × 1 GPU = 2 张卡可满足。
        seed_gpu_device(&pool, "node-1", 0).await;
        seed_gpu_device(&pool, "node-1", 1).await;
        quota::create_quota(
            &pool,
            quota::CreateQuotaInput {
                name: "gang-q".into(),
                description: "".into(),
                tenant_id: 1,
                partition_id: None,
                gpu_limit: 8,
                cpu_limit: 100.0,
                memory_limit_gb: 100.0,
                storage_limit_gb: 100.0,
                status: Some("active".into()),
                ..Default::default()
            },
        )
        .await
        .expect("create quota");

        // Gang 作业：gang_replicas=2、gang_min_available=2、每副本 1 GPU + 2 CPU + 8G 内存。
        let created = crate::services::job::create_job(
            &pool,
            crate::services::job::CreateJobInput {
                name: "gang-admitted".into(),
                description: "".into(),
                kind: "training".into(),
                priority: 2,
                gpus: 1,
                cpus: 2,
                memory: 8,
                duration: 0,
                cluster_id: 1,
                gang_replicas: 2,
                gang_min_available: 2,
                topology_affinity: String::new(),
            },
            crate::services::job::Actor {
                tenant_id: 1,
                user_id: 1,
                is_admin: true,
            },
        )
        .await
        .expect("create gang job");

        dispatch_tick(&pool, &dispatch_config()).await.expect("dispatch tick");

        // 派发后（模拟执行睡眠窗口内）作业应 running，且资源按 2 副本原子预留：
        // gang_fulfilled=2、配额 gpu_used=2/cpu_used=4/mem=16、active GPU 分配=2 张。
        let j = crate::services::job::get_job_unscoped(&pool, created.id)
            .await
            .expect("get gang job");
        assert_eq!(j.status, job::status::RUNNING, "gang job should be running after admission");
        assert_eq!(j.gang_fulfilled, 2, "gang_fulfilled must equal min_available on admission");

        let q = quota::find_quota_for_scope(&pool, 1, None)
            .await
            .expect("find quota")
            .expect("some quota");
        assert_eq!(q.gpu_used, 2, "quota gpu_used must account for 2 replicas");
        assert!((q.cpu_used - 4.0).abs() < 1e-9, "quota cpu_used must account for 2 replicas");
        assert!(
            (q.memory_used_gb - 16.0).abs() < 1e-9,
            "quota memory_used_gb must account for 2 replicas"
        );

        let active: i64 = crate::with_db!(
            &pool,
            "SELECT COUNT(*) FROM gpu_allocations WHERE job_id = ? AND status = 'active' AND deleted_at IS NULL",
            |db_s, db_e| crate::db::query_scalar_db(db_e, db_s)
                .bind(created.id)
                .fetch_one(db_e)
                .await
        )
        .expect("count active allocations");
        assert_eq!(active, 2, "must allocate min_available × gpus = 2 cards");

        // gang_admitted_total 应至少 +1。
        let admitted = crate::metrics::scheduler_metrics().gang_admitted_total.get();
        assert!(admitted >= 1, "gang_admitted_total should be incremented, got {admitted}");

        // 随后模拟执行完成：资源应按 2 副本等量释放（配额归零、无残留分配）。
        let mut completed = false;
        for _ in 0..100 {
            let j = crate::services::job::get_job_unscoped(&pool, created.id)
                .await
                .expect("get job");
            if j.status == job::status::COMPLETED {
                completed = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        assert!(completed, "gang job should reach completed after dispatch");

        let q = quota::find_quota_for_scope(&pool, 1, None)
            .await
            .expect("find quota")
            .expect("some quota");
        assert_eq!(q.gpu_used, 0, "quota must be fully released after completion (2 replicas)");
        let active: i64 = crate::with_db!(
            &pool,
            "SELECT COUNT(*) FROM gpu_allocations WHERE job_id = ? AND status = 'active' AND deleted_at IS NULL",
            |db_s, db_e| crate::db::query_scalar_db(db_e, db_s)
                .bind(created.id)
                .fetch_one(db_e)
                .await
        )
        .expect("count active allocations");
        assert_eq!(active, 0, "all gang gpu allocations must be released after completion");
    }

    /// 播种租户级 active 配额（简化测试样板）。
    async fn seed_quota(pool: &DatabasePool, name: &str, gpu_limit: i64) {
        quota::create_quota(
            pool,
            quota::CreateQuotaInput {
                name: name.into(),
                description: "".into(),
                tenant_id: 1,
                partition_id: None,
                gpu_limit,
                cpu_limit: 100.0,
                memory_limit_gb: 100.0,
                storage_limit_gb: 100.0,
                status: Some("active".into()),
                ..Default::default()
            },
        )
        .await
        .expect("create quota");
    }

    /// 直接改写作业的 topology_affinity（绕过 API，便于测试钉节点 / 共置语义）。
    async fn set_topology_affinity(pool: &DatabasePool, job_id: i64, affinity: &str) {
        let sqlite = pool.as_sqlite().expect("test pool must be sqlite");
        sqlx::query("UPDATE jobs SET topology_affinity = ? WHERE id = ?")
            .bind(affinity)
            .bind(job_id)
            .execute(sqlite)
            .await
            .expect("set topology affinity");
    }

    /// 轮询等待作业进入期望状态。
    async fn wait_for_status(pool: &DatabasePool, job_id: i64, status: &str) -> bool {
        for _ in 0..100 {
            let j = crate::services::job::get_job_unscoped(pool, job_id)
                .await
                .expect("get job");
            if j.status == status {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(30)).await;
        }
        false
    }

    /// 查询作业全部 active GPU 分配所在节点名（单列标量）。
    async fn active_allocation_nodes(pool: &DatabasePool, job_id: i64) -> Vec<String> {
        crate::with_db!(
            pool,
            "SELECT d.node_name FROM gpu_allocations a \
             JOIN gpu_devices d ON d.id = a.device_id \
             WHERE a.job_id = ? AND a.status = 'active' AND a.deleted_at IS NULL",
            |db_s, db_e| crate::db::query_scalar_db(db_e, db_s)
                .bind(job_id)
                .fetch_all(db_e)
                .await
        )
        .expect("query allocation nodes")
    }

    /// 场景①：双节点各 1 卡 + 级别 `node` 作业请求 2 卡 → 同节点卡数不足必须保持
    /// pending（blocked 指标、无残留分配）；补足第 2 卡后成功且两卡共置 node-1。
    #[tokio::test]
    async fn test_topology_colocate_requires_enough_cards_on_single_node() {
        let pool = setup_pool().await;

        // 双节点各 1 卡 + 配额。
        seed_gpu_device(&pool, "node-1", 0).await;
        seed_gpu_device(&pool, "node-2", 0).await;
        seed_quota(&pool, "topo-q", 8).await;

        // gpus=2 + topology_affinity="node"：两卡必须共置同一节点。
        let created = crate::services::job::create_job(
            &pool,
            crate::services::job::CreateJobInput {
                name: "topo-job".into(),
                description: "".into(),
                kind: "training".into(),
                priority: 0,
                gpus: 2,
                cpus: 0,
                memory: 0,
                duration: 0,
                cluster_id: 1,
                gang_replicas: 1,
                gang_min_available: 1,
                topology_affinity: "node".into(),
            },
            crate::services::job::Actor {
                tenant_id: 1,
                user_id: 1,
                is_admin: true,
            },
        )
        .await
        .expect("create job");

        dispatch_tick(&pool, &dispatch_config()).await.expect("dispatch tick");

        // 每节点仅 1 卡：共置不可满足 → 整体回退 pending，无残留分配，配额不残留。
        let j = crate::services::job::get_job_unscoped(&pool, created.id)
            .await
            .expect("get job");
        assert_eq!(
            j.status,
            job::status::PENDING,
            "colocate job must stay pending when no single node has enough cards"
        );
        assert!(
            active_allocation_nodes(&pool, created.id).await.is_empty(),
            "blocked colocate job must not leave active allocations"
        );
        let q = quota::find_quota_for_scope(&pool, 1, None)
            .await
            .expect("find quota")
            .expect("some quota");
        assert_eq!(q.gpu_used, 0, "blocked colocate job must release reserved quota");
        let blocked = crate::metrics::scheduler_metrics().topology_blocked_total.get();
        assert!(blocked >= 1, "topology_blocked_total should be incremented, got {blocked}");

        // 给 node-1 补第 2 卡 → 共置可满足，两卡都落在 node-1。
        seed_gpu_device(&pool, "node-1", 1).await;
        dispatch_tick(&pool, &dispatch_config()).await.expect("dispatch tick");

        let nodes = active_allocation_nodes(&pool, created.id).await;
        assert_eq!(nodes.len(), 2, "must allocate 2 cards");
        assert!(
            nodes.iter().all(|n| n == "node-1"),
            "all replicas must be co-located on node-1, got {nodes:?}"
        );

        // 约束满足 → matched 指标至少 +1；随后模拟执行完成、资源正常释放。
        let matched = crate::metrics::scheduler_metrics().topology_matched_total.get();
        assert!(matched >= 1, "topology_matched_total should be incremented, got {matched}");
        assert!(
            wait_for_status(&pool, created.id, job::status::COMPLETED).await,
            "topology job should complete normally"
        );
        assert!(
            active_allocation_nodes(&pool, created.id).await.is_empty(),
            "allocations must be released after completion"
        );
    }

    /// livelock 回归：node-1（低 id）仅 1 卡、node-2 有 2 卡，级别 node 的作业请求
    /// 2 卡。旧"首卡自由 best-fit → 钉其节点"实现会确定性落在 node-1 并反复回退；
    /// 预选节点实现必须直接成功落在 node-2。
    #[tokio::test]
    async fn test_topology_colocate_preselects_node_with_enough_cards() {
        let pool = setup_pool().await;

        // node-1 先播种（低 id），仅 1 卡；node-2 有 2 卡。
        seed_gpu_device(&pool, "node-1", 0).await;
        seed_gpu_device(&pool, "node-2", 0).await;
        seed_gpu_device(&pool, "node-2", 1).await;
        seed_quota(&pool, "livelock-q", 8).await;

        let created = crate::services::job::create_job(
            &pool,
            crate::services::job::CreateJobInput {
                name: "livelock".into(),
                description: "".into(),
                kind: "training".into(),
                priority: 0,
                gpus: 2,
                cpus: 0,
                memory: 0,
                duration: 0,
                cluster_id: 1,
                gang_replicas: 1,
                gang_min_available: 1,
                topology_affinity: "node".into(),
            },
            crate::services::job::Actor {
                tenant_id: 1,
                user_id: 1,
                is_admin: true,
            },
        )
        .await
        .expect("create job");

        dispatch_tick(&pool, &dispatch_config()).await.expect("dispatch tick");

        // 必须直接成功（不反复回退）：两卡共置 node-2。
        let j = crate::services::job::get_job_unscoped(&pool, created.id)
            .await
            .expect("get job");
        assert_eq!(
            j.status,
            job::status::RUNNING,
            "pre-selection must dispatch onto node-2 instead of livelocking on node-1"
        );
        let nodes = active_allocation_nodes(&pool, created.id).await;
        assert_eq!(nodes.len(), 2, "must allocate 2 cards");
        assert!(
            nodes.iter().all(|n| n == "node-2"),
            "all replicas must land on node-2 (the only node with enough cards), got {nodes:?}"
        );
        let matched = crate::metrics::scheduler_metrics().topology_matched_total.get();
        assert!(matched >= 1, "topology_matched_total should be incremented, got {matched}");

        // 完成后资源正常释放。
        assert!(
            wait_for_status(&pool, created.id, job::status::COMPLETED).await,
            "topology job should complete normally"
        );
        assert!(
            active_allocation_nodes(&pool, created.id).await.is_empty(),
            "allocations must be released after completion"
        );
    }

    /// 场景②：显式节点钉扎（`node:不存在` → pending + blocked；裸节点名 → 成功落位）。
    #[tokio::test]
    async fn test_topology_named_node_pin_blocks_and_then_places() {
        let pool = setup_pool().await;

        // 仅 node-1 有设备；作业钉到不存在的 node-9 → 必须 pending（不违反亲和）。
        seed_gpu_device(&pool, "node-1", 0).await;
        seed_quota(&pool, "pin-q", 4).await;

        let created = crate::services::job::create_job(
            &pool,
            crate::services::job::CreateJobInput {
                name: "pin-job".into(),
                description: "".into(),
                kind: "training".into(),
                priority: 0,
                gpus: 1,
                cpus: 0,
                memory: 0,
                duration: 0,
                cluster_id: 1,
                gang_replicas: 1,
                gang_min_available: 1,
                topology_affinity: "node:node-9".into(),
            },
            crate::services::job::Actor {
                tenant_id: 1,
                user_id: 1,
                is_admin: true,
            },
        )
        .await
        .expect("create job");

        dispatch_tick(&pool, &dispatch_config()).await.expect("dispatch tick");

        // 钉到不存在节点：保持 pending，无 active 分配，配额不残留，blocked 计数。
        let j = crate::services::job::get_job_unscoped(&pool, created.id)
            .await
            .expect("get job");
        assert_eq!(
            j.status,
            job::status::PENDING,
            "job pinned to nonexistent node must stay pending"
        );
        assert!(
            active_allocation_nodes(&pool, created.id).await.is_empty(),
            "blocked pinned job must not leave active allocations"
        );
        let q = quota::find_quota_for_scope(&pool, 1, None)
            .await
            .expect("find quota")
            .expect("some quota");
        assert_eq!(q.gpu_used, 0, "blocked pinned job must release reserved quota");
        let blocked = crate::metrics::scheduler_metrics().topology_blocked_total.get();
        assert!(blocked >= 1, "topology_blocked_total should be incremented, got {blocked}");

        // 改钉到裸节点名 node-1（非关键字值视为节点名）→ 成功且分配落在 node-1。
        set_topology_affinity(&pool, created.id, "node-1").await;
        dispatch_tick(&pool, &dispatch_config()).await.expect("dispatch tick");

        let nodes = active_allocation_nodes(&pool, created.id).await;
        assert_eq!(nodes, vec!["node-1".to_string()], "pinned allocation must be on node-1");
        assert!(
            wait_for_status(&pool, created.id, job::status::COMPLETED).await,
            "pinned job should complete normally"
        );
    }

    /// 前缀钉扎形式大小写不敏感："Node:node-1" 必须剥前缀钉住 node-1，
    /// 而不是整串被当裸节点名导致 blocked。
    #[tokio::test]
    async fn test_topology_pinned_prefix_case_insensitive() {
        let pool = setup_pool().await;

        seed_gpu_device(&pool, "node-1", 0).await;
        seed_quota(&pool, "case-q", 4).await;

        let created = crate::services::job::create_job(
            &pool,
            crate::services::job::CreateJobInput {
                name: "case-job".into(),
                description: "".into(),
                kind: "training".into(),
                priority: 0,
                gpus: 1,
                cpus: 0,
                memory: 0,
                duration: 0,
                cluster_id: 1,
                gang_replicas: 1,
                gang_min_available: 1,
                topology_affinity: "Node:node-1".into(),
            },
            crate::services::job::Actor {
                tenant_id: 1,
                user_id: 1,
                is_admin: true,
            },
        )
        .await
        .expect("create job");

        dispatch_tick(&pool, &dispatch_config()).await.expect("dispatch tick");

        // "Node:node-1" 应被识别为钉扎 node-1（节点存在）→ 直接派发成功。
        let j = crate::services::job::get_job_unscoped(&pool, created.id)
            .await
            .expect("get job");
        assert_eq!(
            j.status,
            job::status::RUNNING,
            "uppercase prefix Node:node-1 must be treated as a pin to node-1, not a bare name"
        );
        let nodes = active_allocation_nodes(&pool, created.id).await;
        assert_eq!(
            nodes,
            vec!["node-1".to_string()],
            "pinned allocation (case-insensitive prefix) must be on node-1"
        );
        assert!(
            wait_for_status(&pool, created.id, job::status::COMPLETED).await,
            "pinned job should complete normally"
        );
    }

    /// 加分场景：Gang × 拓扑组合——2 副本 × 1 GPU 的 all-or-nothing 准入还必须共置同节点。
    #[tokio::test]
    async fn test_gang_with_topology_affinity_places_all_replicas_on_one_node() {
        let pool = setup_pool().await;

        // node-1 两张卡（满足 2 副本共置）、node-2 一张卡（干扰项，不得使用）。
        seed_gpu_device(&pool, "node-1", 0).await;
        seed_gpu_device(&pool, "node-1", 1).await;
        seed_gpu_device(&pool, "node-2", 0).await;
        seed_quota(&pool, "gang-topo-q", 8).await;

        // Gang 作业：2 副本 × 1 GPU + topology_affinity="node"。
        let created = crate::services::job::create_job(
            &pool,
            crate::services::job::CreateJobInput {
                name: "gang-topo".into(),
                description: "".into(),
                kind: "training".into(),
                priority: 2,
                gpus: 1,
                cpus: 0,
                memory: 0,
                duration: 0,
                cluster_id: 1,
                gang_replicas: 2,
                gang_min_available: 2,
                topology_affinity: "node".into(),
            },
            crate::services::job::Actor {
                tenant_id: 1,
                user_id: 1,
                is_admin: true,
            },
        )
        .await
        .expect("create gang+topo job");

        dispatch_tick(&pool, &dispatch_config()).await.expect("dispatch tick");

        // 准入成功：gang_fulfilled=2，两卡必须共置 node-1。
        let j = crate::services::job::get_job_unscoped(&pool, created.id)
            .await
            .expect("get gang+topo job");
        assert_eq!(j.status, job::status::RUNNING, "gang+topo job should be admitted");
        assert_eq!(j.gang_fulfilled, 2, "gang_fulfilled must equal min_available");

        let nodes = active_allocation_nodes(&pool, created.id).await;
        assert_eq!(nodes.len(), 2, "must allocate min_available × gpus = 2 cards");
        assert!(
            nodes.iter().all(|n| n == "node-1"),
            "gang replicas must be co-located on node-1, got {nodes:?}"
        );

        // 完成后资源全量释放（2 卡 + 配额）。
        assert!(
            wait_for_status(&pool, created.id, job::status::COMPLETED).await,
            "gang+topo job should complete normally"
        );
        assert!(
            active_allocation_nodes(&pool, created.id).await.is_empty(),
            "all gang gpu allocations must be released after completion"
        );
        let q = quota::find_quota_for_scope(&pool, 1, None)
            .await
            .expect("find quota")
            .expect("some quota");
        assert_eq!(q.gpu_used, 0, "quota must be fully released after completion");
    }
}
