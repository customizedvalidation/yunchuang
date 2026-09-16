//! 请求日志中间件。
//!
//! 在响应返回后以结构化字段记录一次请求的概要，字段对齐 Go 侧
//! `middlewares/request_logger.go`：method、path、status_code、duration_ms、
//! client_ip、request_id，若请求扩展中已注入 JWT Claims 则附带 user_id。
//!
//! 本中间件必须排在 `request_id` 之后运行，才能在日志中关联到请求 ID；
//! 它运行在路由级 `jwt_auth` 之外，因此 user_id 通常为空（仅在把本中间件
//! 挂在认证层内侧时才会有值），按“如果有则记录”处理。

use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::jwt::Claims;
use crate::middleware::request_id::RequestId;

/// 从请求头中尽力解析客户端 IP（仅看 `X-Forwarded-For` 第一段）。
///
/// 测试与直连场景下没有 `ConnectInfo`，此处退化为 "unknown"，与 Go 侧
/// `c.ClientIP()` 在合成请求下返回空串的行为等价。
fn client_ip(request: &Request) -> String {
    if let Some(xff) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(ip) = xff.split(',').next() {
            let ip = ip.trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }
    "unknown".to_string()
}

/// 中间件：计时并在响应后输出结构化访问日志。
pub async fn request_logger(request: Request, next: Next) -> Response {
    let start = Instant::now();

    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let client_ip = client_ip(&request);
    // 在消费 request 前，尽力取出请求 ID 与（可能已注入的）用户 ID。
    let request_id = request.extensions().get::<RequestId>().map(|r| r.0.clone());
    let user_id = request.extensions().get::<Claims>().map(|c| c.user_id);

    let response = next.run(request).await;

    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    let status_code = response.status().as_u16();

    tracing::info!(
        method = %method,
        path = %path,
        status_code = status_code,
        duration_ms = duration_ms,
        client_ip = %client_ip,
        user_id = user_id,
        request_id = request_id.as_deref().unwrap_or("-"),
        "request completed"
    );

    response
}
