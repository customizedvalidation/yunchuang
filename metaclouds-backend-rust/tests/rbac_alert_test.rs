//! A5: alert 权限常量 RBAC 行为测试。
//!
//! 验证 `permissions::ALERT_WRITE` 中间件对 admin/manager/user 的放行/拒绝行为。
//! 对照 Go：Go `pkg/authz/authz.go` 未定义 alert:* 权限（告警解析走 monitoring:write），
//! 故 manager/user 矩阵中无 alert:write —— 它们写 alert 应 403；admin 短路放行（非 403）。

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
use metaclouds_backend_rust::authz::permissions;
use metaclouds_backend_rust::handlers::alert::{create_alert, list_alerts};

async fn seed_user(pool: &sqlx::SqlitePool, u: &str, role: &str) {
    let hash = metaclouds_backend_rust::hash_password("user-pass-123456").unwrap();
    let now = chrono::Utc::now();
    sqlx::query("INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)")
        .bind(now).bind(now).bind(u).bind(format!("{u}@example.com")).bind(hash).bind(role)
        .execute(pool).await.unwrap();
}

async fn setup_app() -> axum::Router {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .unwrap();

    seed_user(&pool, "mgr", "manager").await;
    seed_user(&pool, "plainuser", "user").await;

    let config = metaclouds_backend_rust::TestConfig::default();
    let state = AppState {
        pool,
        config: Arc::new(config.into()),
    };

    // 读：JWT only（对齐 routes.rs alerts_read）。
    let read = Router::new().route("/alerts", get(list_alerts));
    // 写：ALERT_WRITE（替代原 "alert:write" 字面量）。
    let write = Router::new()
        .route("/alerts", post(create_alert))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::ALERT_WRITE.to_string(),
            require_permission,
        ));
    let protected = read
        .merge(write)
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            jwt_auth,
        ));
    let public = Router::new().route("/auth/login", post(login));
    Router::new()
        .nest("/api/v1", public.merge(protected))
        .layer(axum::middleware::from_fn(csrf_protect))
        .layer(CookieManagerLayer::new())
        .with_state(state)
}

async fn token(app: &mut Router, u: &str) -> String {
    let password = if u == "admin" {
        "Admin@123456"
    } else {
        "user-pass-123456"
    };
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"username": u, "password": password}).to_string(),
        ))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    if u == "admin" {
        assert_eq!(status, StatusCode::OK, "admin login failed");
    }
    v["data"]["token"].as_str().unwrap().to_string()
}

async fn post_alert(app: &mut Router, t: &str) -> StatusCode {
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/alerts")
        .header(header::AUTHORIZATION, format!("Bearer {t}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"name": "disk-full", "severity": "critical"}).to_string(),
        ))
        .unwrap();
    app.clone().oneshot(req).await.unwrap().status()
}

#[tokio::test]
async fn alert_write_permission_matrix() {
    let mut app = setup_app().await;
    let admin = token(&mut app, "admin").await;
    let mgr = token(&mut app, "mgr").await;
    let plain = token(&mut app, "plainuser").await;

    // admin 短路放行：不被 403 拒绝（业务层成功或其他错均可）。
    let s_admin = post_alert(&mut app, &admin).await;
    assert_ne!(s_admin, StatusCode::FORBIDDEN, "admin 应通过 alert:write");

    // manager 矩阵无 alert:write → 403。
    let s_mgr = post_alert(&mut app, &mgr).await;
    assert_eq!(
        s_mgr,
        StatusCode::FORBIDDEN,
        "manager 应被 alert:write 拒绝"
    );

    // user 矩阵无 alert:write → 403。
    let s_user = post_alert(&mut app, &plain).await;
    assert_eq!(s_user, StatusCode::FORBIDDEN, "user 应被 alert:write 拒绝");
}

#[tokio::test]
async fn alert_read_jwt_only() {
    let mut app = setup_app().await;
    // 无 token → 401。
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/alerts")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 任意已登录角色（含 user）都能读（read 仅 JWT，对齐 Go monitoring.GET alerts）。
    let plain = token(&mut app, "plainuser").await;
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/alerts")
                .header(header::AUTHORIZATION, format!("Bearer {plain}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(resp.status(), StatusCode::FORBIDDEN);
    assert_eq!(resp.status(), StatusCode::OK);
}

#[test]
fn alert_permission_constants_match_strings() {
    assert_eq!(permissions::ALERT_READ, "alert:read");
    assert_eq!(permissions::ALERT_WRITE, "alert:write");
}
