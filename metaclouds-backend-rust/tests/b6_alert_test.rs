//! WP-P2-B6 Alert 集成测试。
//!
//! Alert CRUD + acknowledge + resolve + stats + 分页 + 过滤 + 401/403。

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
use metaclouds_backend_rust::handlers::alert::{
    acknowledge_alert, create_alert, delete_alert, get_alert, get_alert_stats, list_alerts,
    resolve_alert, update_alert,
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
        jwt_secret: "b6-alert-secret-at-least-32-chars-long-xxxx".to_string(),
        jwt_expires_secs: 3600,
        server_port: 0,
        server_host: "127.0.0.1".to_string(),
        log_level: "warn".to_string(),
    };
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .expect("seed admin");

    // 额外插入一个普通用户（role=user），用于 403 测试。
    let hash = hash_password("User@123456").expect("hash password");
    sqlx::query(
        "INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) \
         VALUES (?, ?, ?, ?, ?, 'user', 1)",
    )
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now())
    .bind("testuser")
    .bind("testuser@example.com")
    .bind(&hash)
    .execute(&pool)
    .await
    .expect("insert test user");

    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.into()),
    };

    let read = Router::new()
        .route("/alerts", get(list_alerts))
        .route("/alerts/stats", get(get_alert_stats))
        .route("/alerts/{id}", get(get_alert))
        .route_layer(axum::middleware::from_fn_with_state(
            "alert:read".to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/alerts", post(create_alert))
        .route("/alerts/{id}", put(update_alert).delete(delete_alert))
        .route("/alerts/{id}/acknowledge", post(acknowledge_alert))
        .route("/alerts/{id}/resolve", post(resolve_alert))
        .route_layer(axum::middleware::from_fn_with_state(
            "alert:write".to_string(),
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
    assert_eq!(status, StatusCode::OK, "login failed for {username}");
    body["data"]["token"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn b6_create_and_get_alert() {
    let (mut app, _pool) = setup_app().await;
    let token = login_as(&mut app, "admin", "Admin@123456").await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/alerts",
        Some(&token),
        Some(json!({"name": "gpu-overheat", "message": "GPU temp high", "severity": "critical", "type": "gpu"})),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], json!("gpu-overheat"));
    assert_eq!(body["data"]["severity"], json!("critical"));
    assert_eq!(body["data"]["type"], json!("gpu"));
    assert_eq!(body["data"]["status"], json!("active"));

    let id = body["data"]["id"].as_i64().unwrap();
    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/alerts/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(id));
}

#[tokio::test]
async fn b6_list_alerts_pagination() {
    let (mut app, _pool) = setup_app().await;
    let token = login_as(&mut app, "admin", "Admin@123456").await;
    for i in 0..3 {
        do_req(
            &mut app,
            "POST",
            "/api/v1/alerts",
            Some(&token),
            Some(json!({"name": format!("alert-{i}"), "message": "m", "severity": "warning"})),
        )
        .await;
    }
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/alerts?page=1&page_size=2",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], json!(3));
    assert_eq!(body["data"]["data"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn b6_filter_alerts_by_severity_and_type() {
    let (mut app, _pool) = setup_app().await;
    let token = login_as(&mut app, "admin", "Admin@123456").await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/alerts",
        Some(&token),
        Some(json!({"name": "a1", "severity": "critical", "type": "gpu", "message": "m"})),
    )
    .await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/alerts",
        Some(&token),
        Some(json!({"name": "a2", "severity": "warning", "type": "job", "message": "m"})),
    )
    .await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/alerts",
        Some(&token),
        Some(json!({"name": "a3", "severity": "critical", "type": "system", "message": "m"})),
    )
    .await;

    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/alerts?severity=critical",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], json!(2));

    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/alerts?type=gpu",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], json!(1));
}

#[tokio::test]
async fn b6_acknowledge_and_resolve_alert() {
    let (mut app, _pool) = setup_app().await;
    let token = login_as(&mut app, "admin", "Admin@123456").await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/alerts",
        Some(&token),
        Some(json!({"name": "ack-test", "message": "m"})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/alerts/{id}/acknowledge"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], json!("acknowledged"));
    assert!(body["data"]["acknowledged_at"].is_string());
    assert!(body["data"]["acknowledged_by"].is_i64());

    let (status, body) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/alerts/{id}/resolve"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["status"], json!("resolved"));
    assert!(body["data"]["resolved_at"].is_string());
}

#[tokio::test]
async fn b6_alert_stats() {
    let (mut app, _pool) = setup_app().await;
    let token = login_as(&mut app, "admin", "Admin@123456").await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/alerts",
        Some(&token),
        Some(json!({"name": "s1", "severity": "critical", "message": "m"})),
    )
    .await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/alerts",
        Some(&token),
        Some(json!({"name": "s2", "severity": "warning", "message": "m"})),
    )
    .await;

    let (status, body) = do_req(&mut app, "GET", "/api/v1/alerts/stats", Some(&token), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["total"], json!(2));
    assert_eq!(body["data"]["active"], json!(2));
    assert!(body["data"]["by_severity"].is_object());
    assert!(body["data"]["by_status"].is_object());
}

#[tokio::test]
async fn b6_delete_alert_soft() {
    let (mut app, _pool) = setup_app().await;
    let token = login_as(&mut app, "admin", "Admin@123456").await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/alerts",
        Some(&token),
        Some(json!({"name": "del-me", "message": "m"})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/alerts/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/alerts/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn b6_alerts_require_auth() {
    let (mut app, _pool) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/alerts", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b6_alerts_forbidden_for_user_role() {
    let (mut app, _pool) = setup_app().await;
    let user_token = login_as(&mut app, "testuser", "User@123456").await;
    // user role 没有 alert:read / alert:write 权限 → 403
    let (status, _) = do_req(&mut app, "GET", "/api/v1/alerts", Some(&user_token), None).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
