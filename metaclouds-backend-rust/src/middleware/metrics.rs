//! HTTP 请求指标中间件。
//!
//! 对齐 Go `metrics_business.go` 中的 HTTP 指标埋点：
//! - 进入时 `http_requests_in_flight` +1，结束时 -1。
//! - 记录 method / path / status 到 `http_requests_total`。
//! - 观测请求耗时到 `http_request_duration_seconds`。
//!
//! path 标签做归一化：优先使用 axum 注入的 [`MatchedPath`]（路由模板），
//! 否则把纯数字路径段替换为 `{id}`，避免把实际 ID 写入标签导致高基数。

use std::time::Instant;

use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::Response;

use crate::metrics::http_metrics;

/// 把路径中的纯数字段替换为 `{id}`，防止路径参数（如 `/users/123`）
/// 导致 label 基数爆炸。
fn normalize_path(path: &str) -> String {
    path.split('/')
        .map(|seg| {
            if seg.parse::<i64>().is_ok() {
                "{id}"
            } else {
                seg
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// 中间件：统计 HTTP 请求指标。
pub async fn http_metrics_middleware(request: Request, next: Next) -> Response {
    let metrics = http_metrics();
    metrics.inc_in_flight();

    let method = request.method().to_string();
    // 优先使用路由模板（MatchedPath），否则归一化原始路径。
    let path = request
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| normalize_path(request.uri().path()));

    let start = Instant::now();
    let response = next.run(request).await;
    let duration_secs = start.elapsed().as_secs_f64();

    let status = response.status().as_u16();
    metrics.record(&method, &path, status, duration_secs);
    metrics.dec_in_flight();

    response
}
