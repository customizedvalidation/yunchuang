//! WP-P2-B3 作业域集成测试（对齐 Go `job_controller_test.go` 用例面）。
//!
//! Bearer 通道跳过 CSRF。读需 `job:read`，写需 `job:write`；
//! admin 短路放行，plainuser 无作业写权限（403）。
//! 覆盖：CRUD / 状态机（pending→running→completed）/ cancel / stats / 分页 / 过滤 / 401 / 403 / 404。

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
use metaclouds_backend_rust::handlers::job::{
    cancel_job, create_job, delete_job, get_job, get_job_stats, list_jobs, update_job,
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
        jwt_secret: "b3-job-secret-at-least-32-characters-longxxxx".to_string(),
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

    // 读路由：静态 /jobs/stats 先于动态 /jobs/{id} 注册（axum 静态优先，双保险）。
    let read = Router::new()
        .route("/jobs/stats", get(get_job_stats))
        .route("/jobs", get(list_jobs))
        .route("/jobs/{id}", get(get_job))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::JOB_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/jobs", post(create_job))
        .route("/jobs/{id}", put(update_job).delete(delete_job))
        .route("/jobs/{id}/cancel", post(cancel_job))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::JOB_WRITE.to_string(),
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

async fn create_job_raw(app: &mut Router, token: &str, name: &str) -> Value {
    let (status, body) = do_req(
        app,
        "POST",
        "/api/v1/jobs",
        Some(token),
        Some(json!({"name": name, "type": "training", "gpus": 2, "cpus": 8, "memory": 32})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "create job failed for {name}: {body}"
    );
    body
}

#[tokio::test]
async fn b3_job_list_requires_auth() {
    let (mut app, _p) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/jobs", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b3_job_create_and_get_detail() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let body = create_job_raw(&mut app, &token, "train-resnet").await;
    assert_eq!(body["data"]["name"], json!("train-resnet"));
    assert_eq!(body["data"]["type"], json!("training"));
    assert_eq!(body["data"]["status"], json!("pending"));
    assert_eq!(body["data"]["gpus"], json!(2));
    assert!(body["data"].get("deleted_at").is_none());
    let id = body["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/jobs/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(id));
    assert_eq!(body["data"]["name"], json!("train-resnet"));
}

#[tokio::test]
async fn b3_job_list_pagination_and_filter() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    for i in 0..3 {
        create_job_raw(&mut app, &token, &format!("job-{i}")).await;
    }
    do_req(
        &mut app,
        "POST",
        "/api/v1/jobs",
        Some(&token),
        Some(json!({"name": "infer-1", "type": "inference", "gpus": 1})),
    )
    .await;

    // 分页信封
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/jobs?page=1&page_size=2",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["page"], json!(1));
    assert_eq!(body["data"]["page_size"], json!(2));
    assert_eq!(body["data"]["data"].as_array().unwrap().len(), 2);
    assert!(body["data"]["total"].as_i64().unwrap() >= 4);

    // 按 type 过滤
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/jobs?type=inference",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for r in body["data"]["data"].as_array().unwrap() {
        assert_eq!(r["type"], json!("inference"));
    }
    assert_eq!(body["data"]["total"].as_i64().unwrap(), 1);

    // 按 status 过滤（新建全为 pending）
    let (_, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/jobs?status=pending",
        Some(&token),
        None,
    )
    .await;
    assert!(body["data"]["total"].as_i64().unwrap() >= 4);
}

#[tokio::test]
async fn b3_job_status_machine_pending_running_completed() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let body = create_job_raw(&mut app, &token, "sm").await;
    let id = body["data"]["id"].as_i64().unwrap();

    // pending -> running
    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/jobs/{id}"),
        Some(&token),
        Some(json!({"status": "running"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], json!("running"));
    assert!(!body["data"]["start_time"].is_null());

    // running -> completed
    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/jobs/{id}"),
        Some(&token),
        Some(json!({"status": "completed"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], json!("completed"));
    assert!(!body["data"]["end_time"].is_null());
}

#[tokio::test]
async fn b3_job_terminal_state_rejects_transition() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let body = create_job_raw(&mut app, &token, "term").await;
    let id = body["data"]["id"].as_i64().unwrap();

    // 直接 pending -> completed（允许，非终态到终态）
    let (_, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/jobs/{id}"),
        Some(&token),
        Some(json!({"status": "completed"})),
    )
    .await;
    assert_eq!(body["data"]["status"], json!("completed"));

    // completed -> running 应被拒（终态不可流转）
    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/jobs/{id}"),
        Some(&token),
        Some(json!({"status": "running"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], json!("BAD_REQUEST"));
}

#[tokio::test]
async fn b3_job_cancel_pending_succeeds() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let body = create_job_raw(&mut app, &token, "canc").await;
    let id = body["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/jobs/{id}/cancel"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], json!("cancelled"));
}

#[tokio::test]
async fn b3_job_cancel_terminal_rejected() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let body = create_job_raw(&mut app, &token, "canc2").await;
    let id = body["data"]["id"].as_i64().unwrap();
    // 先置为 completed
    do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/jobs/{id}"),
        Some(&token),
        Some(json!({"status": "completed"})),
    )
    .await;

    let (status, body) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/jobs/{id}/cancel"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["code"], json!("BAD_REQUEST"));
}

#[tokio::test]
async fn b3_job_delete_then_404() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let body = create_job_raw(&mut app, &token, "del").await;
    let id = body["data"]["id"].as_i64().unwrap();

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/jobs/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/jobs/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn b3_job_get_nonexistent_404() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(&mut app, "GET", "/api/v1/jobs/99999", Some(&token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], json!("NOT_FOUND"));
}

#[tokio::test]
async fn b3_job_stats_counts_by_status() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let j1 = create_job_raw(&mut app, &token, "s1").await;
    let id1 = j1["data"]["id"].as_i64().unwrap();
    let _j2 = create_job_raw(&mut app, &token, "s2").await;

    // 把 j1 置为 running
    do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/jobs/{id1}"),
        Some(&token),
        Some(json!({"status": "running"})),
    )
    .await;

    let (status, body) = do_req(&mut app, "GET", "/api/v1/jobs/stats", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["pending"], json!(1));
    assert_eq!(body["data"]["running"], json!(1));
}

#[tokio::test]
async fn b3_job_plain_user_cannot_write_forbidden() {
    let (mut app, _p) = setup_app().await;
    let utoken = login_token(&mut app, "plainuser", "user-pass-123456").await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/jobs",
        Some(&utoken),
        Some(json!({"name": "nope", "type": "training"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], json!("FORBIDDEN"));
}

#[tokio::test]
async fn b3_job_plain_user_can_read_own_tenant() {
    let (mut app, _p) = setup_app().await;
    let atoken = admin_token(&mut app).await;
    // admin 在租户 1 下建一个作业（与 plainuser 同租户）
    create_job_raw(&mut app, &atoken, "shared").await;

    let utoken = login_token(&mut app, "plainuser", "user-pass-123456").await;
    // plainuser 有 job:read 权限，应能列出本租户作业
    let (status, body) = do_req(&mut app, "GET", "/api/v1/jobs", Some(&utoken), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["total"].as_i64().unwrap() >= 1);
}
