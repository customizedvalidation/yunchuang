//! B6 安全中间件边界测试：
//! - security_headers 响应头完整性
//! - CSRF 双提交令牌边界（Bearer 跳过 / Cookie 无令牌 403 / 正确令牌通过 / 错误令牌 403 / GET 不校验）
//! - panic 恢复中间件（返回 500 而非进程崩溃）
//! - request_id 中间件（存在性 / 唯一性 / 透传）
//! - 未认证 401

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_cookies::CookieManagerLayer;

use metaclouds_backend_rust::auth::csrf::csrf_protect;
use metaclouds_backend_rust::auth::handler::login;
use metaclouds_backend_rust::auth::middleware::{jwt_auth, AppState};
use metaclouds_backend_rust::middleware::apply_core_stack;

// ---------------------------------------------------------------------------
// 帮助函数
// ---------------------------------------------------------------------------

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, v)
}

fn req_builder(method: &str, uri: &str) -> axum::http::request::Builder {
    Request::builder().method(method).uri(uri)
}

fn header_str<'a>(resp: &'a axum::response::Response, name: &str) -> Option<&'a str> {
    resp.headers().get(name).and_then(|v| v.to_str().ok())
}

// ---------------------------------------------------------------------------
// 路由器装配
// ---------------------------------------------------------------------------

/// 仅 CSRF + CookieManager 的路由（不含 JWT，隔离测试 CSRF 逻辑）。
fn csrf_only_app() -> Router {
    let router = Router::new()
        .route("/api/v1/action", post(|| async { "ok" }))
        .route("/api/v1/view", get(|| async { "ok" }));
    router
        .layer(axum::middleware::from_fn(csrf_protect))
        .layer(CookieManagerLayer::new())
}

/// 核心中间件栈路由（含 panic 恢复、request_id、security_headers）。
fn core_stack_app() -> Router {
    async fn panic_handler() -> &'static str {
        panic!("intentional test panic");
    }
    let router = Router::new()
        .route("/api/v1/ping", get(|| async { "pong" }))
        .route("/api/v1/panic", get(panic_handler));
    apply_core_stack(router)
}

/// 带 JWT 认证的受保护路由（用于 401 测试）。
async fn auth_app() -> Router {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let config = metaclouds_backend_rust::TestConfig {
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: "sec-mw-secret-at-least-32-chars-long-xxxxx".to_string(),
        jwt_expires_secs: 3600,
        server_port: 0,
        server_host: "127.0.0.1".to_string(),
        log_level: "warn".to_string(),
    };
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .expect("seed admin");

    let state = AppState {
        pool,
        config: Arc::new(config.into()),
    };

    let protected = Router::new()
        .route("/api/v1/protected", get(|| async { "secret" }))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth,
        ));

    Router::new()
        .route("/api/v1/auth/login", post(login))
        .merge(protected)
        .layer(axum::middleware::from_fn(csrf_protect))
        .layer(CookieManagerLayer::new())
        .with_state(state)
}

// ---------------------------------------------------------------------------
// 1. security_headers 响应头完整性
// ---------------------------------------------------------------------------

#[tokio::test]
async fn secmw_security_headers_all_present() {
    let app = core_stack_app();
    let req = req_builder("GET", "/api/v1/ping")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();

    assert_eq!(header_str(&resp, "x-content-type-options"), Some("nosniff"));
    assert_eq!(header_str(&resp, "x-frame-options"), Some("DENY"));
    assert_eq!(header_str(&resp, "x-xss-protection"), Some("1; mode=block"));
    assert_eq!(
        header_str(&resp, "referrer-policy"),
        Some("strict-origin-when-cross-origin")
    );
    assert_eq!(
        header_str(&resp, "permissions-policy"),
        Some("geolocation=(), microphone=(), camera=()")
    );
    assert_eq!(header_str(&resp, "server"), Some("Metaclouds"));
    // CSP 在开发模式必须存在。
    let csp = header_str(&resp, "content-security-policy").expect("CSP header");
    assert!(csp.contains("default-src 'self'"), "CSP: {csp}");
    assert!(
        csp.contains("unsafe-inline"),
        "dev CSP allows inline: {csp}"
    );
}

// ---------------------------------------------------------------------------
// 2. CSRF 双提交令牌边界
// ---------------------------------------------------------------------------

#[tokio::test]
async fn secmw_csrf_bearer_token_skips_validation() {
    // 无 access_token Cookie → 视为 Bearer 客户端，POST 直接放行。
    let app = csrf_only_app();
    let req = req_builder("POST", "/api/v1/action")
        .header(header::AUTHORIZATION, "Bearer some-token")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn secmw_csrf_cookie_without_token_rejected() {
    // 有 access_token Cookie 但无 X-CSRF-Token 头 → 403。
    let app = csrf_only_app();
    let req = req_builder("POST", "/api/v1/action")
        .header(header::COOKIE, "access_token=fake-jwt")
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], json!(false));
}

#[tokio::test]
async fn secmw_csrf_cookie_with_matching_token_passes() {
    // access_token Cookie + 匹配的 X-CSRF-Token → 200。
    let app = csrf_only_app();
    let req = req_builder("POST", "/api/v1/action")
        .header(header::COOKIE, "access_token=fake-jwt; csrf_token=abc123")
        .header("x-csrf-token", "abc123")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn secmw_csrf_cookie_with_wrong_token_rejected() {
    // access_token Cookie + 不匹配的 X-CSRF-Token → 403 mismatch。
    let app = csrf_only_app();
    let req = req_builder("POST", "/api/v1/action")
        .header(header::COOKIE, "access_token=fake-jwt; csrf_token=abc123")
        .header("x-csrf-token", "def456")
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], json!(false));
}

#[tokio::test]
async fn secmw_csrf_get_request_not_protected() {
    // GET 即使有 access_token Cookie 也不校验 CSRF。
    let app = csrf_only_app();
    let req = req_builder("GET", "/api/v1/view")
        .header(header::COOKIE, "access_token=fake-jwt")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// 3. panic 恢复中间件
// ---------------------------------------------------------------------------

#[tokio::test]
async fn secmw_panic_returns_500_not_crash() {
    let app = core_stack_app();
    let req = req_builder("GET", "/api/v1/panic")
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body["success"], json!(false));

    // panic 后后续请求仍正常（进程未崩溃）。
    let req2 = req_builder("GET", "/api/v1/ping")
        .body(Body::empty())
        .unwrap();
    let (status2, _) = send(&app, req2).await;
    assert_eq!(status2, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// 4. request_id 中间件
// ---------------------------------------------------------------------------

#[tokio::test]
async fn secmw_request_id_each_response_has_header() {
    let app = core_stack_app();
    let req = req_builder("GET", "/api/v1/ping")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert!(
        header_str(&resp, "x-request-id").is_some(),
        "every response must carry X-Request-Id"
    );
}

#[tokio::test]
async fn secmw_request_id_unique_per_request() {
    let app = core_stack_app();

    let req1 = req_builder("GET", "/api/v1/ping")
        .body(Body::empty())
        .unwrap();
    let resp1 = app.clone().oneshot(req1).await.unwrap();
    let id1 = header_str(&resp1, "x-request-id").unwrap().to_string();

    let req2 = req_builder("GET", "/api/v1/ping")
        .body(Body::empty())
        .unwrap();
    let resp2 = app.clone().oneshot(req2).await.unwrap();
    let id2 = header_str(&resp2, "x-request-id").unwrap().to_string();

    assert_ne!(id1, id2, "each request must get a unique request id");
}

#[tokio::test]
async fn secmw_request_id_passed_through_when_client_sends() {
    let app = core_stack_app();
    let req = req_builder("GET", "/api/v1/ping")
        .header("x-request-id", "client-trace-xyz-789")
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(
        header_str(&resp, "x-request-id"),
        Some("client-trace-xyz-789"),
        "client-supplied request id must be echoed back"
    );
}

// ---------------------------------------------------------------------------
// 5. 未认证 401
// ---------------------------------------------------------------------------

#[tokio::test]
async fn secmw_protected_without_token_returns_401() {
    let app = auth_app().await;
    let req = req_builder("GET", "/api/v1/protected")
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["success"], json!(false));
}
