//! WP-P2-B1 认证链路集成测试（对齐 Go `auth_controller_test.go` 的用例面）。
//!
//! 用内存 SQLite 起一条 B1 认证路由，Bearer 通道自动跳过 CSRF 双提交，
//! 因此测试只用 `Authorization: Bearer <token>`。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::routing::{get, post, put};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_cookies::CookieManagerLayer;

use metaclouds_backend_rust::auth::csrf::csrf_protect;
use metaclouds_backend_rust::auth::handler::{
    change_password, get_profile, login, logout, refresh,
};
use metaclouds_backend_rust::auth::middleware::{jwt_auth, AppState};

async fn setup_app() -> Router {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let config = metaclouds_backend_rust::TestConfig {
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: "b1-auth-secret-at-least-32-characters-long-xxxx".to_string(),
        jwt_expires_secs: 3600,
        server_port: 0,
        server_host: "127.0.0.1".to_string(),
        log_level: "warn".to_string(),
    };
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .expect("seed admin");

    let state = AppState {
        pool,
        config: Arc::new(config.into()),
    };

    let protected = Router::new()
        .route("/auth/refresh", post(refresh))
        .route("/auth/profile", get(get_profile))
        .route("/auth/change-password", put(change_password))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth,
        ));

    Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .route("/auth/login", post(login))
                .route("/auth/logout", post(logout))
                .merge(protected),
        )
        .layer(axum::middleware::from_fn(csrf_protect))
        .layer(CookieManagerLayer::new())
        .with_state(state)
}

async fn do_request(
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

async fn do_login(app: &mut Router, username: &str, password: &str) -> (StatusCode, Value) {
    do_request(
        app,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({"username": username, "password": password})),
    )
    .await
}

fn token_of(body: &Value) -> String {
    body["data"]["token"].as_str().unwrap().to_string()
}

// ---- 登录 ----

#[tokio::test]
async fn b1_login_success_returns_token_and_user() {
    let mut app = setup_app().await;
    let (status, body) = do_login(&mut app, "admin", "Admin@123456").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], json!(true));
    assert!(body["data"]["token"].as_str().unwrap().len() > 10);
    assert_eq!(body["data"]["user"]["username"], json!("admin"));
    assert_eq!(body["data"]["user"]["role"], json!("admin"));
    assert!(body["data"]["user"].get("password_hash").is_none());
    assert!(body["data"]["expires_at"].is_i64());
}

#[tokio::test]
async fn b1_login_wrong_password_is_unauthorized() {
    let mut app = setup_app().await;
    let (status, body) = do_login(&mut app, "admin", "wrong-password").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], json!("UNAUTHORIZED"));
}

#[tokio::test]
async fn b1_login_unknown_user_is_unauthorized() {
    let mut app = setup_app().await;
    // 用独立用户名，避免影响其他并行用例对 admin 的失败计数。
    let (status, body) = do_login(&mut app, "nobody_here", "whatever").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["success"], json!(false));
}

#[tokio::test]
async fn b1_login_missing_fields_is_bad_request() {
    let mut app = setup_app().await;
    let (status, _) = do_request(
        &mut app,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({})),
    )
    .await;
    assert!(
        matches!(
            status,
            StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
        ),
        "unexpected status {status}"
    );
}

#[tokio::test]
async fn b1_account_locked_after_five_failures() {
    let mut app = setup_app().await;
    // 独立用户名：5 次错误后第 6 次即使密码正确也被锁（429）。
    let u = "lockme_b1";
    for _ in 0..5 {
        let (status, _) = do_login(&mut app, u, "definitely-wrong").await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }
    // 第 6 次：即使密码正确（这里用任意密码），也应因锁定被拒。
    let (status, body) = do_login(&mut app, u, "Admin@123456").await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(body["code"], json!("RATE_LIMIT_EXCEEDED"));
}

// ---- 刷新 ----

#[tokio::test]
async fn b1_refresh_success_issues_new_token() {
    let mut app = setup_app().await;
    let (_, body) = do_login(&mut app, "admin", "Admin@123456").await;
    let token = token_of(&body);

    let (status, body) =
        do_request(&mut app, "POST", "/api/v1/auth/refresh", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["token"].as_str().is_some());
    assert_eq!(body["data"]["user"]["username"], json!("admin"));
}

#[tokio::test]
async fn b1_refresh_without_token_is_unauthorized() {
    let mut app = setup_app().await;
    let (status, _) = do_request(&mut app, "POST", "/api/v1/auth/refresh", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ---- Profile ----

#[tokio::test]
async fn b1_profile_success() {
    let mut app = setup_app().await;
    let (_, body) = do_login(&mut app, "admin", "Admin@123456").await;
    let token = token_of(&body);

    let (status, body) =
        do_request(&mut app, "GET", "/api/v1/auth/profile", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["username"], json!("admin"));
    assert_eq!(body["data"]["role"], json!("admin"));
    assert!(body["data"].get("password_hash").is_none());
    assert!(body["data"].get("deleted_at").is_none());
}

#[tokio::test]
async fn b1_profile_without_token_is_unauthorized() {
    let mut app = setup_app().await;
    let (status, _) = do_request(&mut app, "GET", "/api/v1/auth/profile", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ---- 改密 ----

#[tokio::test]
async fn b1_change_password_success_then_new_password_logs_in() {
    let mut app = setup_app().await;
    let (_, body) = do_login(&mut app, "admin", "Admin@123456").await;
    let token = token_of(&body);

    let (status, _) = do_request(
        &mut app,
        "PUT",
        "/api/v1/auth/change-password",
        Some(&token),
        Some(json!({"old_password": "Admin@123456", "new_password": "Brand-New-Pass#1"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 旧密码应登录失败。
    let (status, _) = do_login(&mut app, "admin", "Admin@123456").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // 新密码应登录成功。
    let (status, _) = do_login(&mut app, "admin", "Brand-New-Pass#1").await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn b1_change_password_wrong_old_password_rejected() {
    let mut app = setup_app().await;
    let (_, body) = do_login(&mut app, "admin", "Admin@123456").await;
    let token = token_of(&body);

    let (status, body) = do_request(
        &mut app,
        "PUT",
        "/api/v1/auth/change-password",
        Some(&token),
        Some(json!({"old_password": "not-the-old-one", "new_password": "Brand-New-Pass#1"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], json!("UNAUTHORIZED"));
}

#[tokio::test]
async fn b1_change_password_without_token_is_unauthorized() {
    let mut app = setup_app().await;
    let (status, _) = do_request(
        &mut app,
        "PUT",
        "/api/v1/auth/change-password",
        None,
        Some(json!({"old_password": "x", "new_password": "y-z-long-enough"})),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ---- 登出 ----

#[tokio::test]
async fn b1_logout_returns_ok_message() {
    let mut app = setup_app().await;
    let (status, body) = do_request(&mut app, "POST", "/api/v1/auth/logout", None, None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["message"], json!("logged out"));
}
