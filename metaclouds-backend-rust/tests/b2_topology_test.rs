//! WP-P2-B2 节点拓扑集成测试（对齐 Go `topology_controller.go` 用例面）。
//!
//! Bearer 通道跳过 CSRF。读需 `topology:read`，写需 `topology:write`；admin 放行，plainuser 403。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::routing::{get, post, put};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_cookies::CookieManagerLayer;

use metaclouds_backend_rust::auth::csrf::csrf_protect;
use metaclouds_backend_rust::auth::handler::login;
use metaclouds_backend_rust::auth::middleware::{
    jwt_auth, permissions, require_permission, AppState,
};
use metaclouds_backend_rust::handlers::topology::{
    create_node, delete_node, get_node, list_nodes, update_node,
};

async fn setup_app() -> (Router, sqlx::SqlitePool) {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    let config = metaclouds_backend_rust::TestConfig {
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: "b2-topology-secret-at-least-32-characters-long".to_string(),
        jwt_expires_secs: 3600,
        server_port: 0,
        server_host: "127.0.0.1".to_string(),
        log_level: "warn".to_string(),
    };
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .expect("seed admin");
    let hash = metaclouds_backend_rust::hash_password("user-pass-123456").unwrap();
    let now = chrono::Utc::now();
    sqlx::query("INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) VALUES (?1, ?2, 'plainuser', 'plain@example.com', ?3, 'user', 1)")
        .bind(now).bind(now).bind(hash).execute(&pool).await.unwrap();

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.into()),
    };
    let read = Router::new()
        .route("/topology/nodes", get(list_nodes))
        .route("/topology/nodes/{id}", get(get_node))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::TOPOLOGY_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/topology/nodes", post(create_node))
        .route("/topology/nodes/{id}", put(update_node).delete(delete_node))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::TOPOLOGY_WRITE.to_string(),
            require_permission,
        ));
    let protected = read
        .merge(write)
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth,
        ));
    let public = Router::new().route("/auth/login", post(login));
    let app = Router::new()
        .nest("/api/v1", public.merge(protected))
        .layer(axum::middleware::from_fn(csrf_protect))
        .layer(CookieManagerLayer::new())
        .with_state(state);
    (app, pool)
}

async fn do_req(
    app: &mut Router,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => builder.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, v)
}

async fn admin_token(app: &mut Router) -> String {
    let (s, b) = do_req(
        app,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({"username": "admin", "password": "Admin@123456"})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    b["data"]["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn b2_topology_list_requires_auth() {
    let (mut app, _p) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/topology/nodes", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b2_topology_create_list_filter() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(&mut app, "POST", "/api/v1/topology/nodes", Some(&token),
        Some(json!({"hostname": "gpu-node-1", "ip": "10.0.0.1", "cluster_id": 1, "role": "worker", "cpu_cores": 64, "memory_gb": 512, "gpu_count": 8, "gpu_model": "A100", "labels": {"pool": "gpu"}}))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["hostname"], json!("gpu-node-1"));
    assert_eq!(body["data"]["role"], json!("worker"));
    assert_eq!(body["data"]["labels"]["pool"], json!("gpu"));
    assert!(body["data"].get("deleted_at").is_none());

    do_req(
        &mut app,
        "POST",
        "/api/v1/topology/nodes",
        Some(&token),
        Some(json!({"hostname": "master-1", "cluster_id": 1, "role": "master"})),
    )
    .await;

    // 列表
    let (_, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/topology/nodes",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body["data"]["total"].as_i64().unwrap(), 2);

    // 按 role 过滤
    let (_, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/topology/nodes?role=master",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body["data"]["total"].as_i64().unwrap(), 1);
    assert_eq!(body["data"]["data"][0]["role"], json!("master"));

    // 按 cluster_id 过滤
    let (_, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/topology/nodes?cluster_id=1",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body["data"]["total"].as_i64().unwrap(), 2);
}

#[tokio::test]
async fn b2_topology_update_delete() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/topology/nodes",
        Some(&token),
        Some(json!({"hostname": "upd-node", "role": "worker"})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/topology/nodes/{id}"),
        Some(&token),
        Some(json!({"status": "ready", "gpu_count": 4})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], json!("ready"));
    assert_eq!(body["data"]["gpu_count"], json!(4));

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/topology/nodes/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/topology/nodes/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
