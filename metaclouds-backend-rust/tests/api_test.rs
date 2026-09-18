//! Integration tests for the Metaclouds Rust spike.
//!
//! Each test builds its own axum router backed by an in-memory SQLite
//! database so tests are isolated and can run in parallel.

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

// Pull the library under test. The binary crate exposes everything we need
// via `main.rs`; for tests we re-declare the modules through a small shim.
//
// NOTE: `tests/` cannot reach `src/main.rs` modules directly. We therefore
// build the router here by re-using the public re-exports through a thin
// test-only binary. To keep this simple, tests drive the *already-built*
// router exposed via `metaclouds_backend_rust` — see `src/lib.rs` below.
//
// We rely on the crate being compiled as a library (src/lib.rs) so the
// integration tests can import it.

use metaclouds_backend_rust::build_app;

async fn setup_app() -> axum::Router {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let config = metaclouds_backend_rust::TestConfig {
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: "test-secret-at-least-32-characters-long-xxxx".to_string(),
        jwt_expires_secs: 3600,
        server_port: 0,
        server_host: "127.0.0.1".to_string(),
        log_level: "warn".to_string(),
    };
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .expect("seed admin");
    build_app(pool, config)
}

async fn login_as(
    app: &mut axum::Router,
    username: &str,
    password: &str,
) -> (StatusCode, Value, String) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": username, "password": password}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    let token = v
        .get("data")
        .and_then(|d| d.get("token"))
        .and_then(|t| t.as_str())
        .unwrap_or("")
        .to_string();
    (status, v, token)
}

#[tokio::test]
async fn test_login_success() {
    let mut app = setup_app().await;
    let (status, body, token) = login_as(&mut app, "admin", "Admin@123456").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], json!(true));
    assert!(body["data"]["token"].as_str().unwrap().len() > 10);
    assert_eq!(body["data"]["user"]["username"], json!("admin"));
    assert_eq!(body["data"]["user"]["role"], json!("admin"));
    assert!(body["timestamp"].is_i64());
    assert!(!token.is_empty());
}

#[tokio::test]
async fn test_login_wrong_password() {
    let mut app = setup_app().await;
    let (status, body, _) = login_as(&mut app, "admin", "wrong-password").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["success"], json!(false));
    assert_eq!(body["code"], json!("UNAUTHORIZED"));
}

#[tokio::test]
async fn test_login_missing_fields() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    // Either 400 (bad request) or 422 (unprocessable); accept both.
    assert!(
        matches!(
            resp.status(),
            StatusCode::BAD_REQUEST | StatusCode::UNPROCESSABLE_ENTITY
        ),
        "unexpected status {}",
        resp.status()
    );
}

#[tokio::test]
async fn test_protected_route_no_token() {
    let app = setup_app().await;
    let resp = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/users")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_user_create() {
    let mut app = setup_app().await;
    let (_, _, token) = login_as(&mut app, "admin", "Admin@123456").await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "alice", "email": "alice@example.com", "password": "hunter2!pass"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], json!(true));
    assert_eq!(v["data"]["username"], json!("alice"));
    assert_eq!(v["data"]["role"], json!("user"));
    assert!(v["data"].get("password_hash").is_none());
}

#[tokio::test]
async fn test_user_list_pagination() {
    let mut app = setup_app().await;
    let (_, _, token) = login_as(&mut app, "admin", "Admin@123456").await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/users?page=1&page_size=5")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], json!(true));
    assert!(v["data"].is_array());
}

#[tokio::test]
async fn test_user_get_by_id() {
    let mut app = setup_app().await;
    let (_, _, token) = login_as(&mut app, "admin", "Admin@123456").await;

    // Create a user.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "bob", "email": "bob@example.com", "password": "hunter2!pass"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: Value = serde_json::from_slice(&body).unwrap();
    let id = created["data"]["id"].as_i64().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/users/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["data"]["username"], json!("bob"));
}

#[tokio::test]
async fn test_user_update() {
    let mut app = setup_app().await;
    let (_, _, token) = login_as(&mut app, "admin", "Admin@123456").await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "carol", "email": "carol@example.com", "password": "hunter2!pass"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: Value = serde_json::from_slice(&body).unwrap();
    let id = created["data"]["id"].as_i64().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/api/v1/users/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"email": "carol.updated@example.com"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["data"]["email"], json!("carol.updated@example.com"));
}

#[tokio::test]
async fn test_user_delete() {
    let mut app = setup_app().await;
    let (_, _, token) = login_as(&mut app, "admin", "Admin@123456").await;

    // Create.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "dave", "email": "dave@example.com", "password": "hunter2!pass"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let created: Value = serde_json::from_slice(&body).unwrap();
    let id = created["data"]["id"].as_i64().unwrap();

    // Delete.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/v1/users/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Then GET -> 404.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/v1/users/{id}"))
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_rbac_forbidden() {
    let mut app = setup_app().await;
    let (_, _, admin_token) = login_as(&mut app, "admin", "Admin@123456").await;

    // Create a plain user.
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "regular", "email": "regular@example.com", "password": "hunter2!pass", "role": "user"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Log in as that user.
    let (_, _, user_token) = login_as(&mut app, "regular", "hunter2!pass").await;
    assert!(!user_token.is_empty());

    // Try to create another user -> 403.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/users")
                .header(header::AUTHORIZATION, format!("Bearer {user_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "mallory", "email": "mallory@example.com", "password": "hunter2!pass"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["success"], json!(false));
    assert_eq!(v["code"], json!("FORBIDDEN"));
}

#[tokio::test]
async fn test_envelope_format() {
    let mut app = setup_app().await;
    // Success envelope.
    let (status, body, _) = login_as(&mut app, "admin", "Admin@123456").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["success"].is_boolean());
    assert!(body["data"].is_object());
    assert!(body["timestamp"].is_i64());
    assert!(body.get("message").is_none());
    assert!(body.get("code").is_none());

    // Error envelope.
    let (status, body, _) = login_as(&mut app, "admin", "nope").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["success"], json!(false));
    assert!(body["message"].is_string());
    assert!(body["code"].is_string());
    assert!(body["timestamp"].is_i64());
    assert!(body.get("data").is_none());
}
