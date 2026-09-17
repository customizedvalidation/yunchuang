//! WP-P2-B5 Acceleration Suite 集成测试。
//!
//! Suite CRUD + 组合查询 + start/stop + 404。

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
use metaclouds_backend_rust::handlers::acceleration::{
    create_suite, delete_suite, get_suite, list_suites, start_suite, stop_suite, update_suite,
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
        jwt_secret: "b5-accel-secret-at-least-32-characters-long-x".to_string(),
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
    .bind(now)
    .bind(now)
    .bind(hash)
    .execute(&pool)
    .await
    .unwrap();

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.into()),
    };

    let read = Router::new()
        .route("/acceleration/suites", get(list_suites))
        .route("/acceleration/suites/{id}", get(get_suite))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::ACCELERATION_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/acceleration/suites", post(create_suite))
        .route(
            "/acceleration/suites/{id}",
            put(update_suite).delete(delete_suite),
        )
        .route("/acceleration/suites/{id}/start", post(start_suite))
        .route("/acceleration/suites/{id}/stop", post(stop_suite))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::ACCELERATION_WRITE.to_string(),
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
    assert_eq!(status, StatusCode::OK);
    body["data"]["token"].as_str().unwrap().to_string()
}

async fn admin_token(app: &mut Router) -> String {
    login_token(app, "admin", "Admin@123456").await
}

#[tokio::test]
async fn b5_suite_list_requires_auth() {
    let (mut app, _pool) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/acceleration/suites", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b5_create_and_get_suite() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/acceleration/suites",
        Some(&token),
        Some(json!({"name": "train-suite-1", "description": "PyTorch training suite", "suite_type": "training"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], json!("train-suite-1"));
    assert_eq!(body["data"]["suite_type"], json!("training"));
    assert_eq!(body["data"]["status"], json!("active"));

    let id = body["data"]["id"].as_i64().unwrap();
    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/acceleration/suites/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(id));
}

#[tokio::test]
async fn b5_suite_pagination() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    for i in 0..3 {
        do_req(
            &mut app,
            "POST",
            "/api/v1/acceleration/suites",
            Some(&token),
            Some(json!({"name": format!("suite-{i}"), "suite_type": "training"})),
        )
        .await;
    }
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/acceleration/suites?page=1&page_size=2",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["page"], json!(1));
    assert!(body["data"]["data"].as_array().unwrap().len() <= 2);
}

#[tokio::test]
async fn b5_suite_start_stop() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/acceleration/suites",
        Some(&token),
        Some(json!({"name": "start-stop-suite", "suite_type": "inference"})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/acceleration/suites/{id}/start"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], json!("running"));

    let (status, body) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/acceleration/suites/{id}/stop"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], json!("stopped"));
}

#[tokio::test]
async fn b5_suite_start_not_found() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/acceleration/suites/99999/start",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], json!("NOT_FOUND"));
}

#[tokio::test]
async fn b5_suite_update_and_delete() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/acceleration/suites",
        Some(&token),
        Some(json!({"name": "upd-suite", "suite_type": "hybrid"})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/acceleration/suites/{id}"),
        Some(&token),
        Some(json!({"description": "updated", "status": "paused"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["description"], json!("updated"));
    assert_eq!(body["data"]["status"], json!("paused"));

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/acceleration/suites/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/acceleration/suites/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn b5_user_role_cannot_write_suite() {
    let (mut app, _pool) = setup_app().await;
    let utoken = login_token(&mut app, "plainuser", "user-pass-123456").await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/acceleration/suites",
        Some(&utoken),
        Some(json!({"name": "fail-suite", "suite_type": "training"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], json!("FORBIDDEN"));
}
