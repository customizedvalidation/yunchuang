//! WP-P2-B3 GPU 域集成测试（对齐 Go `gpu_controller_test.go` 用例面）。
//!
//! Bearer 通道跳过 CSRF。读需 `gpu:read`，写需 `gpu:write`；
//! admin 短路放行，plainuser 无 GPU 写权限（403）。
//! 覆盖：设备 CRUD / allocate·release（联动设备显存与状态）/ allocations 列表 / 401 / 403。

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
use metaclouds_backend_rust::handlers::gpu::{
    allocate_gpu, create_gpu_device, delete_gpu_device, get_gpu_device, get_gpu_utilization,
    list_allocations, list_gpu_devices, release_gpu, update_gpu_device,
};

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
        jwt_secret: "b3-gpu-secret-at-least-32-characters-long-xxxx".to_string(),
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
    sqlx::query(
        "INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) \
         VALUES (?1, ?2, 'plainuser', 'plain@example.com', ?3, 'user', 1)",
    )
    .bind(now).bind(now).bind(hash).execute(&pool).await.unwrap();

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.into()),
    };

    // 读路由：静态 /gpus/utilization、/gpus/allocations 先于动态 /gpus/{id}。
    let read = Router::new()
        .route("/gpus", get(list_gpu_devices))
        .route("/gpus/utilization", get(get_gpu_utilization))
        .route("/gpus/allocations", get(list_allocations))
        .route("/gpus/{id}", get(get_gpu_device))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::GPU_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/gpus", post(create_gpu_device))
        .route(
            "/gpus/{id}",
            put(update_gpu_device).delete(delete_gpu_device),
        )
        .route("/gpus/allocations", post(allocate_gpu))
        .route("/gpus/allocations/{id}", axum::routing::delete(release_gpu))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::GPU_WRITE.to_string(),
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

async fn login_token(app: &mut Router, username: &str, password: &str) -> String {
    let (status, body) = do_req(
        app,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({"username": username, "password": password})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "login failed for {username}");
    body["data"]["token"].as_str().unwrap().to_string()
}

async fn admin_token(app: &mut Router) -> String {
    login_token(app, "admin", "Admin@123456").await
}

async fn create_device_raw(app: &mut Router, token: &str, name: &str) -> Value {
    let (status, body) = do_req(
        app,
        "POST",
        "/api/v1/gpus",
        Some(token),
        Some(json!({
            "node_name": name,
            "vendor": "nvidia",
            "model": "A100",
            "total_memory_gb": 80,
            "allocatable_memory_gb": 80
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create device failed: {body}");
    body
}

#[tokio::test]
async fn b3_gpu_list_requires_auth() {
    let (mut app, _p) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/gpus", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b3_gpu_create_and_get_detail() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let body = create_device_raw(&mut app, &token, "node-0").await;
    assert_eq!(body["data"]["vendor"], json!("nvidia"));
    assert_eq!(body["data"]["model"], json!("A100"));
    assert_eq!(body["data"]["status"], json!("available"));
    assert_eq!(body["data"]["total_memory_gb"], json!(80));
    assert!(body["data"].get("deleted_at").is_none());
    // JSON 字段名为 "index"（对齐 Go）
    assert!(body["data"].get("index").is_some());
    let id = body["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/gpus/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(id));
    assert_eq!(body["data"]["node_name"], json!("node-0"));
}

#[tokio::test]
async fn b3_gpu_list_filter_by_vendor_and_status() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    create_device_raw(&mut app, &token, "n-node").await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/gpus",
        Some(&token),
        Some(json!({"node_name": "amd-node", "vendor": "amd", "model": "MI300", "total_memory_gb": 192})),
    )
    .await;

    // 按 vendor 过滤
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/gpus?vendor=amd",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["vendor"], json!("amd"));

    // 按 status 过滤（新建全为 available）
    let (_, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/gpus?status=available",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body["data"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn b3_gpu_allocate_updates_device_status_then_release() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let dev = create_device_raw(&mut app, &token, "alloc-node").await;
    let dev_id = dev["data"]["id"].as_i64().unwrap();

    // 分配 40GB
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/gpus/allocations",
        Some(&token),
        Some(json!({"job_id": 1, "tenant_id": 1, "user_id": 1, "fraction": 0.5, "memory_gb": 40, "vendor": "nvidia"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "allocate failed: {body}");
    assert_eq!(body["data"]["status"], json!("active"));
    assert_eq!(body["data"]["memory_gb"], json!(40));
    let alloc_id = body["data"]["id"].as_i64().unwrap();

    // 设备状态应变为 allocated，used_memory_gb=40
    let (_, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/gpus/{dev_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body["data"]["status"], json!("allocated"));
    assert_eq!(body["data"]["used_memory_gb"], json!(40));

    // 释放
    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/gpus/allocations/{alloc_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 设备应恢复 available，used=0
    let (_, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/gpus/{dev_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body["data"]["status"], json!("available"));
    assert_eq!(body["data"]["used_memory_gb"], json!(0));
}

#[tokio::test]
async fn b3_gpu_allocations_list_filter_by_job() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    create_device_raw(&mut app, &token, "list-node").await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/gpus/allocations",
        Some(&token),
        Some(json!({"job_id": 10, "tenant_id": 1, "user_id": 1, "fraction": 1.0, "memory_gb": 10, "vendor": "nvidia"})),
    )
    .await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/gpus/allocations",
        Some(&token),
        Some(json!({"job_id": 20, "tenant_id": 1, "user_id": 1, "fraction": 1.0, "memory_gb": 10, "vendor": "nvidia"})),
    )
    .await;

    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/gpus/allocations?job_id=10",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["job_id"], json!(10));
}

#[tokio::test]
async fn b3_gpu_release_non_active_rejected() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    create_device_raw(&mut app, &token, "rel-node").await;
    let (_, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/gpus/allocations",
        Some(&token),
        Some(json!({"job_id": 1, "tenant_id": 1, "user_id": 1, "fraction": 1.0, "memory_gb": 10, "vendor": "nvidia"})),
    )
    .await;
    let alloc_id = body["data"]["id"].as_i64().unwrap();

    // 第一次释放成功
    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/gpus/allocations/{alloc_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    // 第二次释放应 400（not active）
    let (status, body) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/gpus/allocations/{alloc_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], json!("BAD_REQUEST"));
}

#[tokio::test]
async fn b3_gpu_delete_then_404() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let dev = create_device_raw(&mut app, &token, "del-node").await;
    let id = dev["data"]["id"].as_i64().unwrap();

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/gpus/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/gpus/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn b3_gpu_plain_user_cannot_write_forbidden() {
    let (mut app, _p) = setup_app().await;
    let utoken = login_token(&mut app, "plainuser", "user-pass-123456").await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/gpus",
        Some(&utoken),
        Some(json!({"vendor": "nvidia", "model": "A100"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], json!("FORBIDDEN"));
}
