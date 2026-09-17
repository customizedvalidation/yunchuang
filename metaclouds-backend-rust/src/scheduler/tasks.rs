//! 定时任务体：对齐 Go 版 `services/scheduler.go` 的 `executeJob` 语义。
//!
//! 每个任务通过 [`spawn_task_safely`] 在独立的 tokio 任务中运行：
//! 任务内部 panic 只会让 `JoinHandle::is_panic()` 为 true，
//! 绝不会冒泡进 tokio-cron-scheduler 的调度循环，也不会影响其他任务。

use std::future::Future;
use std::sync::Arc;

use sqlx::SqlitePool;
use tracing::{error, info, warn};

use crate::config::Config;

/// 每个任务的共享上下文（由调度器在注册时克隆进闭包）。
#[derive(Clone)]
pub struct TaskContext {
    pub pool: SqlitePool,
    pub config: Arc<Config>,
}

/// 在独立 tokio 任务中运行任务体，并吞掉 panic。
///
/// 与 Go robfig/cron 每个 job 独立 goroutine + recover 的语义对齐：
/// 任务 panic 仅记录日志，调度循环与其他任务不受影响。
pub async fn spawn_task_safely<F, Fut>(name: &'static str, body: F)
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let handle = tokio::spawn(async move {
        body().await;
    });

    match handle.await {
        Ok(()) => info!(task = name, "scheduled task completed"),
        Err(join) if join.is_panic() => {
            error!(
                task = name,
                "scheduled task panicked (isolated, scheduler continues)"
            )
        }
        Err(join) => warn!(task = name, error = %join, "scheduled task cancelled"),
    }
}

/// `sample-training`：对齐 Go `executeJob(jobID=1, "sample-training")`。
///
/// Go 逻辑：查 job=1 → pending 则置 running → 提交 K8s → 监控完成。
/// 当前阶段落地为骨架日志 + TODO；K8s 提交等业务随 `services::k8s` 接通后填充。
pub async fn sample_training(ctx: TaskContext) {
    info!(
        task = "sample-training",
        job_id = 1,
        cron = "*/30 * * * *",
        "executing scheduled training job (skeleton)"
    );
    // TODO(P3-02): 对齐 Go executeJobOnce：
    //   1. jobService.GetJob(1)
    //   2. pending → status=running
    //   3. k8sService.SubmitJob({JobID:1})，失败回滚为 failed
    //   4. monitorJobCompletion(jobID=1) 轮询至终态
    let _ = ctx;
}

/// `sample-inference`：对齐 Go `executeJob(jobID=2, "sample-inference")`。
pub async fn sample_inference(ctx: TaskContext) {
    info!(
        task = "sample-inference",
        job_id = 2,
        cron = "0 */2 * * *",
        "executing scheduled inference job (skeleton)"
    );
    // TODO(P3-02): 同 sample_training，job_id=2。
    let _ = ctx;
}
