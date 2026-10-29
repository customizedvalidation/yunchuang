//! 13 个业务指标（Gauge），对齐 B6 monitoring dashboard stats。
//!
//! 指标名带 `metaclouds_` 命名空间前缀（对齐 Go `metrics_business.go` 的约定）。
//! 值由 [`crate::services::monitoring::get_dashboard_stats`] 从 DB 拉取后刷新，
//! 不在此重复实现统计逻辑。

use prometheus::{
    register_int_gauge, register_int_gauge_vec, IntGauge, IntGaugeVec,
};

/// 持有全部业务 Gauge 句柄。
pub struct BusinessMetrics {
    pub total_users: IntGauge,
    pub active_users: IntGauge,
    pub total_tenants: IntGauge,
    pub total_clusters: IntGauge,
    pub total_resources: IntGauge,
    pub total_jobs: IntGauge,
    pub running_jobs: IntGauge,
    /// 队列中 pending 的 job 总数（供 JobQueueLength 告警；DB/调度口径由后端 A 透出）。
    pub pending_jobs: IntGauge,
    pub total_gpus: IntGauge,
    pub allocated_gpus: IntGauge,
    pub total_datasets: IntGauge,
    pub total_alerts: IntGauge,
    pub active_alerts: IntGauge,
    pub system_uptime: IntGauge,
    /// Redis 缓存累计命中次数（原始原子值，label kind="redis"）。
    pub cache_hits_total: IntGaugeVec,
    /// Redis 缓存累计未命中次数（原始原子值，label kind="redis"）。
    pub cache_misses_total: IntGaugeVec,
}

impl BusinessMetrics {
    /// 向 Prometheus 全局注册表注册全部 13 个 Gauge。
    ///
    /// 应通过 [`crate::metrics::business_metrics`] 的 `OnceLock` 单次调用，
    /// 避免重复注册同名指标导致 panic。
    pub fn new() -> Self {
        Self {
            total_users: register_int_gauge!(
                "metaclouds_total_users",
                "Total number of users in the system."
            )
            .expect("register metaclouds_total_users"),
            active_users: register_int_gauge!(
                "metaclouds_active_users",
                "Number of active (recently used) users."
                // TODO: 当前 schema 无 session 表，值 = registered_users；
                // 待接入登录会话表后改为 24h 活跃去重计数。
            )
            .expect("register metaclouds_active_users"),
            total_tenants: register_int_gauge!(
                "metaclouds_total_tenants",
                "Total number of tenants."
            )
            .expect("register metaclouds_total_tenants"),
            total_clusters: register_int_gauge!(
                "metaclouds_total_clusters",
                "Total number of clusters."
            )
            .expect("register metaclouds_total_clusters"),
            total_resources: register_int_gauge!(
                "metaclouds_total_resources",
                "Total number of resources."
            )
            .expect("register metaclouds_total_resources"),
            total_jobs: register_int_gauge!("metaclouds_total_jobs", "Total number of jobs.")
                .expect("register metaclouds_total_jobs"),
            running_jobs: register_int_gauge!("metaclouds_running_jobs", "Number of running jobs.")
                .expect("register metaclouds_running_jobs"),
            pending_jobs: register_int_gauge!(
                "metaclouds_pending_jobs",
                "Number of pending (queued) jobs. Placeholder until backend A exposes pending_jobs in dashboard stats."
            )
            .expect("register metaclouds_pending_jobs"),
            total_gpus: register_int_gauge!(
                "metaclouds_total_gpus",
                "Total number of GPU devices."
            )
            .expect("register metaclouds_total_gpus"),
            allocated_gpus: register_int_gauge!(
                "metaclouds_allocated_gpus",
                "Number of allocated GPUs."
            )
            .expect("register metaclouds_allocated_gpus"),
            total_datasets: register_int_gauge!(
                "metaclouds_total_datasets",
                "Total number of datasets."
            )
            .expect("register metaclouds_total_datasets"),
            total_alerts: register_int_gauge!("metaclouds_total_alerts", "Total number of alerts.")
                .expect("register metaclouds_total_alerts"),
            active_alerts: register_int_gauge!(
                "metaclouds_active_alerts",
                "Number of active (unresolved) alerts."
            )
            .expect("register metaclouds_active_alerts"),
            system_uptime: register_int_gauge!(
                "metaclouds_system_uptime",
                "System uptime in seconds."
            )
            .expect("register metaclouds_system_uptime"),
            cache_hits_total: register_int_gauge_vec!(
                "metaclouds_cache_hits_total",
                "Cumulative Redis cache hits (raw atomic value), labeled by backend kind.",
                &["kind"]
            )
            .expect("register metaclouds_cache_hits_total"),
            cache_misses_total: register_int_gauge_vec!(
                "metaclouds_cache_misses_total",
                "Cumulative Redis cache misses (raw atomic value), labeled by backend kind.",
                &["kind"]
            )
            .expect("register metaclouds_cache_misses_total"),
        }
    }

    /// 从 dashboard stats JSON 刷新全部 13 个 Gauge。
    ///
    /// `stats` 由 [`crate::services::monitoring::get_dashboard_stats`] 返回，
    /// 键名与 B6 对齐（total_users / active_users / ...）。
    pub fn update_from_stats(&self, stats: &serde_json::Value) {
        let g = |name: &str| -> i64 { stats.get(name).and_then(|v| v.as_i64()).unwrap_or(0) };
        self.total_users.set(g("total_users"));
        self.active_users.set(g("active_users"));
        self.total_tenants.set(g("total_tenants"));
        self.total_clusters.set(g("total_clusters"));
        self.total_resources.set(g("total_resources"));
        self.total_jobs.set(g("total_jobs"));
        self.running_jobs.set(g("running_jobs"));
        // pending_jobs 暂由后端 A 在 dashboard stats 中透出；未透出时 g() 回退 0。
        self.pending_jobs.set(g("pending_jobs"));
        self.total_gpus.set(g("total_gpus"));
        self.allocated_gpus.set(g("allocated_gpus"));
        self.total_datasets.set(g("total_datasets"));
        self.total_alerts.set(g("total_alerts"));
        self.active_alerts.set(g("active_alerts"));
        self.system_uptime.set(g("system_uptime"));
    }
}

impl Default for BusinessMetrics {
    fn default() -> Self {
        Self::new()
    }
}
