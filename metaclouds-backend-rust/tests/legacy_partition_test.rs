//! A3: Partition 遗留路由集成测试（priority / max-runtime / permissions 列表）。
//!
//! 对照 Go partition_controller.go：PUT /partitions/:id/priority、
//! PUT /partitions/:id/max-runtime（partition:write）、GET /partitions/:id/permissions（JWT）。
//! Go 无 GET priority / GET max-runtime，故不测。

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
use metaclouds_backend_rust::handlers::partition::{
    create_partition, grant_permission, list_partition_permissions, update_max_runtime,
    update_priority,
};

async fn setup_app() -> (Router, sqlx::SqlitePool) {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    let config = metaclouds_backend_rust::TestConfig {
        database_url: "sqlite::memory:".into(),
        jwt_secret: "part-legacy-secret-at-least-32-characters-long-xx".into(),
        jwt_expires_secs: 3600,
        server_port: 0,
        server_host: "127.0.0.1".into(),
        log_level: "warn".into(),
    };
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .unwrap();
    let hash = metaclouds_backend_rust::hash_password("user-pass-123456").unwrap();
    let now = chrono::Utc::now();
    sqlx::query("INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) VALUES (?1, ?2, 'plainuser', 'plain@example.com', ?3, 'user', 1)")
        .bind(now).bind(now).bind(hash).execute(&pool).await.unwrap();

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.into()),
    };
    let read = Router::new().route(
        "/partitions/{id}/permissions",
        get(list_partition_permissions),
    );
    let write = Router::new()
        .route("/partitions", post(create_partition))
        .route("/partitions/{id}/priority", put(update_priority))
        .route("/partitions/{id}/max-runtime", put(update_max_runtime))
        .route("/partitions/{id}/permissions", post(grant_permission))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::PARTITION_WRITE.to_string(),
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn token(app: &mut Router, u: &str, p: &str) -> String {
    let (s, b) = do_req(
        app,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({"username": u, "password": p})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    b["data"]["token"].as_str().unwrap().to_string()
}

async fn make_partition(app: &mut Router, t: &str) -> i64 {
    let (_, body) = do_req(
        app,
        "POST",
        "/api/v1/partitions",
        Some(t),
        Some(json!({"cluster_id": 1, "name": "legacy"})),
    )
    .await;
    body["data"]["id"].as_i64().unwrap()
}

#[tokio::test]
async fn partition_update_priority_and_max_runtime() {
    let (mut app, _p) = setup_app().await;
    let t = token(&mut app, "admin", "Admin@123456").await;
    let id = make_partition(&mut app, &t).await;

    let (s, b) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/partitions/{id}/priority"),
        Some(&t),
        Some(json!({"priority": 7})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["data"]["message"], json!("priority updated"));

    let (s, b) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/partitions/{id}/max-runtime"),
        Some(&t),
        Some(json!({"max_runtime_minutes": 120})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["data"]["message"], json!("max runtime updated"));

    // 字段已持久化（新建分区默认 0，更新后应为 7 / 120）。
    let got: (i64, i64) =
        sqlx::query_as("SELECT priority, max_runtime_minutes FROM partitions WHERE id = ?1")
            .bind(id)
            .fetch_one(&_p)
            .await
            .unwrap();
    assert_eq!(got, (7, 120));
}

#[tokio::test]
async fn partition_list_permissions() {
    let (mut app, _p) = setup_app().await;
    let t = token(&mut app, "admin", "Admin@123456").await;
    let id = make_partition(&mut app, &t).await;

    // 先授权一条，再列表。
    do_req(
        &mut app,
        "POST",
        &format!("/api/v1/partitions/{id}/permissions"),
        Some(&t),
        Some(json!({"user_id": 2, "permission_type": "read"})),
    )
    .await;
    let (s, b) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/partitions/{id}/permissions"),
        Some(&t),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert!(!b["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn partition_priority_plain_user_forbidden() {
    let (mut app, _p) = setup_app().await;
    let t = token(&mut app, "admin", "Admin@123456").await;
    let id = make_partition(&mut app, &t).await;
    let ut = token(&mut app, "plainuser", "user-pass-123456").await;
    let (s, _) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/partitions/{id}/priority"),
        Some(&ut),
        Some(json!({"priority": 1})),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}
