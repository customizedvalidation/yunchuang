//! 中间件栈集成测试：请求 ID、计时、安全头、统一错误信封。
//!
//! 直接用 `apply_core_stack` 组装一条最小路由，避免依赖完整业务路由。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use tower::ServiceExt;

use metaclouds_backend_rust::middleware::apply_core_stack;

/// 组装带核心中间件栈的最小应用。
fn app() -> Router {
    let router = Router::new()
        .route("/api/v1/ping", get(|| async { "pong" }))
        .route("/api/v1/echo", post(|| async { "ok" }));
    apply_core_stack(router)
}

#[tokio::test]
async fn core_stack_injects_request_id_and_timing() {
    let resp = app()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ping")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    // 请求 ID 被回写响应头。
    assert!(resp.headers().contains_key("x-request-id"));
    // 计时头为 "Xms" 形式。
    let rt = resp
        .headers()
        .get("x-response-time")
        .and_then(|v| v.to_str().ok())
        .unwrap();
    assert!(rt.ends_with("ms"), "unexpected X-Response-Time: {rt}");
}

#[tokio::test]
async fn security_headers_are_set() {
    let resp = app()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/ping")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let h = resp.headers();
    assert_eq!(h.get("x-content-type-options").unwrap(), "nosniff");
    assert_eq!(h.get("x-frame-options").unwrap(), "DENY");
    assert_eq!(h.get("x-xss-protection").unwrap(), "1; mode=block");
    assert_eq!(
        h.get("referrer-policy").unwrap(),
        "strict-origin-when-cross-origin"
    );
    assert_eq!(
        h.get("permissions-policy").unwrap(),
        "geolocation=(), microphone=(), camera=()"
    );
    assert_eq!(h.get("server").unwrap(), "Metaclouds");
    // CSP 在开发环境必须存在且允许 unsafe-inline。
    let csp = h
        .get("content-security-policy")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    assert!(csp.contains("default-src 'self'"), "CSP 缺省策略: {csp}");
}

#[tokio::test]
async fn unknown_route_returns_json_envelope() {
    let resp = app()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/does/not/exist")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    // 必须是 JSON 信封，而非 axum 默认纯文本。
    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap()
        .to_string();
    assert!(ct.contains("application/json"), "404 必须返回 JSON: {ct}");

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], serde_json::json!(false));
    assert_eq!(v["code"], serde_json::json!("NOT_FOUND"));
}

#[tokio::test]
async fn post_echo_passes_through_stack() {
    let resp = app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/echo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
