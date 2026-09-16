//! CSRF 双提交令牌中间件。
//!
//! 对齐 Go 侧 `middlewares/csrf.go`：
//!
//! - Cookie `csrf_token`（非 httpOnly，供同源 JS 读取），请求头 `X-CSRF-Token`。
//! - 仅对 `POST/PUT/DELETE/PATCH` 这四类状态变更方法校验；GET/HEAD/OPTIONS 放行。
//! - 仅当请求携带 `access_token` Cookie（浏览器会话）时强制校验；走
//!   `Authorization: Bearer` 的非浏览器客户端没有该 Cookie，直接跳过。
//! - 校验「请求头值 == Cookie 值」，使用常量时间比较，避免计时侧信道。
//! - 缺失或不匹配均返回 403：分别给出
//!   "CSRF token missing or invalid" 与 "CSRF token mismatch"。

use axum::extract::Request;
use axum::http::header::HeaderName;
use axum::middleware::Next;
use axum::response::Response;
use tower_cookies::Cookies;

use crate::error::{AppError, AppResult};

/// 双提交令牌所在的 Cookie 名（非 httpOnly，JS 可读）。
pub const CSRF_COOKIE_NAME: &str = "csrf_token";
/// 前端在请求头中回传令牌的名字。
pub const CSRF_HEADER_NAME: &str = "X-CSRF-Token";
/// 存放 JWT 的 httpOnly Cookie 名（存在即视为浏览器会话）。
pub const AUTH_COOKIE_NAME: &str = "access_token";

static X_CSRF_TOKEN: HeaderName = HeaderName::from_static("x-csrf-token");

/// 判断该方法是否属于需要 CSRF 校验的状态变更方法。
fn is_protected_method(method: &axum::http::Method) -> bool {
    matches!(
        *method,
        axum::http::Method::POST
            | axum::http::Method::PUT
            | axum::http::Method::DELETE
            | axum::http::Method::PATCH
    )
}

/// 常量时间比较两个字节串，避免通过响应时间侧信道推断令牌差异。
///
/// 本仓库未引入 `subtle` crate，这里用“逐字节异或后汇总”的等价实现：
/// 只有在全程无任何差异字节时才返回 true，比较耗时与是否含差异无关。
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// 生成 32 字节密码学随机令牌并做 hex 编码（64 个十六进制字符），
/// 对齐 Go `generateCSRFToken`。
pub fn generate_csrf_token() -> String {
    use rand::RngCore;
    let mut buf = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut buf);
    let mut out = String::with_capacity(64);
    for b in buf {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// CSRF 双提交校验中间件。
pub async fn csrf_protect(cookies: Cookies, request: Request, next: Next) -> AppResult<Response> {
    // 幂等读取方法不校验。
    if !is_protected_method(request.method()) {
        return Ok(next.run(request).await);
    }

    // 没有 access_token Cookie：视为非浏览器（Bearer）客户端，跳过校验。
    if cookies.get(AUTH_COOKIE_NAME).is_none() {
        return Ok(next.run(request).await);
    }

    let header_token = request
        .headers()
        .get(&X_CSRF_TOKEN)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let cookie_token = cookies.get(CSRF_COOKIE_NAME).map(|c| c.value().to_string());

    match (header_token, cookie_token) {
        (Some(header), Some(cookie)) if constant_time_eq(header.as_bytes(), cookie.as_bytes()) => {
            Ok(next.run(request).await)
        }
        (Some(_), Some(_)) => {
            tracing::warn!(
                path = %request.uri().path(),
                method = %request.method(),
                "CSRF validation failed - token mismatch"
            );
            Err(AppError::forbidden("CSRF token mismatch"))
        }
        _ => {
            tracing::warn!(
                path = %request.uri().path(),
                method = %request.method(),
                "CSRF validation failed - token missing"
            );
            Err(AppError::forbidden("CSRF token missing or invalid"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csrf_token_is_64_hex_chars() {
        let t = generate_csrf_token();
        assert_eq!(t.len(), 64);
        assert!(t.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn constant_time_eq_matches_and_rejects() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(constant_time_eq(b"", b""));
    }
}
