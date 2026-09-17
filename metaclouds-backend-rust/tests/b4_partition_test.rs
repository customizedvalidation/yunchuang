//! WP-P2-B4 分区域集成测试（对齐 Go `partition_controller_test.go` 用例面）。
//!
//! Bearer 通道跳过 CSRF。读需 `partition:read`，写需 `partition:write`；
//! admin 短路放行，plainuser 仅有读权限（写/授权 403）。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::routing::{delete, get, post, put};
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
    create_partition, delete_partition, get_partition, get_partition_resources, grant_permission,
    list_partitions, revoke_permission, update_partition,
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
        jwt_secret: "b4-partition-secret-at-least-32-characters-long-xxxxx".to_string(),
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
        .route("/partitions", get(list_partitions))
        .route("/partitions/{id}", get(get_partition))
        .route("/partitions/{id}/resources", get(get_partition_resources))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::PARTITION_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/partitions", post(create_partition))
        .route(
            "/partitions/{id}",
            put(update_partition).delete(delete_partition),
        )
        .route("/partitions/{id}/permissions", post(grant_permission))
        .route(
            "/partitions/{id}/permissions/{permId}",
            delete(revoke_permission),
        )
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
async fn b4_partition_list_requires_auth() {
    let (mut app, _p) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/partitions", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b4_partition_create_and_get() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/partitions",
        Some(&token),
        Some(json!({
            "cluster_id": 1,
            "name": "p1",
            "description": "main",
            "partition_type": "exclusive",
            "gpu_count": 8,
            "cpu_cores": 64.0,
            "memory_gb": 512.0
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], json!("p1"));
    assert_eq!(body["data"]["partition_type"], json!("exclusive"));
    assert_eq!(body["data"]["status"], json!("active"));
    assert!(body["data"].get("deleted_at").is_none());
    let id = body["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/partitions/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(id));
}

#[tokio::test]
async fn b4_partition_list_pagination_and_filter() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    for (n, t) in [
        ("alpha", "exclusive"),
        ("beta", "shared"),
        ("gamma", "shared"),
    ] {
        do_req(
            &mut app,
            "POST",
            "/api/v1/partitions",
            Some(&token),
            Some(json!({"cluster_id": 1, "name": n, "partition_type": t})),
        )
        .await;
    }
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/partitions?page=1&page_size=2",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["page"], json!(1));
    assert!(body["data"]["data"].as_array().unwrap().len() <= 2);

    let (_, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/partitions?partition_type=shared",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body["data"]["total"].as_i64().unwrap(), 2);
}

#[tokio::test]
async fn b4_partition_resources_and_soft_delete() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/partitions",
        Some(&token),
        Some(json!({"cluster_id": 1, "name": "res", "gpu_count": 16, "cpu_cores": 128.0})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/partitions/{id}/resources"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["partition_id"], json!(id));
    assert_eq!(body["data"]["gpu_count"], json!(16));

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/partitions/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/partitions/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn b4_partition_grant_and_revoke_permission() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/partitions",
        Some(&token),
        Some(json!({"cluster_id": 1, "name": "perm"})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/partitions/{id}/permissions"),
        Some(&token),
        Some(json!({"user_id": 2, "tenant_id": 1, "permission_type": "write"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["permission_type"], json!("write"));
    let perm_id = body["data"]["id"].as_i64().unwrap();

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/partitions/{id}/permissions/{perm_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn b4_partition_duplicate_name_conflict() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/partitions",
        Some(&token),
        Some(json!({"cluster_id": 1, "name": "dup"})),
    )
    .await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/partitions",
        Some(&token),
        Some(json!({"cluster_id": 1, "name": "dup"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], json!("CONFLICT"));
}

#[tokio::test]
async fn b4_partition_plain_user_forbidden_on_write() {
    let (mut app, _p) = setup_app().await;
    let utoken = login_token(&mut app, "plainuser", "user-pass-123456").await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/partitions",
        Some(&utoken),
        Some(json!({"cluster_id": 1, "name": "x"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], json!("FORBIDDEN"));
}
