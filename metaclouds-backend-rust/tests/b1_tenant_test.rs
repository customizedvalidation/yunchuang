//! WP-P2-B1 租户 CRUD 集成测试（对齐 Go `tenant_controller_test.go` 的用例面）。
//!
//! Bearer 通道跳过 CSRF。读路由需 `tenant:read`，写路由需 `tenant:write`；
//! admin 角色短路放行，普通 user 角色无租户权限（403）。

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
use metaclouds_backend_rust::handlers::tenant::{
    create_tenant, delete_tenant, get_tenant, list_tenants, update_tenant,
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
        jwt_secret: "b1-tenant-secret-at-least-32-characters-long-x".to_string(),
        jwt_expires_secs: 3600,
        server_port: 0,
        server_host: "127.0.0.1".to_string(),
        log_level: "warn".to_string(),
    };
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .expect("seed admin");

    // 直接插一个普通 user 角色，用于测 403 越权。
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

    // 读：tenant:read；写：tenant:write。两组都在 jwt_auth 之后。
    let read = Router::new()
        .route("/tenants", get(list_tenants))
        .route("/tenants/{id}", get(get_tenant))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::TENANT_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/tenants", post(create_tenant))
        .route("/tenants/{id}", put(update_tenant).delete(delete_tenant))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::TENANT_WRITE.to_string(),
            require_permission,
        ));

    let protected = read
        .merge(write)
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth,
        ));

    // 公开登录路由：测试通过真实登录拿 token（admin / plainuser）。
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

/// 通过真实登录换取 token。
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
async fn b1_list_requires_auth() {
    let (mut app, _pool) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/tenants", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b1_list_sees_default_tenant_and_pagination_envelope() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/tenants?page=1&page_size=10",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], json!(true));
    assert!(body["data"].is_array());
    // 默认租户名应为 seed 的「默认租户」。
    assert!(body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["status"] == json!("active")));
}

#[tokio::test]
async fn b1_create_tenant_admin() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/tenants",
        Some(&token),
        Some(json!({"name": "alpha-tenant", "description": "first", "gpu_quota": 4})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], json!("alpha-tenant"));
    assert_eq!(body["data"]["status"], json!("active"));
    assert_eq!(body["data"]["gpu_quota"], json!(4));
    assert!(body["data"].get("deleted_at").is_none());
}

#[tokio::test]
async fn b1_create_tenant_missing_name_bad_request() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, _) = do_req(
        &mut app,
        "POST",
        "/api/v1/tenants",
        Some(&token),
        Some(json!({"gpu_quota": 5})),
    )
    .await;
    // 缺必填字段：axum 反序列化阶段可能 422，handler 校验阶段 400，二者皆可。
    assert!(
        matches!(
            status,
            StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
        ),
        "unexpected status {status}"
    );
}

#[tokio::test]
async fn b1_create_duplicate_name_conflict() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let _ = do_req(
        &mut app,
        "POST",
        "/api/v1/tenants",
        Some(&token),
        Some(json!({"name": "dup-tenant"})),
    )
    .await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/tenants",
        Some(&token),
        Some(json!({"name": "dup-tenant"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], json!("CONFLICT"));
}

#[tokio::test]
async fn b1_get_tenant_detail() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    // seed 默认租户 id=1。
    let (status, body) = do_req(&mut app, "GET", "/api/v1/tenants/1", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(1));
}

#[tokio::test]
async fn b1_get_tenant_not_found() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(&mut app, "GET", "/api/v1/tenants/99999", Some(&token), None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], json!("NOT_FOUND"));
}

#[tokio::test]
async fn b1_update_tenant_fields() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/tenants",
        Some(&token),
        Some(json!({"name": "upd-tenant"})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/tenants/{id}"),
        Some(&token),
        Some(json!({"description": "updated desc", "gpu_quota": 42})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["description"], json!("updated desc"));
    assert_eq!(body["data"]["gpu_quota"], json!(42));
}

#[tokio::test]
async fn b1_update_tenant_not_found() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "PUT",
        "/api/v1/tenants/99999",
        Some(&token),
        Some(json!({"name": "ghost"})),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], json!("NOT_FOUND"));
}

#[tokio::test]
async fn b1_delete_tenant_then_404() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/tenants",
        Some(&token),
        Some(json!({"name": "del-tenant"})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/tenants/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // 软删除后详情应 404。
    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/tenants/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn b1_user_role_cannot_write_tenant_forbidden() {
    let (mut app, _pool) = setup_app().await;
    let utoken = login_token(&mut app, "plainuser", "user-pass-123456").await;

    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/tenants",
        Some(&utoken),
        Some(json!({"name": "should-fail"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], json!("FORBIDDEN"));
}

#[tokio::test]
async fn b1_user_role_tenant_read_forbidden() {
    let (mut app, _pool) = setup_app().await;
    let utoken = login_token(&mut app, "plainuser", "user-pass-123456").await;

    // user 角色无 tenant:read 权限 → 403（Go RBAC：user 只有 job/monitoring 等读）。
    let (status, _) = do_req(&mut app, "GET", "/api/v1/tenants", Some(&utoken), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
