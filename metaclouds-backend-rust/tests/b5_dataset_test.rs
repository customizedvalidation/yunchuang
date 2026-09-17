//! WP-P2-B5 Dataset CRUD 集成测试。
//!
//! Bearer 通道跳过 CSRF。读路由需 `dataset:read`，写路由需 `dataset:write`；
//! admin 角色短路放行，普通 user 角色无 dataset:write 权限（403）。

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
use metaclouds_backend_rust::handlers::dataset::{
    create_dataset, delete_dataset, get_dataset, list_datasets, update_dataset,
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
        jwt_secret: "b5-dataset-secret-at-least-32-characters-long-x".to_string(),
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
        .route("/datasets", get(list_datasets))
        .route("/datasets/{id}", get(get_dataset))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::DATASET_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/datasets", post(create_dataset))
        .route("/datasets/{id}", put(update_dataset).delete(delete_dataset))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::DATASET_WRITE.to_string(),
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

#[tokio::test]
async fn b5_list_requires_auth() {
    let (mut app, _pool) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/datasets", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b5_create_and_get_dataset() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/datasets",
        Some(&token),
        Some(json!({"name": "imagenet", "description": "ImageNet dataset", "type": "public", "source_path": "/data/imagenet", "format": "tfrecord", "size_bytes": 1073741824})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], json!("imagenet"));
    assert_eq!(body["data"]["type"], json!("public"));
    assert_eq!(body["data"]["size_bytes"], json!(1073741824));

    let id = body["data"]["id"].as_i64().unwrap();
    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/datasets/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(id));
    assert_eq!(body["data"]["name"], json!("imagenet"));
}

#[tokio::test]
async fn b5_list_pagination() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    // Create 3 datasets
    for i in 0..3 {
        do_req(
            &mut app,
            "POST",
            "/api/v1/datasets",
            Some(&token),
            Some(json!({"name": format!("ds-{i}"), "type": "private"})),
        )
        .await;
    }
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/datasets?page=1&page_size=2",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["page"], json!(1));
    assert_eq!(body["data"]["page_size"], json!(2));
    assert!(body["data"]["data"].as_array().unwrap().len() <= 2);
    assert!(body["data"]["total"].as_i64().unwrap() >= 3);
}

#[tokio::test]
async fn b5_filter_by_type() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/datasets",
        Some(&token),
        Some(json!({"name": "pub-ds", "type": "public"})),
    )
    .await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/datasets",
        Some(&token),
        Some(json!({"name": "priv-ds", "type": "private"})),
    )
    .await;

    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/datasets?type=public",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for ds in body["data"]["data"].as_array().unwrap() {
        assert_eq!(ds["type"], json!("public"));
    }
}

#[tokio::test]
async fn b5_update_and_delete_dataset() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/datasets",
        Some(&token),
        Some(json!({"name": "upd-ds", "type": "private"})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/datasets/{id}"),
        Some(&token),
        Some(json!({"description": "updated desc", "format": "parquet"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["description"], json!("updated desc"));
    assert_eq!(body["data"]["format"], json!("parquet"));

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/datasets/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/datasets/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn b5_user_role_cannot_write_dataset() {
    let (mut app, _pool) = setup_app().await;
    let utoken = login_token(&mut app, "plainuser", "user-pass-123456").await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/datasets",
        Some(&utoken),
        Some(json!({"name": "should-fail", "type": "private"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], json!("FORBIDDEN"));
}

#[tokio::test]
async fn b5_get_not_found() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/datasets/99999",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], json!("NOT_FOUND"));
}
