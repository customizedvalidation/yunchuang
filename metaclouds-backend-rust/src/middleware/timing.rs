//! 计时中间件。
//!
//! 测量请求耗时，并把毫秒数写入 `X-Response-Time` 响应头（如 "1.23ms"），
//! 对齐 Go 侧 `middlewares/timing.go` 对响应时延的观测语义。

use std::time::Instant;

use axum::extract::Request;
use axum::http::header::HeaderName;
use axum::middleware::Next;
use axum::response::Response;

static X_RESPONSE_TIME: HeaderName = HeaderName::from_static("x-response-time");

/// 中间件：计时并注入 `X-Response-Time` 响应头。
pub async fn timing(request: Request, next: Next) -> Response {
    let start = Instant::now();
    let mut response = next.run(request).await;

    let ms = start.elapsed().as_secs_f64() * 1000.0;
    if let Ok(value) = format!("{ms:.2}ms").parse() {
        response
            .headers_mut()
            .insert(X_RESPONSE_TIME.clone(), value);
    }
    response
}
