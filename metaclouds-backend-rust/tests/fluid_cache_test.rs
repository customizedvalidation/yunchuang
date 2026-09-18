//! A2: FluidCache 路由集成测试（对齐 Go：嵌套 datasets + 顶层 /fluid-caches）。
//!
//! 覆盖：CRUD（create/list/update/delete）+ enable/disable/prefetch + 分页 + 401/403。
//! admin 具备 dataset:write；plainuser 无写权限（403）；无 token 401。

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
use metaclouds_backend_rust::handlers::fluid_cache::{
    create_fluid_cache, delete_fluid_cache, disable_fluid_cache, enable_fluid_cache,
    list_fluid_caches, prefetch_fluid_cache, update_fluid_cache,
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
        jwt_secret: "fluid-secret-at-least-32-characters-long-xxxxx".to_string(),
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

    // 读：JWT only（Go GET caches 无 RequirePermission）。
    let read = Router::new()
        .route("/datasets/{id}/caches", get(list_fluid_caches))
        .route("/datasets/{id}/fluid-caches", get(list_fluid_caches));

    // 写：dataset:write。
    let write = Router::new()
        .route("/datasets/{id}/caches", post(create_fluid_cache))
        .route("/datasets/{id}/fluid-caches", post(create_fluid_cache))
        .route(
            "/fluid-caches/{cache_id}",
            put(update_fluid_cache).delete(delete_fluid_cache),
        )
        .route("/fluid-caches/{cache_id}/enable", post(enable_fluid_cache))
        .route(
            "/fluid-caches/{cache_id}/disable",
            post(disable_fluid_cache),
        )
        .route(
            "/fluid-caches/{cache_id}/prefetch",
            post(prefetch_fluid_cache),
        )
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::DATASET_WRITE.to_string(),
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

async fn token(app: &mut Router, u: &str, p: &str) -> String {
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

#[tokio::test]
async fn fluid_cache_requires_auth() {
    let (mut app, _p) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/datasets/1/caches", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn fluid_cache_create_list_update_delete_cycle() {
    let (mut app, _p) = setup_app().await;
    let t = token(&mut app, "admin", "Admin@123456").await;

    // 创建（dataset 1）。
    let (status, body) = do_req(&mut app, "POST", "/api/v1/datasets/1/caches", Some(&t), Some(json!({
        "name": "cache-a", "namespace": "default", "path": "/var/lib/fluid/a", "cache_class": "read-through", "replicas": 1
    }))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], json!("cache-a"));
    assert_eq!(body["data"]["dataset_id"], json!(1));
    let cache_id = body["data"]["id"].as_i64().unwrap();

    // 列表（按 dataset 过滤）。
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/datasets/1/caches?page=1&page_size=10",
        Some(&t),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let arr = body["data"].as_array().unwrap();
    assert!(arr.iter().any(|c| c["id"] == json!(cache_id)));

    // Vue 别名列表。
    let (status, _) = do_req(
        &mut app,
        "GET",
        "/api/v1/datasets/1/fluid-caches",
        Some(&t),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 更新。
    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/fluid-caches/{cache_id}"),
        Some(&t),
        Some(json!({"replicas": 3})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["replicas"], json!(3));

    // enable / disable / prefetch。
    let (status, _) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/fluid-caches/{cache_id}/enable"),
        Some(&t),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/fluid-caches/{cache_id}/disable"),
        Some(&t),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = do_req(
        &mut app,
        "POST",
        &format!("/api/v1/fluid-caches/{cache_id}/prefetch"),
        Some(&t),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    // 删除（软删除，204）。
    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/fluid-caches/{cache_id}"),
        Some(&t),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn fluid_cache_plain_user_forbidden_on_write() {
    let (mut app, _p) = setup_app().await;
    let ut = token(&mut app, "plainuser", "user-pass-123456").await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/datasets/2/caches",
        Some(&ut),
        Some(json!({"name": "x"})),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "plainuser 无 dataset:write：{body}"
    );
}
