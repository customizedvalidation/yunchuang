//! 统一错误处理中间件。
//!
//! `AppError` 已经通过 `IntoResponse` 输出统一信封；本中间件补齐剩下的缺口：
//! axum 对“无路由匹配 / 方法不允许”的默认响应是纯文本，而非 JSON 信封。
//! 这里在响应返回后检查：若状态码为 404/405 且正文仍是 `text/plain`，
//! 则用统一信封重写，保证任何对外响应（含路由错误）都符合 Go 侧契约。

use axum::extract::Request;
use axum::http::header;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::ErrorCode;
use crate::response::ApiResponse;

/// 判断该响应是否是 axum 默认的纯文本路由错误。
fn is_plain_text_route_error(response: &Response) -> bool {
    let status = response.status();
    if !matches!(
        status,
        StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
    ) {
        return false;
    }
    match response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
    {
        // axum 0.8 的默认 404/405 可能不带 content-type，同样按路由错误处理。
        None => true,
        Some(ct) => ct.contains("text/plain"),
    }
}

/// 用统一信封替换纯文本路由错误响应（保留原状态码）。
fn envelope_for(status: StatusCode) -> Response {
    let (code, message) = match status {
        StatusCode::NOT_FOUND => (ErrorCode::NotFound, "route not found"),
        StatusCode::METHOD_NOT_ALLOWED => (ErrorCode::BadRequest, "method not allowed"),
        _ => (ErrorCode::InternalServerError, "internal server error"),
    };
    let body = ApiResponse::<()>::error(message, code.as_str());
    (status, axum::Json(body)).into_response()
}

/// 中间件：把路由层产生的纯文本错误改写成统一 JSON 信封。
pub async fn error_handler(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    if is_plain_text_route_error(&response) {
        envelope_for(response.status())
    } else {
        response
    }
}
