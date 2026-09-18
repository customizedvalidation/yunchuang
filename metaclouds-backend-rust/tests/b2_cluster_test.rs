//! WP-P2-B2 集群域集成测试（对齐 Go `cluster_controller_test.go` 用例面）。
//!
//! Bearer 通道跳过 CSRF。读需 `cluster:read`，写需 `cluster:write`；
//! admin 短路放行，plainuser 无集群权限（403）。

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
use metaclouds_backend_rust::handlers::cluster::{
    create_cluster, delete_cluster, get_cluster, list_clusters, update_cluster,
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
        jwt_secret: "b2-cluster-secret-at-least-32-characters-long-xxxxxx".to_string(),
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
        .route("/clusters", get(list_clusters))
        .route("/clusters/{id}", get(get_cluster))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::CLUSTER_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/clusters", post(create_cluster))
        .route("/clusters/{id}", put(update_cluster).delete(delete_cluster))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::CLUSTER_WRITE.to_string(),
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
async fn b2_cluster_list_requires_auth() {
    let (mut app, _p) = setup_app().await;
    let (status, _) = do_req(&mut app, "GET", "/api/v1/clusters", None, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn b2_cluster_create_and_get() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(&mut app, "POST", "/api/v1/clusters", Some(&token),
        Some(json!({"name": "gpu-cluster-1", "description": "main", "nodes": 4, "gpus": 32, "location": "shanghai"}))).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["data"]["name"], json!("gpu-cluster-1"));
    assert_eq!(body["data"]["status"], json!("active"));
    assert_eq!(body["data"]["gpus"], json!(32));
    assert!(body["data"].get("deleted_at").is_none());
    let id = body["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/clusters/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["id"], json!(id));
}

#[tokio::test]
async fn b2_cluster_list_pagination_and_search() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    for n in ["alpha", "beta", "gamma"] {
        do_req(
            &mut app,
            "POST",
            "/api/v1/clusters",
            Some(&token),
            Some(json!({"name": n})),
        )
        .await;
    }
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/clusters?page=1&page_size=2",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["data"].as_array().unwrap().len() <= 2);

    let (_, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/clusters?search=alpha",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(body["data"].as_array().unwrap().len(), 1);
    assert_eq!(body["data"][0]["name"], json!("alpha"));
}

#[tokio::test]
async fn b2_cluster_get_not_found() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/clusters/99999",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["code"], json!("NOT_FOUND"));
}

#[tokio::test]
async fn b2_cluster_update_and_soft_delete() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    let (_, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/clusters",
        Some(&token),
        Some(json!({"name": "del-me"})),
    )
    .await;
    let id = created["data"]["id"].as_i64().unwrap();

    let (status, body) = do_req(
        &mut app,
        "PUT",
        &format!("/api/v1/clusters/{id}"),
        Some(&token),
        Some(json!({"description": "updated", "status": "active"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["data"]["description"], json!("updated"));

    let (status, _) = do_req(
        &mut app,
        "DELETE",
        &format!("/api/v1/clusters/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/clusters/{id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn b2_cluster_duplicate_name_conflict() {
    let (mut app, _p) = setup_app().await;
    let token = admin_token(&mut app).await;
    do_req(
        &mut app,
        "POST",
        "/api/v1/clusters",
        Some(&token),
        Some(json!({"name": "dup"})),
    )
    .await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/clusters",
        Some(&token),
        Some(json!({"name": "dup"})),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["code"], json!("CONFLICT"));
}

#[tokio::test]
async fn b2_cluster_plain_user_forbidden() {
    let (mut app, _p) = setup_app().await;
    let utoken = login_token(&mut app, "plainuser", "user-pass-123456").await;
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/clusters",
        Some(&utoken),
        Some(json!({"name": "x"})),
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], json!("FORBIDDEN"));
}
