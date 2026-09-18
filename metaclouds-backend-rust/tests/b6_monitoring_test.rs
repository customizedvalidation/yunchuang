//! WP-P2-B6 Monitoring 集成测试。
//!
//! Dashboard 13 指标 + metrics 查询 + 16 告警规则 + evaluate + 401。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_cookies::CookieManagerLayer;

use metaclouds_backend_rust::auth::csrf::csrf_protect;
use metaclouds_backend_rust::auth::handler::login;
use metaclouds_backend_rust::auth::middleware::{jwt_auth, require_permission, AppState};
use metaclouds_backend_rust::handlers::monitoring::{
    evaluate_alert_rules, get_dashboard, get_metrics, list_alert_rules,
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
        jwt_secret: "b6-monitoring-secret-at-least-32-chars-long".to_string(),
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
        .route("/monitoring/dashboard", get(get_dashboard))
        .route("/monitoring/metrics", get(get_metrics))
        .route("/monitoring/alert-rules", get(list_alert_rules))
        .route_layer(axum::middleware::from_fn_with_state(
            "monitoring:read".to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route(
            "/monitoring/alert-rules/evaluate",
            post(evaluate_alert_rules),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            "monitoring:write".to_string(),
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
async fn b6_dashboard_returns_13_metrics() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/monitoring/dashboard",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let d = &body["data"];
    // 13 个指标字段名逐字核对。
    for key in [
        "total_users",
        "active_users",
        "total_tenants",
        "total_clusters",
        "total_resources",
        "total_jobs",
        "running_jobs",
        "total_gpus",
        "allocated_gpus",
        "total_datasets",
        "total_alerts",
        "active_alerts",
        "system_uptime",
    ] {
        assert!(d.get(key).is_some(), "dashboard missing metric: {key}");
    }
    assert!(d["system_uptime"].as_i64().unwrap() >= 0);
}

#[tokio::test]
async fn b6_metrics_query_by_name() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/monitoring/metrics?name=total_users",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["name"], json!("total_users"));
    assert!(body["data"]["value"].is_number());
}

#[tokio::test]
async fn b6_alert_rules_returns_16() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/monitoring/alert-rules",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let rules = body["data"]["rules"].as_array().unwrap();
    assert_eq!(
        rules.len(),
        16,
        "expected 16 alert rules, got {}",
        rules.len()
    );
    // 抽查几条关键规则
    let names: Vec<&str> = rules.iter().filter_map(|r| r["name"].as_str()).collect();
    assert!(names.contains(&"GPU High Utilization"));
    assert!(names.contains(&"Login Failure Threshold"));
    assert!(names.contains(&"API Error Rate High"));
}

#[tokio::test]
async fn b6_evaluate_alert_rules() {
    let (mut app, _pool) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/monitoring/alert-rules/evaluate",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"]["triggered"].is_array());
    assert!(body["data"]["count"].is_number());
}

#[tokio::test]
async fn b6_monitoring_requires_auth() {
    let (mut app, _pool) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/monitoring/dashboard", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
