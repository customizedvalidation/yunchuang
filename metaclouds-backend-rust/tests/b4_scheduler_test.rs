//! WP-P2-B4 调度器集成域集成测试。
//!
//! Bearer 通道跳过 CSRF。读需 `scheduler:read`，写（含 test-connection/sync）需 `scheduler:write`；
//! admin 短路放行，plainuser 仅有读权限（写 403）。

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
use metaclouds_backend_rust::handlers::scheduler::{
    create_scheduler, delete_scheduler, get_scheduler, list_schedulers, sync_resources,
    test_connection, update_scheduler,
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
        jwt_secret: "b4-scheduler-secret-at-least-32-characters-long-xxxx".to_string(),
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
        .route("/schedulers", get(list_schedulers))
        .route("/schedulers/{id}", get(get_scheduler))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::SCHEDULER_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/schedulers", post(create_scheduler))
        .route(
            "/schedulers/{id}",
            put(update_scheduler).delete(delete_scheduler),
        )
        .route("/schedulers/{id}/test-connection", post(test_connection))
        .route("/schedulers/{id}/sync", post(sync_resources))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::SCHEDULER_WRITE.to_string(),
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

async fn login_token(app: &mut Router, u: &str, p: &str) -> String {
    let (s, b) = do_req(
        app,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({"username": u, "password": p})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "login failed {u}");
    b["data"]["token"].as_str().unwrap().to_string()
}
async fn admin_token(app: &mut Router) -> String {
    login_token(app, "admin", "Admin@123456").await
}

#[tokio::test]
async fn b4_scheduler_list_requires_auth() {
    let (mut app, _p) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/schedulers", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b4_scheduler_create_get_update_delete() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/schedulers",
        Some(&token),
        Some(json!({
            "name": "slurm-main",
            "scheduler_type": "slurm",
            "endpoint": "http://slurm:6820",
            "auth_type": "token",
            "cluster_id": 1,
            "version": "23.02"
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], json!("slurm-main"));
    assert_eq!(body["data"]["scheduler_type"], json!("slurm"));
    assert_eq!(body["data"]["status"], json!("disconnected"));
    // 响应不回吐敏感 credentials。
    assert!(body["data"].get("credentials").is_none());
    let id = body["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/schedulers/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(id));

    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/schedulers/{id}"),
        Some(&token),
        Some(json!({"version": "24.05"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["version"], json!("24.05"));

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/schedulers/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn b4_scheduler_test_connection_and_sync() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/schedulers",
        Some(&token),
        Some(json!({"name": "k8s-1", "scheduler_type": "kubernetes", "cluster_id": 1})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/schedulers/{id}/test-connection"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], json!("connected"));
    assert!(body["data"]["latency_ms"].as_i64().unwrap() > 0);

    let (status, body) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/schedulers/{id}/sync"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["total_gpus"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn b4_scheduler_list_filter_and_pagination() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    for (n, t) in [("a", "kubernetes"), ("b", "slurm"), ("c", "custom")] {
        do_req(
            &mut app,
            "POST",
            "/api/v1/schedulers",
            Some(&token),
            Some(json!({"name": n, "scheduler_type": t, "cluster_id": 1})),
        )
        .await;
    }
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/schedulers?scheduler_type=slurm",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"].as_i64().unwrap(), 1);

    let (_, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/schedulers?page=1&page_size=2",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body["data"]["total"].as_i64().unwrap(), 3);
    assert!(body["data"]["data"].as_array().unwrap().len() <= 2);
}

#[tokio::test]
async fn b4_scheduler_plain_user_forbidden_on_write() {
    let (mut app, _p) = setup_app().await;
    let utoken = login_token(&mut app, "plainuser", "user-pass-123456").await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/schedulers",
        Some(&utoken),
        Some(json!({"name": "x"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], json!("FORBIDDEN"));
}
