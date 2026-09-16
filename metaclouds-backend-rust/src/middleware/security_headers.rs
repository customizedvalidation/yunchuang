//! 安全响应头中间件。
//!
//! 对齐 Go 侧 `middlewares/security_headers.go`：固定注入一组防嗅探/防点击劫持/
//! 引用策略/权限策略头，并按运行环境选择 CSP；生产环境额外开启 HSTS。
//!
//! 运行环境通过环境变量 `SERVER_ENV` 读取（默认 `development`），与 Go 配置
//! `config.go` 中 `getEnv("SERVER_ENV", "development")` 保持一致；本仓库的
//! `Config` 结构尚未包含环境字段，故直接读环境变量，避免修改 `config.rs`。

use axum::extract::Request;
use axum::http::header::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;

use tower_cookies::cookie::SameSite;

static X_CONTENT_TYPE_OPTIONS: HeaderName = HeaderName::from_static("x-content-type-options");
static X_FRAME_OPTIONS: HeaderName = HeaderName::from_static("x-frame-options");
static X_XSS_PROTECTION: HeaderName = HeaderName::from_static("x-xss-protection");
static REFERRER_POLICY: HeaderName = HeaderName::from_static("referrer-policy");
static PERMISSIONS_POLICY: HeaderName = HeaderName::from_static("permissions-policy");
static STRICT_TRANSPORT_SECURITY: HeaderName = HeaderName::from_static("strict-transport-security");
static CONTENT_SECURITY_POLICY: HeaderName = HeaderName::from_static("content-security-policy");
static SERVER: HeaderName = HeaderName::from_static("server");

/// 是否运行在生产环境（`SERVER_ENV=production`）。
pub fn is_production() -> bool {
    std::env::var("SERVER_ENV")
        .map(|v| v.eq_ignore_ascii_case("production"))
        .unwrap_or(false)
}

/// 读取认证 Cookie 的 SameSite 属性，对齐 Go `CookieSameSiteMode()`。
///
/// 环境变量 `COOKIE_SAME_SITE` 取值 `none`/`strict`，其余（含空串、非法值）回退 Lax。
pub fn cookie_same_site() -> SameSite {
    match std::env::var("COOKIE_SAME_SITE")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "none" => SameSite::None,
        "strict" => SameSite::Strict,
        _ => SameSite::Lax,
    }
}

fn insert_header(response: &mut Response, name: &HeaderName, value: &'static str) {
    // from_static 在编译期校验字面量，不会失败；直接取值即可。
    let v = HeaderValue::from_static(value);
    response.headers_mut().insert(name.clone(), v);
}

/// 中间件：注入全部安全响应头。
pub async fn security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    insert_header(&mut response, &X_CONTENT_TYPE_OPTIONS, "nosniff");
    insert_header(&mut response, &X_FRAME_OPTIONS, "DENY");
    insert_header(&mut response, &X_XSS_PROTECTION, "1; mode=block");
    insert_header(
        &mut response,
        &REFERRER_POLICY,
        "strict-origin-when-cross-origin",
    );
    insert_header(
        &mut response,
        &PERMISSIONS_POLICY,
        "geolocation=(), microphone=(), camera=()",
    );

    if is_production() {
        // 生产环境强制 HTTPS 并预告一年（含子域）。
        insert_header(
            &mut response,
            &STRICT_TRANSPORT_SECURITY,
            "max-age=31536000; includeSubDomains",
        );
        insert_header(
            &mut response,
            &CONTENT_SECURITY_POLICY,
            "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'",
        );
    } else {
        // 开发环境允许内联脚本/样式与 CDN，便于前端调试。
        insert_header(
            &mut response,
            &CONTENT_SECURITY_POLICY,
            "default-src 'self'; script-src 'self' 'unsafe-inline' https://cdnjs.cloudflare.com; style-src 'self' 'unsafe-inline' https://cdnjs.cloudflare.com; img-src 'self' data:; font-src 'self'; connect-src 'self'",
        );
    }

    insert_header(&mut response, &SERVER, "Metaclouds");

    response
}
