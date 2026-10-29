//! Prometheus 指标模块。
//!
//! 聚合 13 个业务 Gauge（[`business`]）+ 3 个 HTTP 请求指标（[`http`]），
//! 通过 `/metrics` 端点以 Prometheus 文本格式暴露。
//!
//! 指标名带 `metaclouds_` 前缀（对齐 Go `metrics_business.go` 的命名约定），
//! HTTP 指标名 / 标签 / buckets 与 Go 版逐字一致。
//!
//! 注册采用 `OnceLock` 单次初始化，挂到 Prometheus 全局默认注册表；
//! [`prometheus::gather`] 会自动收集所有已注册指标。

pub mod business;
pub mod http;
pub mod scheduler;

use std::sync::OnceLock;

use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use prometheus::{Encoder, TextEncoder};

use crate::auth::middleware::AppState;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::services::monitoring::get_dashboard_stats;

/// 业务指标全局句柄（进程级单次初始化）。
static BUSINESS: OnceLock<business::BusinessMetrics> = OnceLock::new();
/// HTTP 指标全局句柄（进程级单次初始化）。
static HTTP: OnceLock<http::HttpMetrics> = OnceLock::new();
/// 调度器 / 配额 / GPU 分配指标全局句柄（进程级单次初始化）。
static SCHEDULER: OnceLock<scheduler::SchedulerMetrics> = OnceLock::new();

/// 获取（或首次初始化）业务指标句柄。
pub fn business_metrics() -> &'static business::BusinessMetrics {
    BUSINESS.get_or_init(business::BusinessMetrics::new)
}

/// 获取（或首次初始化）HTTP 指标句柄。
pub fn http_metrics() -> &'static http::HttpMetrics {
    HTTP.get_or_init(http::HttpMetrics::new)
}

/// 获取（或首次初始化）调度器指标句柄。
pub fn scheduler_metrics() -> &'static scheduler::SchedulerMetrics {
    SCHEDULER.get_or_init(scheduler::SchedulerMetrics::new)
}

/// 收集全局注册表中全部指标并编码为 Prometheus 文本格式。
///
/// 编码失败时不 panic：prometheus 侧异常属内部错误，交由全局错误处理
/// 转成 5xx 信封（panic 会让 /metrics 抓取线程崩溃并被 recovery 兜成裸 500）。
pub fn encode_metrics() -> AppResult<String> {
    let encoder = TextEncoder::new();
    let families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&families, &mut buffer).map_err(|e| {
        AppError::with_source(
            ErrorCode::InternalServerError,
            "failed to encode prometheus metrics",
            e,
        )
    })?;
    String::from_utf8(buffer).map_err(|e| {
        AppError::with_source(
            ErrorCode::InternalServerError,
            "prometheus metrics output is not valid utf-8",
            e,
        )
    })
}

/// `GET /metrics` handler：先从 DB 刷新业务 Gauge，再返回 Prometheus 文本。
///
/// 对齐 Go `metrics.go` 的 `MetricsHandler()`：无 JWT 认证，供 Prometheus scraper 直连。
/// 若配置 `prometheus_enabled = false`，返回 404。
pub async fn metrics_handler(State(state): State<AppState>) -> AppResult<impl IntoResponse> {
    if !state.config.prometheus_enabled {
        return Ok((
            axum::http::StatusCode::NOT_FOUND,
            "prometheus metrics disabled",
        )
            .into_response());
    }

    // 每次抓取时从 DB 拉取最新 dashboard stats 刷新业务 Gauge，
    // 避免依赖 P3-02 定时任务（此处保证 /metrics 始终反映当前值）。
    if let Ok(stats) = get_dashboard_stats(&state.pool).await {
        business_metrics().update_from_stats(&stats);
    }

    // 刷新 Redis 缓存命中/未命中 Gauge（原始原子值，kind="redis"）。
    {
        let bs = business_metrics();
        let cs = crate::cache::global_cache_stats();
        bs.cache_hits_total
            .with_label_values(&["redis"])
            .set(cs.hits as i64);
        bs.cache_misses_total
            .with_label_values(&["redis"])
            .set(cs.misses as i64);
    }

    let body = encode_metrics()?;
    Ok((
        [(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
        .into_response())
}
