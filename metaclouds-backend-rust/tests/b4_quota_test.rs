//! WP-P2-B4 配额域集成测试。
//!
//! Bearer 通道跳过 CSRF。读需 `quota:read`，写需 `quota:write`；
//! admin 短路放行，plainuser 仅有读权限（写/check 超限 403）。

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
use metaclouds_backend_rust::handlers::quota::{
    check_quota, create_quota, delete_quota, get_quota, list_quotas, update_quota,
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
        jwt_secret: "b4-quota-secret-at-least-32-characters-long-xxxxxxxxxx".to_string(),
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
        .route("/quotas", get(list_quotas))
        .route("/quotas/{id}", get(get_quota))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::QUOTA_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/quotas", post(create_quota))
        .route("/quotas/{id}", put(update_quota).delete(delete_quota))
        .route("/quotas/check", post(check_quota))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::QUOTA_WRITE.to_string(),
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
async fn b4_quota_list_requires_auth() {
    let (mut app, _p) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/quotas", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b4_quota_create_and_get() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/quotas",
        Some(&token),
        Some(json!({
            "name": "t1-quota",
            "tenant_id": 1,
            "gpu_limit": 8,
            "cpu_limit": 64.0,
            "memory_limit_gb": 512.0
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], json!("t1-quota"));
    assert_eq!(body["data"]["gpu_limit"], json!(8));
    assert_eq!(body["data"]["gpu_used"], json!(0));
    assert_eq!(body["data"]["status"], json!("active"));
    let id = body["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/quotas/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(id));
}

#[tokio::test]
async fn b4_quota_check_allows_and_blocks() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/quotas",
        Some(&token),
        Some(json!({"name": "q", "tenant_id": 1, "gpu_limit": 4})),
    )
    .await;
    let _id = created["data"]["id"].as_i64().unwrap();

    // 请求 2 GPU <= limit 4：允许。
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/quotas/check",
        Some(&token),
        Some(
            json!({"scope_type": "tenant", "scope_id": 1, "resource_type": "gpu", "requested": 2}),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["allowed"], json!(true));

    // 请求 10 GPU > limit 4：超限。
    let (_, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/quotas/check",
        Some(&token),
        Some(
            json!({"scope_type": "tenant", "scope_id": 1, "resource_type": "gpu", "requested": 10}),
        ),
    )
    .await;
    assert_eq!(body["data"]["allowed"], json!(false));
}

#[tokio::test]
async fn b4_quota_update_and_soft_delete() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/quotas",
        Some(&token),
        Some(json!({"name": "ud", "tenant_id": 1, "gpu_limit": 4})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/quotas/{id}"),
        Some(&token),
        Some(json!({"gpu_limit": 16, "status": "suspended"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["gpu_limit"], json!(16));
    assert_eq!(body["data"]["status"], json!("suspended"));

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/quotas/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/quotas/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn b4_quota_list_filter_and_pagination() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    for n in ["a", "b", "c"] {
        do_req(
            &mut app,
            "POST",
            "/api/v1/quotas",
            Some(&token),
            Some(json!({"name": n, "tenant_id": 1})),
        )
        .await;
    }
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/quotas?tenant_id=1&page=1&page_size=2",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].is_array());

    // 另一个 tenant 过滤应为空数组。
    let (_, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/quotas?tenant_id=999",
        Some(&token),
        None,
    )
    .await;
    assert!(body["data"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn b4_quota_plain_user_forbidden_on_write() {
    let (mut app, _p) = setup_app().await;
    let utoken = login_token(&mut app, "plainuser", "user-pass-123456").await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/quotas",
        Some(&utoken),
        Some(json!({"name": "x", "tenant_id": 1})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], json!("FORBIDDEN"));
}
