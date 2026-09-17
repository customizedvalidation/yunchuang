//! P3-02：tokio-cron-scheduler 定时任务调度器。
//!
//! # 与 Go 版对齐
//!
//! - Go：`services/scheduler.go` 用 `robfig/cron/v3`（5 字段、本地时区），
//!   仅非生产环境注册两个示例任务（见 `DEFAULT_JOBS` 与
//!   `docs/cron-migration-reference.md`）。
//! - Rust：tokio-cron-scheduler 0.10，7 字段（秒 分 时 日 月 周 年），
//!   统一 UTC 时区（`chrono::Utc`），字段映射对照表见 docs。
//!
//! # 任务隔离
//!
//! 每个任务体经 [`tasks::spawn_task_safely`] 包装，在独立 tokio 任务中运行；
//! 任务 panic 不会冒泡进调度循环，也不影响其他任务。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use chrono::Utc;
use sqlx::SqlitePool;
use tokio::sync::Mutex;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::info;

use crate::config::Config;
use crate::error::{AppError, AppResult, ErrorCode};
pub mod tasks;

pub use tasks::TaskContext;

/// 默认任务表：`(任务名, Go 表达式, Rust 7 字段表达式)`。
///
/// 与 `docs/cron-migration-reference.md` §3 逐行一致，
/// 数据来自 Go `services/scheduler.go::AddDefaultSchedules`。
pub const DEFAULT_JOBS: &[(&str, &str, &str)] = &[
    ("sample-training", "*/30 * * * *", "0 */30 * * * * *"),
    ("sample-inference", "0 */2 * * *", "0 0 */2 * * * *"),
];

/// cron 调度器封装。
///
/// 内部用 `Arc<Mutex<JobScheduler>>` 做内部可变性：`start` / `shutdown` 均可通过
/// `&self` 调用，便于作为 `Arc<Scheduler>` 在 main.rs 与信号处理间共享。
pub struct Scheduler {
    inner: Arc<Mutex<JobScheduler>>,
    ctx: TaskContext,
    /// 已注册任务数（仅统计默认任务），供测试断言。
    registered: Arc<AtomicUsize>,
}

impl Scheduler {
    /// 构造调度器。异步是因为 `JobScheduler::new().await` 为异步。
    pub async fn new(pool: SqlitePool, config: Arc<Config>) -> AppResult<Self> {
        let inner = JobScheduler::new().await.map_err(|e| {
            AppError::with_source(ErrorCode::InternalServerError, "create cron scheduler", e)
        })?;
        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
            ctx: TaskContext { pool, config },
            registered: Arc::new(AtomicUsize::new(0)),
        })
    }

    /// 注册全部默认 cron 任务。
    ///
    /// 对齐 Go `main.go`：`Environment == "production"` 时不注册示例任务，
    /// 调度器仍然启动（生产环境留空，由后续动态注册 API 填充）。
    pub async fn register_all_jobs(&self) -> AppResult<()> {
        if self.ctx.config.environment == "production" {
            tracing::warn!("scheduler running in production: default sample jobs not registered");
            return Ok(());
        }

        let scheduler = self.inner.lock().await;
        for &(name, go_expr, rust_expr) in DEFAULT_JOBS {
            let ctx = self.ctx.clone();
            // Job 闭包会被多次调用（每次触发），因此每次调用时重新 clone ctx，
            // 且任务体经 spawn_task_safely 在独立任务中隔离 panic。
            let job = Job::new_async_tz(rust_expr, Utc, move |_uuid, _lock| {
                let ctx = ctx.clone();
                Box::pin(async move {
                    tasks::spawn_task_safely(name, move || async move {
                        match name {
                            "sample-training" => tasks::sample_training(ctx).await,
                            "sample-inference" => tasks::sample_inference(ctx).await,
                            other => {
                                tracing::warn!(task = other, "no task body registered");
                            }
                        }
                    })
                    .await;
                })
            })
            .map_err(|e| {
                AppError::with_source(
                    ErrorCode::InternalServerError,
                    format!("build job '{name}' with cron '{rust_expr}' (go: '{go_expr}')"),
                    e,
                )
            })?;

            let id = scheduler.add(job).await.map_err(|e| {
                AppError::with_source(
                    ErrorCode::InternalServerError,
                    format!("add job '{name}' to scheduler"),
                    e,
                )
            })?;
            self.registered.fetch_add(1, Ordering::SeqCst);
            info!(
                task = name,
                go_expr,
                rust_expr,
                job_id = %id,
                "registered scheduled job"
            );
        }
        Ok(())
    }

    /// 启动调度器（开始按 cron 表达式触发任务）。
    pub async fn start(&self) -> AppResult<()> {
        info!(
            count = self.registered.load(Ordering::SeqCst),
            "starting cron scheduler"
        );
        let s = self.inner.lock().await;
        s.start().await.map_err(|e| {
            AppError::with_source(ErrorCode::InternalServerError, "scheduler start", e)
        })
    }

    /// 优雅关闭：不再触发新任务，等待在跑任务结束。
    pub async fn shutdown(&self) -> AppResult<()> {
        info!("shutting down cron scheduler");
        let mut s = self.inner.lock().await;
        s.shutdown().await.map_err(|e| {
            AppError::with_source(ErrorCode::InternalServerError, "scheduler shutdown", e)
        })
    }

    /// 已注册任务数（供测试断言与 Go 版一致）。
    pub fn registered_count(&self) -> usize {
        self.registered.load(Ordering::SeqCst)
    }
}
