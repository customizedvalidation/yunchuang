//! 安全攻击面集成测试：
//! - 越权（user/manager 访问更高权限端点 → 403）
//! - CSRF 绕过（Bearer 跳过 / Cookie 缺/错/对 token / GET 不校验）
//! - JWT 伪造（无 token / 格式错误 / 错误密钥 / 过期 / 篡改签名）
//! - SQL 注入、路径遍历、超大请求体
//!
//! 目标：覆盖真实攻击面，确保参数化查询、RBAC、CSRF 双提交与 JWT 签名校验
//! 在装配后的完整路由栈上生效。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::routing::{delete, get, post};
use axum::Router;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_cookies::CookieManagerLayer;

use metaclouds_backend_rust::auth::csrf::csrf_protect;
use metaclouds_backend_rust::auth::handler::login;
use metaclouds_backend_rust::auth::middleware::{
    jwt_auth, permissions, require_permission, AppState,
};
use metaclouds_backend_rust::handlers::cluster::create_cluster;
use metaclouds_backend_rust::handlers::tenant::{create_tenant, delete_tenant, list_tenants};
use metaclouds_backend_rust::handlers::user::{create_user, delete_user};

const JWT_SECRET: &str = "sec-attack-surface-secret-32-chars-long-xx";

// ---------------------------------------------------------------------------
// 帮助函数
// ---------------------------------------------------------------------------

async fn send(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, v)
}

fn builder(method: &str, uri: &str) -> axum::http::request::Builder {
    Request::builder().method(method).uri(uri)
}

async fn login_token(app: &Router, username: &str, password: &str) -> String {
    let req = builder("POST", "/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"username": username, "password": password}).to_string(),
        ))
        .unwrap();
    let (status, body) = send(app, req).await;
    assert_eq!(status, StatusCode::OK, "login failed for {username}");
    body["data"]["token"].as_str().unwrap().to_string()
}

fn bearer_req(method: &str, uri: &str, token: Option<&str>, body: Option<Value>) -> Request<Body> {
    let mut b = builder(method, uri);
    if let Some(t) = token {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    match body {
        Some(v) => b
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(v.to_string()))
            .unwrap(),
        None => b.body(Body::empty()).unwrap(),
    }
}

// ---------------------------------------------------------------------------
// 应用装配
// ---------------------------------------------------------------------------

async fn seed_user(pool: &sqlx::SqlitePool, username: &str, role: &str) {
    let hash = metaclouds_backend_rust::hash_password("attack-pass-123456").unwrap();
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
    )
    .bind(now)
    .bind(now)
    .bind(username)
    .bind(format!("{username}@example.com"))
    .bind(hash)
    .bind(role)
    .execute(pool)
    .await
    .unwrap();
}

async fn setup_app() -> (Router, sqlx::SqlitePool) {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .expect("seed admin");
    seed_user(&pool, "plainuser", "user").await;
    seed_user(&pool, "mgruser", "manager").await;

    let config = metaclouds_backend_rust::TestConfig {
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: JWT_SECRET.to_string(),
        jwt_expires_secs: 3600,
        server_port: 0,
        server_host: "127.0.0.1".to_string(),
        log_level: "warn".to_string(),
    };
    let state = AppState {
        pool: pool.clone(),
        config: Arc::new(config.into()),
    };

    // 受保护路由：按权限层叠。
    let tenants_write = Router::new()
        .route("/tenants", post(create_tenant))
        .route("/tenants/{id}", delete(delete_tenant))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::TENANT_WRITE.to_string(),
            require_permission,
        ));
    let tenants_read = Router::new()
        .route("/tenants", get(list_tenants))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::TENANT_READ.to_string(),
            require_permission,
        ));
    let clusters_write = Router::new()
        .route("/clusters", post(create_cluster))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::CLUSTER_WRITE.to_string(),
            require_permission,
        ));
    // /users 整组要求 admin。
    let users = Router::new()
        .route("/", post(create_user))
        .route("/{id}", delete(delete_user))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::ADMIN.to_string(),
            require_permission,
        ));

    let protected = tenants_read
        .merge(tenants_write)
        .merge(clusters_write)
        .nest("/users", users)
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

/// 仅 CSRF 隔离路由（无 JWT，便于测 Cookie 通道）。
fn csrf_only_app() -> Router {
    Router::new()
        .route("/api/v1/action", post(|| async { "ok" }))
        .route("/api/v1/view", get(|| async { "ok" }))
        .layer(axum::middleware::from_fn(csrf_protect))
        .layer(CookieManagerLayer::new())
}

// ---------------------------------------------------------------------------
// 1. 越权测试
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sec_attack_user_cannot_create_tenant_forbidden() {
    let (app, _pool) = setup_app().await;
    let token = login_token(&app, "plainuser", "attack-pass-123456").await;
    let (status, body) = send(
        &app,
        bearer_req(
            "POST",
            "/api/v1/tenants",
            Some(&token),
            Some(json!({"name": "x"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], json!("FORBIDDEN"));
}

#[tokio::test]
async fn sec_attack_user_cannot_create_cluster_forbidden() {
    let (app, _pool) = setup_app().await;
    let token = login_token(&app, "plainuser", "attack-pass-123456").await;
    let (status, _) = send(
        &app,
        bearer_req(
            "POST",
            "/api/v1/clusters",
            Some(&token),
            Some(json!({"name": "x"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn sec_attack_manager_cannot_delete_tenant_forbidden() {
    let (app, _pool) = setup_app().await;
    let token = login_token(&app, "mgruser", "attack-pass-123456").await;
    // manager 有 cluster:write 但无 tenant:write。
    let (status, _) = send(
        &app,
        bearer_req("DELETE", "/api/v1/tenants/1", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn sec_attack_manager_cannot_create_user_forbidden() {
    let (app, _pool) = setup_app().await;
    let token = login_token(&app, "mgruser", "attack-pass-123456").await;
    let (status, _) = send(
        &app,
        bearer_req(
            "POST",
            "/api/v1/users",
            Some(&token),
            Some(json!({"username": "new", "password": "x", "email": "a@b.c", "role": "user"})),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn sec_attack_user_cannot_delete_user_forbidden() {
    let (app, _pool) = setup_app().await;
    let token = login_token(&app, "plainuser", "attack-pass-123456").await;
    let (status, _) = send(
        &app,
        bearer_req("DELETE", "/api/v1/users/1", Some(&token), None),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

// ---------------------------------------------------------------------------
// 2. CSRF 绕过测试（Cookie 通道）
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sec_attack_csrf_bearer_skips_validation() {
    let app = csrf_only_app();
    let req = builder("POST", "/api/v1/action")
        .header(header::AUTHORIZATION, "Bearer some-token")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn sec_attack_csrf_cookie_without_token_rejected() {
    let app = csrf_only_app();
    let req = builder("POST", "/api/v1/action")
        .header(header::COOKIE, "access_token=fake")
        .body(Body::empty())
        .unwrap();
    let (status, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["success"], json!(false));
}

#[tokio::test]
async fn sec_attack_csrf_wrong_token_rejected() {
    let app = csrf_only_app();
    let req = builder("POST", "/api/v1/action")
        .header(header::COOKIE, "access_token=fake; csrf_token=good")
        .header("x-csrf-token", "bad")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn sec_attack_csrf_correct_token_passes() {
    let app = csrf_only_app();
    let req = builder("POST", "/api/v1/action")
        .header(header::COOKIE, "access_token=fake; csrf_token=good")
        .header("x-csrf-token", "good")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn sec_attack_csrf_get_not_validated() {
    let app = csrf_only_app();
    let req = builder("GET", "/api/v1/view")
        .header(header::COOKIE, "access_token=fake")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::OK);
}

// ---------------------------------------------------------------------------
// 3. JWT 伪造测试
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
struct FakeClaims {
    user_id: u64,
    username: String,
    email: String,
    role: String,
    tenant_id: u64,
    exp: i64,
    iat: i64,
    jti: String,
}

fn craft_token(secret: &str, exp_offset_secs: i64, role: &str) -> String {
    let now = chrono::Utc::now().timestamp();
    let claims = FakeClaims {
        user_id: 42,
        username: "forger".into(),
        email: "forger@example.com".into(),
        role: role.into(),
        tenant_id: 1,
        exp: now + exp_offset_secs,
        iat: now,
        jti: "forged-jti".into(),
    };
    jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

#[tokio::test]
async fn sec_attack_jwt_missing_is_unauthorized() {
    let (app, _pool) = setup_app().await;
    let (status, body) = send(&app, bearer_req("GET", "/api/v1/tenants", None, None)).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["code"], json!("UNAUTHORIZED"));
}

#[tokio::test]
async fn sec_attack_jwt_malformed_is_unauthorized() {
    let (app, _pool) = setup_app().await;
    let (status, _) = send(
        &app,
        bearer_req("GET", "/api/v1/tenants", Some("not.a-jwt"), None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sec_attack_jwt_basic_auth_format_is_bad_request() {
    let (app, _pool) = setup_app().await;
    // 非 Bearer 方案 → 400。
    let req = builder("GET", "/api/v1/tenants")
        .header(header::AUTHORIZATION, "Basic dXNlcjpwYXNz")
        .body(Body::empty())
        .unwrap();
    let (status, _) = send(&app, req).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn sec_attack_jwt_wrong_secret_is_unauthorized() {
    let (app, _pool) = setup_app().await;
    let forged = craft_token("a-completely-different-secret-32-chars", 3600, "admin");
    let (status, _) = send(
        &app,
        bearer_req("GET", "/api/v1/tenants", Some(&forged), None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sec_attack_jwt_expired_is_unauthorized() {
    let (app, _pool) = setup_app().await;
    // 即使是用正确密钥签发，过期也必须被拒（leeway 仅 60s，这里 -300 必过期）。
    let expired = craft_token(JWT_SECRET, -300, "admin");
    let (status, _) = send(
        &app,
        bearer_req("GET", "/api/v1/tenants", Some(&expired), None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn sec_attack_jwt_tampered_signature_is_unauthorized() {
    let (app, _pool) = setup_app().await;
    let valid = craft_token(JWT_SECRET, 3600, "user");
    // 用一段全新的同长度 base64 签名替换原签名 → HMAC 必然不匹配。
    let mut parts: Vec<&str> = valid.split('.').collect();
    let sig = parts[2];
    let fake_sig = "A".repeat(sig.len());
    parts[2] = &fake_sig;
    let tampered = parts.join(".");
    let (status, _) = send(
        &app,
        bearer_req("GET", "/api/v1/tenants", Some(&tampered), None),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

// ---------------------------------------------------------------------------
// 4. 其他攻击面
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sec_attack_sql_injection_login_does_not_bypass() {
    let (app, _pool) = setup_app().await;
    // 经典万能口令注入：参数化查询应使其仅作为普通用户名查找 → 401。
    let req = builder("POST", "/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"username": "' OR 1=1 --", "password": "' OR '1'='1"}).to_string(),
        ))
        .unwrap();
    let (status, body) = send(&app, req).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["success"], json!(false));
}

#[tokio::test]
async fn sec_attack_path_traversal_is_not_found() {
    let (app, _pool) = setup_app().await;
    let token = login_token(&app, "admin", "Admin@123456").await;
    // 路径穿越：axum 不做目录遍历逃逸，应 404（或 400）。
    let (status, _) = send(
        &app,
        bearer_req("GET", "/api/v1/../etc/passwd", Some(&token), None),
    )
    .await;
    assert!(
        matches!(status, StatusCode::NOT_FOUND | StatusCode::BAD_REQUEST),
        "unexpected status {status}"
    );
}

#[tokio::test]
async fn sec_attack_oversized_body_is_rejected() {
    let (app, _pool) = setup_app().await;
    // 远超合理 JSON 的请求体：不应成功处理（400/413/422）。
    let huge = "a".repeat(2_000_000);
    let req = builder("POST", "/api/v1/auth/login")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(huge))
        .unwrap();
    let (status, _) = send(&app, req).await;
    assert!(
        matches!(
            status,
            StatusCode::BAD_REQUEST
                | StatusCode::PAYLOAD_TOO_LARGE
                | StatusCode::UNPROCESSABLE_ENTITY
        ),
        "unexpected status {status}"
    );
}
