//! WP-P2-B6 Security Policy 集成测试。
//!
//! Policy CRUD + enable/disable + 分页 + 401/403。

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
use metaclouds_backend_rust::auth::middleware::{jwt_auth, require_permission, AppState};
use metaclouds_backend_rust::auth::password::hash_password;
use metaclouds_backend_rust::handlers::security::{
    create_policy, delete_policy, disable_policy, enable_policy, get_policy, list_policies,
    update_policy,
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
        jwt_secret: "b6-security-secret-at-least-32-chars-long".to_string(),
        jwt_expires_secs: 3600,
        server_port: 0,
        server_host: "127.0.0.1".to_string(),
        log_level: "warn".to_string(),
    };
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .expect("seed admin");

    let hash = hash_password("User@123456").expect("hash password");
    sqlx::query(
        "INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) \
         VALUES (?, ?, ?, ?, ?, 'user', 1)",
    )
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now())
    .bind("secuser")
    .bind("secuser@example.com")
    .bind(&hash)
    .execute(&pool)
    .await
    .expect("insert test user");

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.into()),
    };

    let read = Router::new()
        .route("/security/policies", get(list_policies))
        .route("/security/policies/{id}", get(get_policy))
        .route_layer(axum::middleware::from_fn_with_state(
            "security:read".to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/security/policies", post(create_policy))
        .route(
            "/security/policies/{id}",
            put(update_policy).delete(delete_policy),
        )
        .route("/security/policies/{id}/enable", post(enable_policy))
        .route("/security/policies/{id}/disable", post(disable_policy))
        .route_layer(axum::middleware::from_fn_with_state(
            "security:write".to_string(),
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

async fn login_as(app: &mut Router, username: &str, password: &str) -> String {
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

#[tokio::test]
async fn b6_create_and_get_policy() {
    let (mut app, _pool) = setup_app().await;
    let token = login_as(&mut app, "admin", "Admin@123456").await;
    let (status, body) = do_req(
        &mut app, "POST", "/api/v1/security/policies", Some(&token),
        Some(json!({"name": "deny-ssh", "description": "deny ssh", "policy_type": "network", "effect": "deny", "priority": 10})),
    ).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], json!("deny-ssh"));
    assert_eq!(body["data"]["policy_type"], json!("network"));
    assert_eq!(body["data"]["effect"], json!("deny"));
    assert_eq!(body["data"]["priority"], json!(10));
    assert_eq!(body["data"]["enabled"], json!(true));

    let id = body["data"]["id"].as_i64().unwrap();
    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/security/policies/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(id));
}

#[tokio::test]
async fn b6_list_policies_pagination_and_filter() {
    let (mut app, _pool) = setup_app().await;
    let token = login_as(&mut app, "admin", "Admin@123456").await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/security/policies",
        Some(&token),
        Some(json!({"name": "rbac-1", "policy_type": "rbac", "effect": "allow"})),
    )
    .await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/security/policies",
        Some(&token),
        Some(json!({"name": "net-1", "policy_type": "network", "effect": "deny"})),
    )
    .await;

    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/security/policies?page=1&page_size=10",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 2);

    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/security/policies?policy_type=rbac",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn b6_enable_disable_policy() {
    let (mut app, _pool) = setup_app().await;
    let token = login_as(&mut app, "admin", "Admin@123456").await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/security/policies",
        Some(&token),
        Some(json!({"name": "toggle", "enabled": true})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/security/policies/{id}/disable"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["enabled"], json!(false));

    let (status, body) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/security/policies/{id}/enable"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["enabled"], json!(true));
}

#[tokio::test]
async fn b6_update_and_delete_policy() {
    let (mut app, _pool) = setup_app().await;
    let token = login_as(&mut app, "admin", "Admin@123456").await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/security/policies",
        Some(&token),
        Some(json!({"name": "upd-pol", "description": "old"})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/security/policies/{id}"),
        Some(&token),
        Some(json!({"description": "new desc", "priority": 50})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["description"], json!("new desc"));
    assert_eq!(body["data"]["priority"], json!(50));

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/security/policies/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/security/policies/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn b6_policies_require_auth() {
    let (mut app, _pool) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/security/policies", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b6_policies_write_forbidden_for_user() {
    let (mut app, _pool) = setup_app().await;
    let user_token = login_as(&mut app, "secuser", "User@123456").await;
    // user role 有 security:read 但没有 security:write
    let (status, _) = do_req(
        &mut app,
        "POST",
        "/api/v1/security/policies",
        Some(&user_token),
        Some(json!({"name": "x"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
