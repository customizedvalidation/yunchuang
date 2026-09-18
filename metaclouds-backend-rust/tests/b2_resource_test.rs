//! WP-P2-B2 资源域集成测试（对齐 Go `resource_controller_test.go` 用例面）。
//!
//! Bearer 通道跳过 CSRF。读需 `resource:read`，写需 `resource:write`；
//! admin 短路放行，plainuser 无资源权限（403）。

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
use metaclouds_backend_rust::handlers::resource::{
    create_resource, delete_resource, get_resource, list_resources, update_resource,
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
        jwt_secret: "b2-resource-secret-at-least-32-characters-long".to_string(),
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

    let read = Router::new()
        .route("/resources", get(list_resources))
        .route("/resources/{id}", get(get_resource))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::RESOURCE_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/resources", post(create_resource))
        .route(
            "/resources/{id}",
            put(update_resource).delete(delete_resource),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::RESOURCE_WRITE.to_string(),
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
async fn b2_resource_list_requires_auth() {
    let (mut app, _p) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/resources", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b2_resource_create_and_get_detail() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(&mut app, "POST", "/api/v1/resources", Some(&token),
        Some(json!({"name": "a100-0", "type": "gpu", "cluster_id": 1, "total": 8, "used": 0, "available": 8, "vendor": "nvidia", "gpu_model": "A100"}))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], json!("a100-0"));
    assert_eq!(body["data"]["type"], json!("gpu"));
    assert_eq!(body["data"]["status"], json!("available"));
    assert!(body["data"].get("deleted_at").is_none());
    let id = body["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/resources/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(id));
    assert_eq!(body["data"]["vendor"], json!("nvidia"));
}

#[tokio::test]
async fn b2_resource_list_pagination_and_filter() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    for i in 0..3 {
        do_req(
            &mut app,
            "POST",
            "/api/v1/resources",
            Some(&token),
            Some(json!({"name": format!("cpu-{i}"), "type": "cpu", "cluster_id": 1, "total": 16})),
        )
        .await;
    }
    do_req(
        &mut app,
        "POST",
        "/api/v1/resources",
        Some(&token),
        Some(json!({"name": "gpu-x", "type": "gpu", "cluster_id": 2, "total": 8})),
    )
    .await;

    // 分页信封
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/resources?page=1&page_size=2",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 2);

    // 按 type 过滤
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/resources?type=gpu",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    for r in body["data"].as_array().unwrap() {
        assert_eq!(r["type"], json!("gpu"));
    }
    assert_eq!(body["data"].as_array().unwrap().len(), 1);

    // 按 cluster_id 过滤
    let (_, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/resources?cluster_id=2",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn b2_resource_update_fields() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/resources",
        Some(&token),
        Some(json!({"name": "upd-gpu", "type": "gpu", "total": 8, "used": 2, "available": 6})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/resources/{id}"),
        Some(&token),
        Some(json!({"status": "high_utilization", "used": 7, "available": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], json!("high_utilization"));
    assert_eq!(body["data"]["used"], json!(7));
}

#[tokio::test]
async fn b2_resource_delete_then_404() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/resources",
        Some(&token),
        Some(json!({"name": "del-gpu", "type": "gpu", "total": 4})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/resources/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/resources/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn b2_resource_plain_user_cannot_write_forbidden() {
    let (mut app, _p) = setup_app().await;
    let utoken = login_token(&mut app, "plainuser", "user-pass-123456").await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/resources",
        Some(&utoken),
        Some(json!({"name": "nope", "type": "gpu"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], json!("FORBIDDEN"));
}
