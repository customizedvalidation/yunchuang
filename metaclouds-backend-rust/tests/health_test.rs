//! A1: `/health` 端点集成测试。
//!
//! 根级 `GET /health`，无 `/api/v1` 前缀、无需认证。
//! 验证 200 + 最小信封结构 {success,data:{status,version,uptime}}。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

use metaclouds_backend_rust::build_app;

async fn setup_app() -> axum::Router {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .expect("seed admin");

    let config = metaclouds_backend_rust::TestConfig::default();
    build_app(pool, config)
}

#[tokio::test]
async fn health_is_public_and_returns_envelope() {
    let app = setup_app().await;
    let req = Request::builder()
        .method("GET")
        .uri("/health")
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "/health 应 200");

    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();

    assert_eq!(v["success"], serde_json::json!(true));
    assert_eq!(v["data"]["status"], serde_json::json!("ok"));
    // version 为 cargo 版本字符串，非空。
    assert!(
        v["data"]["version"]
            .as_str()
            .map(|s| !s.is_empty())
            .unwrap_or(false),
        "version 应为非空字符串"
    );
    // uptime 为非负整数秒。
    assert!(
        v["data"]["uptime"].as_i64().unwrap_or(-1) >= 0,
        "uptime 应为非负整数"
    );
}

#[tokio::test]
async fn health_does_not_require_auth() {
    let app = setup_app().await;
    // 不带任何 Authorization / Cookie，仍应 200（不是 401）。
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "/health 不应要求认证"
    );
    assert_eq!(resp.status(), StatusCode::OK);
}
