//! 2026-09-21 P1 端点对齐回归测试。
//!
//! 验证前端实际调用但 Rust 此前缺失/路径不一致的端点已正确注册且不再 404：
//!   - GET  /monitoring/alerts           （Vue3 别名，对齐 /alerts）
//!   - GET  /resources/gpu               （GPU 资源汇总）
//!   - POST /jobs/:id/submit             （提交 K8S，mock）
//!   - GET  /schedulers/:id/queues       （mock）
//!   - GET  /schedulers/:id/nodes        （mock）
//!   - GET  /schedulers/:id/health      （mock）
//!   - POST /topology/score              （拓扑打分，mock）
//!   - DELETE /partitions/permissions/:id（Go 风格兼容别名）
//!
//! Bearer 通道跳过 CSRF；统一挂 jwt_auth（admin 短路放行权限校验），
//! 本测试聚焦"路由已注册且 handler 可运行"，不重复 RBAC 用例面。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::routing::{delete, get, post};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_cookies::CookieManagerLayer;

use metaclouds_backend_rust::auth::csrf::csrf_protect;
use metaclouds_backend_rust::auth::handler::login;
use metaclouds_backend_rust::auth::middleware::{jwt_auth, AppState};
use metaclouds_backend_rust::handlers::job::{create_job, submit_job_to_k8s};
use metaclouds_backend_rust::handlers::monitoring::list_monitoring_alerts;
use metaclouds_backend_rust::handlers::partition::{
    create_partition, grant_permission, revoke_permission_by_id,
};
use metaclouds_backend_rust::handlers::resource::list_gpu_resources;
use metaclouds_backend_rust::handlers::scheduler::{
    list_scheduler_nodes, list_scheduler_queues, scheduler_health,
};
use metaclouds_backend_rust::handlers::topology::calculate_topology_score;

async fn setup_app() -> (Router, sqlx::SqlitePool) {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let config = metaclouds_backend_rust::TestConfig {
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: "p1-align-secret-at-least-32-characters-long-xxxxx".to_string(),
        jwt_expires_secs: 3600,
        server_port: 0,
        server_host: "127.0.0.1".to_string(),
        log_level: "warn".to_string(),
    };
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .expect("seed admin");

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.into()),
    };

    // 只读/mock 路由（JWT 即可）。
    let read = Router::new()
        .route("/monitoring/alerts", get(list_monitoring_alerts))
        .route("/resources/gpu", get(list_gpu_resources))
        .route("/schedulers/{id}/queues", get(list_scheduler_queues))
        .route("/schedulers/{id}/nodes", get(list_scheduler_nodes))
        .route("/schedulers/{id}/health", get(scheduler_health));

    // 写路由（JWT；admin 短路放行权限校验）。
    let write = Router::new()
        .route("/jobs", post(create_job))
        .route("/jobs/{id}/submit", post(submit_job_to_k8s))
        .route("/topology/score", post(calculate_topology_score))
        .route("/partitions", post(create_partition))
        .route(
            "/partitions/{id}/permissions",
            post(grant_permission),
        )
        .route("/partitions/permissions/{id}", delete(revoke_permission_by_id));

    let protected = read
        .merge(write)
        .route_layer(axum::middleware::from_fn_with_state(state.clone(), jwt_auth));
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
    let (status, body) = do_req(
        app,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({"username": "admin", "password": "Admin@123456"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "admin login failed");
    body["data"]["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn p1_monitoring_alerts_alias_returns_200() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) =
        do_req(&mut app, "GET", "/api/v1/monitoring/alerts", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].is_array());
}

#[tokio::test]
async fn p1_resources_gpu_returns_200() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) =
        do_req(&mut app, "GET", "/api/v1/resources/gpu", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].is_array());
}

#[tokio::test]
async fn p1_scheduler_queues_nodes_health_mock() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;

    let (status, body) =
        do_req(&mut app, "GET", "/api/v1/schedulers/1/queues", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].is_array());

    let (status, body) =
        do_req(&mut app, "GET", "/api/v1/schedulers/1/nodes", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].is_array());

    let (status, body) =
        do_req(&mut app, "GET", "/api/v1/schedulers/1/health", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["healthy"], json!(true));
}

#[tokio::test]
async fn p1_topology_score_mock() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/topology/score",
        Some(&token),
        Some(json!({"job_id": 1, "candidate_nodes": [{"id": 10}, {"id": 20}]})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["10"], json!(100.0));
    assert_eq!(body["data"]["20"], json!(90.0));
}

#[tokio::test]
async fn p1_job_submit_route_registered() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    // 先创建作业，再提交到 K8S（mock），断言 200 证明路由已注册。
    let (status, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/jobs",
        Some(&token),
        Some(json!({"name": "submitme"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let job_id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/jobs/{job_id}/submit"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["job_id"], json!(job_id));
}

#[tokio::test]
async fn p1_partition_permission_go_style_delete() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    // 创建分区 + 授权。
    let (_, partition) = do_req(
        &mut app,
        "POST",
        "/api/v1/partitions",
        Some(&token),
        Some(json!({"cluster_id": 1, "name": "perm-alias"})),
    )
    .await;
    let partition_id = partition["data"]["id"].as_i64().unwrap();

    let (_, granted) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/partitions/{partition_id}/permissions"),
        Some(&token),
        Some(json!({"user_id": 2, "tenant_id": 1, "permission_type": "read"})),
    )
    .await;
    let perm_id = granted["data"]["id"].as_i64().unwrap();

    // Go 风格扁平路径 DELETE /partitions/permissions/:id。
    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/partitions/permissions/{perm_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}
