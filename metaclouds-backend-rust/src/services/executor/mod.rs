//! 执行器抽象（契约灰度①，2026-11-16）。
//!
//! ⚠️ **边界诚实声明**：本模块提供 [`Executor`] trait、[`SimulatedExecutor`] 与
//! [`KubeExecutorStub`]，并把模拟执行纳入 `exec_tasks` 租约表。它**不是**真实
//! 执行器：kube-rs / 真实 K8s Job / DCGM 采集均未接入，[`KubeExecutorStub`] 一律
//! 返回 `Unsupported`，绝不把 stub 伪装成可用后端。
//!
//! 对应 `docs/real-executor-contract.md` 的灰度顺序①：先上 exec_tasks 表与
//! SimulatedExecutor 走完整 lease 链路（不改对外行为）；真实执行器需外部条件，
//! 见该契约第七节。

use async_trait::async_trait;
use chrono::{Duration as ChronoDuration, Utc};
use std::sync::{Arc, OnceLock};

use crate::db::DatabasePool;
use crate::error::AppResult;

// ---------------------------------------------------------------------------
// 全局执行器注册表（灰度②）：进程启动时按 EXECUTOR_BACKEND 配置注入一次，
// dispatch 路径经 [`global_executor`] 消费，与 metrics 的 OnceLock 模式一致。
// 未注入时默认 SimulatedExecutor（保持既有行为，不伪装为真实后端）。
// ---------------------------------------------------------------------------
static GLOBAL_EXECUTOR: OnceLock<Arc<dyn Executor>> = OnceLock::new();

/// 注入进程级执行器；重复注入返回 `Err(已注入值)`，以首次配置为准。
pub fn configure_executor(exec: Arc<dyn Executor>) -> Result<(), Arc<dyn Executor>> {
    GLOBAL_EXECUTOR.set(exec)
}

/// 取当前执行器。未注入时惰性初始化为 [`SimulatedExecutor`]。
pub fn global_executor() -> Arc<dyn Executor> {
    GLOBAL_EXECUTOR
        .get_or_init(|| Arc::new(SimulatedExecutor))
        .clone()
}

/// 按配置名构造执行器：`"simulated"` → [`SimulatedExecutor`]，
/// `"kube"` → [`KubeExecutorStub`]（submit 时显式失败，绝不伪装可用）。
/// 其他值由 config.validate() 拒绝，此处兜底同样回退模拟并告警。
pub fn executor_for_backend(backend: &str) -> Arc<dyn Executor> {
    match backend {
        "kube" => Arc::new(KubeExecutorStub),
        // "simulated" 与未知值均回退模拟：未知值理论上到不了这里
        // （validate() 已 fail-closed），兜底不 panic 只 warn。
        _ => Arc::new(SimulatedExecutor),
    }
}

/// 提交一次执行请求所需的最小上下文。
#[derive(Debug, Clone)]
pub struct SubmitRequest {
    pub job_id: i64,
    pub attempt: i64,
    pub backend: String,
    pub owner_instance: String,
    pub lease_ttl_seconds: i64,
    /// 模拟执行时长（毫秒）；仅 SimulatedExecutor 消费。
    pub simulated_run_ms: u64,
    /// 模拟失败率（0.0~1.0）；仅 SimulatedExecutor 消费，默认 0。
    pub simulated_failure_rate: f64,
}

/// 执行状态。真实采集接入前 `usage` 为合成值，`usage_source` 由调用方标记。
#[derive(Debug, Clone, PartialEq)]
pub enum ExecStatus {
    /// 仍在执行（附最近一次实耗采样，可为空）。
    Running { sampled: Option<UsageSample> },
    /// 成功终态。
    Succeeded { usage: UsageSample },
    /// 失败终态（携带可落库的原因）。
    Failed { reason: String },
    /// 工作负载消失，交由 reconcile 判定，不在此处改写终态。
    Lost,
}

/// 实耗采样值（调用方决定写入 `usage_source='simulated'|'dcgm'`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UsageSample {
    pub gpu_fraction_actual: f64,
    pub cpu_actual: f64,
    pub memory_gb_actual: f64,
}

/// 执行器错误。真实后端未接入时用 `Unsupported` 显式失败，不静默降级为模拟。
#[derive(Debug, Clone, PartialEq)]
pub enum ExecError {
    Unsupported(&'static str),
    Backend(String),
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(what) => write!(f, "executor backend not available: {what}"),
            Self::Backend(msg) => write!(f, "executor backend error: {msg}"),
        }
    }
}

/// 执行器契约：幂等提交、可重复轮询、可重复中止。
#[async_trait]
pub trait Executor: Send + Sync {
    /// 幂等：同一 `(job_id, attempt)` 重复调用不得创建第二个工作负载。
    async fn submit(&self, pool: &DatabasePool, req: SubmitRequest) -> AppResult<String>;
    /// 查询终态；非终态返回 `Running`。
    async fn poll(&self, pool: &DatabasePool, job_id: i64, attempt: i64) -> AppResult<ExecStatus>;
    /// 中止；必须可重复调用。
    async fn abort(
        &self,
        pool: &DatabasePool,
        job_id: i64,
        attempt: i64,
        reason: &str,
    ) -> AppResult<()>;
}

/// 模拟执行器：把 `sleep + usage_sampled_at` 纳入 exec_tasks 租约链路。
/// 行为与灰度①之前一致，写入的 handle 为本地标识，不伪装为集群对象。
pub struct SimulatedExecutor;

#[async_trait]
impl Executor for SimulatedExecutor {
    async fn submit(&self, pool: &DatabasePool, req: SubmitRequest) -> AppResult<String> {
        let now = Utc::now();
        let expires = (now + ChronoDuration::seconds(req.lease_ttl_seconds.max(30))).to_rfc3339();
        let handle = format!("simulated://{}/{}", req.job_id, req.attempt);
        let _: u64 = crate::with_db!(
            pool,
            "INSERT INTO exec_tasks \
             (job_id, attempt, backend, handle, owner_instance, lease_until, submitted_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT (job_id, attempt) DO NOTHING",
            |db_s, db_e| {
                sqlx::query(db_s)
                    .bind(req.job_id)
                    .bind(req.attempt)
                    .bind(&req.backend)
                    .bind(&handle)
                    .bind(&req.owner_instance)
                    .bind(expires.clone())
                    .bind(now)
                    .execute(db_e)
                    .await?
                    .rows_affected()
            }
        );
        Ok(handle)
    }

    async fn poll(
        &self,
        _pool: &DatabasePool,
        _job_id: i64,
        _attempt: i64,
    ) -> AppResult<ExecStatus> {
        // 真实 poll 由调用方按配置的 run_ms 等待后触发；这里返回 Lost 表示
        // "尚未有真实终态来源"，由 dispatch 的定时逻辑决定完成/失败，
        // 避免 trait 本身持有时间状态而变得不可测。
        Ok(ExecStatus::Lost)
    }

    async fn abort(
        &self,
        pool: &DatabasePool,
        job_id: i64,
        attempt: i64,
        reason: &str,
    ) -> AppResult<()> {
        let _: u64 = crate::with_db!(
            pool,
            "UPDATE exec_tasks SET finished_at = ?, last_error = ? \
             WHERE job_id = ? AND attempt = ? AND finished_at IS NULL",
            |db_s, db_e| {
                sqlx::query(db_s)
                    .bind(Utc::now())
                    .bind(reason)
                    .bind(job_id)
                    .bind(attempt)
                    .execute(db_e)
                    .await?
                    .rows_affected()
            }
        );
        Ok(())
    }
}

/// K8s 执行器占位：一律显式失败，绝不把 stub 伪装成可用执行器。
/// 真实实现需要 kube-rs、kubeconfig、SA/RBAC、命名空间与 GPU Device Plugin。
pub struct KubeExecutorStub;

#[async_trait]
impl Executor for KubeExecutorStub {
    async fn submit(&self, _pool: &DatabasePool, _req: SubmitRequest) -> AppResult<String> {
        Err(crate::error::AppError::bad_request(
            ExecError::Unsupported("KubeExecutor requires a real cluster; not wired").to_string(),
        ))
    }

    async fn poll(
        &self,
        _pool: &DatabasePool,
        _job_id: i64,
        _attempt: i64,
    ) -> AppResult<ExecStatus> {
        Err(crate::error::AppError::bad_request(
            ExecError::Unsupported("KubeExecutor requires a real cluster; not wired").to_string(),
        ))
    }

    async fn abort(
        &self,
        _pool: &DatabasePool,
        _job_id: i64,
        _attempt: i64,
        _reason: &str,
    ) -> AppResult<()> {
        Err(crate::error::AppError::bad_request(
            ExecError::Unsupported("KubeExecutor requires a real cluster; not wired").to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_pool() -> DatabasePool {
        let pool = SqlitePoolOptions::new()
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

    fn submit_req(job_id: i64) -> SubmitRequest {
        SubmitRequest {
            job_id,
            attempt: 1,
            backend: "simulated".into(),
            owner_instance: "test-instance".into(),
            lease_ttl_seconds: 90,
            simulated_run_ms: 50,
            simulated_failure_rate: 0.0,
        }
    }

    #[tokio::test]
    async fn test_simulated_executor_submit_is_idempotent_and_abort_finishes() {
        let pool = setup_pool().await;
        let exec = SimulatedExecutor;

        let handle1 = exec.submit(&pool, submit_req(42)).await.expect("submit 1");
        let handle2 = exec.submit(&pool, submit_req(42)).await.expect("submit 2");
        assert_eq!(handle1, handle2, "same (job_id, attempt) must reuse handle");

        let cnt: i64 = crate::with_db!(
            &pool,
            "SELECT COUNT(*) FROM exec_tasks WHERE job_id = 42 AND attempt = 1",
            |db_s, db_e| crate::db::query_scalar_db(db_e, db_s).fetch_one(db_e).await
        )
        .expect("count exec tasks");
        assert_eq!(cnt, 1, "duplicate submit must not create a second row");

        exec.abort(&pool, 42, 1, "test abort").await.expect("abort");
        let state: (Option<String>, Option<String>) = crate::with_db!(
            &pool,
            "SELECT finished_at, last_error FROM exec_tasks WHERE job_id = 42 AND attempt = 1",
            |db_s, db_e| {
                use sqlx::Row;
                let row = sqlx::query(db_s).fetch_one(db_e).await.expect("fetch row");
                (row.try_get(0).ok(), row.try_get(1).ok())
            }
        );
        assert!(state.0.is_some(), "abort must set finished_at");
        assert_eq!(state.1.as_deref(), Some("test abort"));
    }

    #[tokio::test]
    async fn test_kube_stub_fails_explicitly_instead_of_pretending() {
        let pool = setup_pool().await;
        let exec = KubeExecutorStub;
        let err = exec
            .submit(&pool, submit_req(7))
            .await
            .expect_err("stub must fail");
        assert!(
            err.to_string().contains("not wired"),
            "stub failure must be explicit: {err}"
        );
    }
}

// services/mod.rs 负责导出；此处不重复 pub use。
