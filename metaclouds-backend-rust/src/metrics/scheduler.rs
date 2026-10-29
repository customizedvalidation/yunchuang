//! 调度器 / 配额 / GPU 分配相关业务指标（Prometheus）。
//!
//! 与 [`crate::metrics::business`] 的静态 Gauge 不同，本组指标由调度器与
//! 分配逻辑在运行时主动埋点（counter / gauge / histogram），反映**进程内
//! 优先级队列的实时行为**，而不是 DB 快照。
//!
//! 命名统一带 `metaclouds_scheduler_` 前缀，避免与 [`business`] 里由
//! dashboard stats 刷新的 `metaclouds_running_jobs`（DB 口径）重名导致
//! 重复注册 panic。

use prometheus::{
    register_histogram, register_int_counter, register_int_counter_vec, register_int_gauge,
    register_int_gauge_vec, Histogram, IntCounter, IntCounterVec, IntGauge, IntGaugeVec,
};

/// 调度器 / 配额 / GPU 分配指标句柄。
///
/// 通过 [`crate::metrics::scheduler_metrics`] 的 `OnceLock` 单次初始化，
/// 挂到 Prometheus 全局默认注册表；切勿在两处用同名 `register_*!` 重复注册。
#[derive(Clone)]
pub struct SchedulerMetrics {
    /// 各优先级（0..=3）pending 的 job 数 Gauge。
    pub pending_jobs: IntGaugeVec,
    /// 当前正在运行的 job 数 Gauge（进程内 in-memory 口径）。
    pub running_jobs: IntGauge,
    /// 累计完成的 job 数。
    pub completed_total: IntCounter,
    /// 累计失败的 job 数。
    pub failed_total: IntCounter,
    /// 累计被取消的 job 数。
    pub cancelled_total: IntCounter,
    /// 运行中 job 被强制 abort 的次数，label `reason` = `cancel_user` | `shutdown`。
    pub job_aborted_total: IntCounterVec,
    /// job 从入队到被弹出调度的等待延迟（秒）。
    pub schedule_latency_seconds: Histogram,
    /// 因配额不足被拒绝的分配次数，label `tenant` / `resource`。
    pub quota_rejected_total: IntCounterVec,
    /// GPU 分配失败次数，label `reason` = `no_capacity` | `quota_exceeded` | `error`。
    pub gpu_allocation_failed_total: IntCounterVec,
}

impl SchedulerMetrics {
    /// 向 Prometheus 全局注册表注册全部调度器指标。
    pub fn new() -> Self {
        let pending_jobs = register_int_gauge_vec!(
            "metaclouds_scheduler_pending_jobs",
            "Number of pending jobs in the priority queue, labeled by priority level.",
            &["priority"]
        )
        .expect("register metaclouds_scheduler_pending_jobs");

        let running_jobs = register_int_gauge!(
            "metaclouds_scheduler_running_jobs",
            "Number of jobs currently running in the in-process scheduler."
        )
        .expect("register metaclouds_scheduler_running_jobs");

        let completed_total = register_int_counter!(
            "metaclouds_scheduler_completed_total",
            "Total number of jobs that reached the Completed state."
        )
        .expect("register metaclouds_scheduler_completed_total");

        let failed_total = register_int_counter!(
            "metaclouds_scheduler_failed_total",
            "Total number of jobs that reached the Failed state."
        )
        .expect("register metaclouds_scheduler_failed_total");

        let cancelled_total = register_int_counter!(
            "metaclouds_scheduler_cancelled_total",
            "Total number of jobs that reached the Cancelled state."
        )
        .expect("register metaclouds_scheduler_cancelled_total");

        let job_aborted_total = register_int_counter_vec!(
            "metaclouds_scheduler_job_aborted_total",
            "Total number of running jobs force-aborted, labeled by reason.",
            &["reason"]
        )
        .expect("register metaclouds_scheduler_job_aborted_total");

        // 与任务约定一致的 buckets：覆盖亚毫秒到秒级调度等待。
        let schedule_latency_seconds = register_histogram!(
            "metaclouds_scheduler_schedule_latency_seconds",
            "Wait time from enqueue to dispatch (seconds).",
            vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]
        )
        .expect("register metaclouds_scheduler_schedule_latency_seconds");

        let quota_rejected_total = register_int_counter_vec!(
            "metaclouds_quota_rejected_total",
            "Number of allocations rejected due to quota limits, labeled by tenant and resource.",
            &["tenant", "resource"]
        )
        .expect("register metaclouds_quota_rejected_total");

        let gpu_allocation_failed_total = register_int_counter_vec!(
            "metaclouds_gpu_allocation_failed_total",
            "Number of GPU allocation failures, labeled by reason.",
            &["reason"]
        )
        .expect("register metaclouds_gpu_allocation_failed_total");

        Self {
            pending_jobs,
            running_jobs,
            completed_total,
            failed_total,
            cancelled_total,
            job_aborted_total,
            schedule_latency_seconds,
            quota_rejected_total,
            gpu_allocation_failed_total,
        }
    }
}

impl Default for SchedulerMetrics {
    fn default() -> Self {
        Self::new()
    }
}
