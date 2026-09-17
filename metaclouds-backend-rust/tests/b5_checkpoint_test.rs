//! WP-P2-B5 Checkpoint 集成测试。
//!
//! Checkpoint CRUD + 按 job_id 过滤。

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
use metaclouds_backend_rust::handlers::checkpoint::{
    create_checkpoint, delete_checkpoint, get_checkpoint, list_checkpoints, update_checkpoint,
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
        jwt_secret: "b5-checkpoint-secret-at-least-32-chars-long-x".to_string(),
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

    let read = Router::new()
        .route("/checkpoints", get(list_checkpoints))
        .route("/checkpoints/{id}", get(get_checkpoint))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::CHECKPOINT_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/checkpoints", post(create_checkpoint))
        .route(
            "/checkpoints/{id}",
            put(update_checkpoint).delete(delete_checkpoint),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::CHECKPOINT_WRITE.to_string(),
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
    let (status, body) = do_req(
        app,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({"username": "admin", "password": "Admin@123456"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    body["data"]["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn b5_create_and_get_checkpoint() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/checkpoints",
        Some(&token),
        Some(json!({"name": "ckpt-1", "path": "/ckpts/step-100.pt", "format": "pytorch", "step": 100, "epoch": 5, "size_bytes": 2048})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], json!("ckpt-1"));
    assert_eq!(body["data"]["step"], json!(100));
    assert_eq!(body["data"]["epoch"], json!(5));

    let id = body["data"]["id"].as_i64().unwrap();
    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/checkpoints/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(id));
}

#[tokio::test]
async fn b5_filter_by_job_id() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/checkpoints",
        Some(&token),
        Some(json!({"name": "ckpt-job1-a", "job_id": 10, "path": "/a.pt"})),
    )
    .await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/checkpoints",
        Some(&token),
        Some(json!({"name": "ckpt-job1-b", "job_id": 10, "path": "/b.pt"})),
    )
    .await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/checkpoints",
        Some(&token),
        Some(json!({"name": "ckpt-job2", "job_id": 20, "path": "/c.pt"})),
    )
    .await;

    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/checkpoints?job_id=10",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], json!(2));
    for ckpt in body["data"]["data"].as_array().unwrap() {
        assert_eq!(ckpt["job_id"], json!(10));
    }
}

#[tokio::test]
async fn b5_update_and_delete_checkpoint() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/checkpoints",
        Some(&token),
        Some(json!({"name": "upd-ckpt", "path": "/old.pt", "step": 1})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/checkpoints/{id}"),
        Some(&token),
        Some(json!({"step": 200, "epoch": 10})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["step"], json!(200));
    assert_eq!(body["data"]["epoch"], json!(10));

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/checkpoints/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/checkpoints/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn b5_checkpoint_list_requires_auth() {
    let (mut app, _pool) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/checkpoints", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
