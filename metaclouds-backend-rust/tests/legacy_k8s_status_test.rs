//! A4: K8s 只读路由集成测试 —— `GET /api/v1/clusters/:id/status`。
//!
//! 对照 Go：clusters 组仅暴露 `/:id/status`（k8sController.GetClusterStatus）。
//! nodes/pods/health Go 侧挂在 /k8s 别名下（Rust 已有），不在此重复。

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
use metaclouds_backend_rust::handlers::k8s::cluster_status;

async fn setup_app() -> axum::Router {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .unwrap();
    let config = metaclouds_backend_rust::TestConfig::default();
    let state = AppState {
        pool,
        config: Arc::new(config.into()),
    };
    let protected = Router::new()
        .route("/clusters/{id}/status", get(cluster_status))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth,
        ));
    let public = Router::new().route("/auth/login", post(login));
    Router::new()
        .nest("/api/v1", public.merge(protected))
        .layer(axum::middleware::from_fn(csrf_protect))
        .layer(CookieManagerLayer::new())
        .with_state(state)
}

async fn do_req(
    app: &mut Router,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = builder.body(Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn cluster_status_requires_auth() {
    let mut app = setup_app().await;
    let (s, _) = do_req(&mut app, "GET", "/api/v1/clusters/1/status", None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn cluster_status_returns_gpu_usage_shape() {
    let mut app = setup_app().await;
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"username":"admin","password":"Admin@123456"}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let login: Value = serde_json::from_slice(&bytes).unwrap();
    let token = login["data"]["token"].as_str().unwrap().to_string();

    let (s, b) = do_req(&mut app, "GET", "/api/v1/clusters/1/status", Some(&token)).await;
    assert_eq!(s, StatusCode::OK);
    let d = &b["data"];
    // 对齐 Go ClusterStatus 字段。
    for key in [
        "id",
        "name",
        "status",
        "nodes",
        "gpus_total",
        "gpus_used",
        "gpus_free",
        "cpu_usage",
        "memory_usage",
    ] {
        assert!(d.get(key).is_some(), "status 应含字段 {key}");
    }
    assert_eq!(d["id"], json!(1));
}
