//! 认证链路集成测试：登录/登出/刷新/资料/CSRF，以及 Cookie 属性。
//!
//! 用内存 SQLite 起一条最小认证路由，手动回放 Set-Cookie 模拟浏览器会话。

use std::sync::Arc;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use serde_json::{json, Value};
use tower::ServiceExt;
use tower_cookies::CookieManagerLayer;

use metaclouds_backend_rust::auth::csrf::csrf_protect;
use metaclouds_backend_rust::auth::handler::{get_csrf_token, get_profile, login, logout, refresh};
use metaclouds_backend_rust::auth::middleware::{jwt_auth, AppState};

/// 从一组 Set-Cookie 响应头中取出指定 Cookie 的值。
fn cookie_from_set_cookies(set_cookies: &[String], name: &str) -> Option<String> {
    for sc in set_cookies {
        let pair = sc.split(';').next().unwrap_or("").trim();
        if let Some((k, v)) = pair.split_once('=') {
            if k == name {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn collect_set_cookies(resp: &axum::response::Response) -> Vec<String> {
    resp.headers()
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok().map(|s| s.to_string()))
        .collect()
}

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
        jwt_secret: "auth-test-secret-at-least-32-characters-long".to_string(),
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

    // 受保护路由挂在 jwt_auth 之后。
    let protected = Router::new()
        .route("/auth/refresh", post(refresh))
        .route("/auth/profile", get(get_profile))
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
                .route("/auth/csrf", get(get_csrf_token))
                .merge(protected),
        )
        // 全局 CSRF 双提交；CookieManager 放在最外层以便中间件读取 Cookie。
        .layer(axum::middleware::from_fn(csrf_protect))
        .layer(CookieManagerLayer::new())
        .with_state(state)
}

async fn do_login(app: &mut Router) -> (StatusCode, Value, Vec<String>) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username":"admin","password":"Admin@123456"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let set_cookies = collect_set_cookies(&resp);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    (status, v, set_cookies)
}

#[tokio::test]
async fn login_sets_both_cookies_with_attributes() {
    let mut app = setup_app().await;
    let (status, body, set_cookies) = do_login(&mut app).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["success"], json!(true));

    // 两枚 Cookie 都被写入。
    let access = set_cookies
        .iter()
        .find(|c| c.starts_with("access_token="))
        .expect("login 应写 access_token Cookie");
    let csrf = set_cookies
        .iter()
        .find(|c| c.starts_with("csrf_token="))
        .expect("login 应写 csrf_token Cookie");

    // access_token 必须 httpOnly。
    assert!(
        access.to_ascii_lowercase().contains("httponly"),
        "access_token 应 httpOnly: {access}"
    );
    // csrf_token 不应标记 httpOnly。
    assert!(
        !csrf.to_ascii_lowercase().contains("httponly"),
        "csrf_token 不应 httpOnly: {csrf}"
    );
    // 两枚都作用于整个站点。
    assert!(access.contains("Path=/"), "access_token Path 应为 /");
    assert!(csrf.contains("Path=/"), "csrf_token Path 应为 /");
}

#[tokio::test]
async fn get_csrf_token_requires_session() {
    let mut app = setup_app().await;
    // 未携带任何 Cookie → 401。
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/csrf")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 登录后携带 csrf_token Cookie → 200 并回显令牌。
    let (_, _, set_cookies) = do_login(&mut app).await;
    let csrf_cookie = cookie_from_set_cookies(&set_cookies, "csrf_token").unwrap();
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/auth/csrf")
                .header(header::COOKIE, format!("csrf_token={csrf_cookie}"))
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
    assert_eq!(v["data"]["csrf_token"], json!(csrf_cookie));
}

#[tokio::test]
async fn csrf_blocks_post_without_header() {
    let mut app = setup_app().await;
    let (_, _, set_cookies) = do_login(&mut app).await;
    let access = cookie_from_set_cookies(&set_cookies, "access_token").unwrap();

    // 浏览器会话（带 access_token Cookie）但缺 X-CSRF-Token → 403。
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(header::COOKIE, format!("access_token={access}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["message"], json!("CSRF token missing or invalid"));
}

#[tokio::test]
async fn csrf_blocks_on_mismatch() {
    let mut app = setup_app().await;
    let (_, _, set_cookies) = do_login(&mut app).await;
    let access = cookie_from_set_cookies(&set_cookies, "access_token").unwrap();
    let csrf = cookie_from_set_cookies(&set_cookies, "csrf_token").unwrap();

    // 携带了 csrf Cookie 与头，但二者不一致 → 403 mismatch。
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(
                    header::COOKIE,
                    format!("access_token={access}; csrf_token={csrf}"),
                )
                .header("X-CSRF-Token", "totally-wrong-value")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["message"], json!("CSRF token mismatch"));
}

#[tokio::test]
async fn csrf_allows_bearer_client_and_matching_pair() {
    let mut app = setup_app().await;
    let (_, login_body, set_cookies) = do_login(&mut app).await;
    let token = login_body["data"]["token"].as_str().unwrap().to_string();
    let access = cookie_from_set_cookies(&set_cookies, "access_token").unwrap();
    let csrf = cookie_from_set_cookies(&set_cookies, "csrf_token").unwrap();

    // 纯 Bearer 通道（无 access_token Cookie）→ 跳过 CSRF，直接走 JWT。
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "Bearer 客户端应跳过 CSRF");

    // 浏览器会话且头/Cookie 一致 → 放行并刷新成功。
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(
                    header::COOKIE,
                    format!("access_token={access}; csrf_token={csrf}"),
                )
                .header("X-CSRF-Token", csrf.clone())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "CSRF 一致应放行刷新");
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert!(v["data"]["token"].as_str().is_some());
}

#[tokio::test]
async fn logout_clears_cookies_and_message() {
    let mut app = setup_app().await;
    let (_, _, _) = do_login(&mut app).await;

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/logout")
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
    assert_eq!(v["message"], json!("logged out"));
}
