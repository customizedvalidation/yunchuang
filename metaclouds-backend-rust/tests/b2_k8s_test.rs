//! WP-P2-B2 K8s mock 只读接口测试。
//!
//! Bearer 通道跳过 CSRF。三个只读端点均需 JWT + `cluster:read`；
//! admin 放行，未认证 401。验证 mock 返回结构字段对齐 Go Pod / ClusterStatus。

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
use metaclouds_backend_rust::auth::middleware::{
    jwt_auth, permissions, require_permission, AppState,
};
use metaclouds_backend_rust::handlers::k8s::{cluster_health, list_nodes, list_pods};

async fn setup_app() -> Router {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let config = metaclouds_backend_rust::TestConfig {
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: "b2-k8s-secret-at-least-32-characters-long-xxxxxxxxx".to_string(),
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

    let read = Router::new()
        .route("/k8s/clusters/{id}/pods", get(list_pods))
        .route("/k8s/clusters/{id}/nodes", get(list_nodes))
        .route("/k8s/clusters/{id}/health", get(cluster_health))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::CLUSTER_READ.to_string(),
            require_permission,
        ));
    let protected = read.route_layer(axum::middleware::from_fn_with_state(
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
    let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, v)
}

#[tokio::test]
async fn b2_k8s_pods_shape_and_auth() {
    let mut app = setup_app().await;
    // 未认证 401
    let (status, _) = do_req(&mut app, "GET", "/api/v1/k8s/clusters/1/pods", None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    // 登录拿 admin token（inline）。
    let login_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"username": "admin", "password": "Admin@123456"}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(login_req).await.unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    let token = body["data"]["token"].as_str().unwrap().to_string();

    let (status, body) = do_req(&mut app, "GET", "/api/v1/k8s/clusters/1/pods", Some(&token)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], json!(true));
    let pods = body["data"].as_array().unwrap();
    assert!(!pods.is_empty());
    // 字段对齐 Go Pod：name / status / node / gpus。
    let p = &pods[0];
    assert!(p.get("name").is_some());
    assert!(p.get("status").is_some());
    assert!(p.get("node").is_some());
    assert!(p.get("gpus").is_some());
}

#[tokio::test]
async fn b2_k8s_health_and_nodes_shape() {
    let mut app = setup_app().await;
    let login_req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"username": "admin", "password": "Admin@123456"}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(login_req).await.unwrap();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let token = serde_json::from_slice::<Value>(&bytes).unwrap()["data"]["token"]
        .as_str()
        .unwrap()
        .to_string();

    // health 字段对齐 Go ClusterStatus。
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/k8s/clusters/7/health",
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let h = &body["data"];
    assert_eq!(h["id"], json!(7));
    assert!(h.get("gpus_total").is_some());
    assert!(h.get("gpus_used").is_some());
    assert!(h.get("gpus_free").is_some());
    assert!(h.get("cpu_usage").is_some());
    assert!(h.get("memory_usage").is_some());

    // nodes 列表。
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/k8s/clusters/7/nodes",
        Some(&token),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|n| n["status"] == json!("Ready")));
}
