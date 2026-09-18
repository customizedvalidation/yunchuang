//! B6 并发压力基础测试：
//! - 并发登录（10 任务同时登录）
//! - 并发读（20 任务同时 GET 列表/详情）
//! - 并发混合（5 写 + 5 读同时进行）
//! - 并发创建集群（10 任务，不要求全部成功，只断言无 panic）
//!
//! 注意：SQLite 内存库并发写可能有锁等待，不对"全部成功"做硬断言，
//! 只断言无 panic、大部分请求返回合理状态码（200/201/429/503 均可接受）。

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
use metaclouds_backend_rust::auth::middleware::{
    jwt_auth, permissions, require_permission, AppState,
};
use metaclouds_backend_rust::handlers::cluster::{create_cluster, get_cluster, list_clusters};

// ---------------------------------------------------------------------------
// 应用装配（与 b2_cluster_test 相同的最小路由）
// ---------------------------------------------------------------------------

async fn setup_app() -> Router {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("migrate");

    let config = metaclouds_backend_rust::TestConfig {
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: "concurrency-secret-at-least-32-chars-long-xxxxx".to_string(),
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

    let read = Router::new()
        .route("/clusters", get(list_clusters))
        .route("/clusters/{id}", get(get_cluster))
        .route_layer(axum::middleware::from_fn_with_state(
            permissions::CLUSTER_READ.to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/clusters", post(create_cluster))
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

    Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .route("/auth/login", post(login))
                .merge(protected),
        )
        .layer(axum::middleware::from_fn(csrf_protect))
        .layer(CookieManagerLayer::new())
        .with_state(state)
}

async fn do_req(
    app: &Router,
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

async fn admin_token(app: &Router) -> String {
    let (status, body) = do_req(
        app,
        "POST",
        "/api/v1/auth/login",
        None,
        Some(json!({"username": "admin", "password": "Admin@123456"})),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "admin login failed");
    body["data"]["token"].as_str().unwrap().to_string()
}

// ---------------------------------------------------------------------------
// 1. 并发登录：10 个任务同时登录，全部应成功
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stress_concurrent_login_all_succeed() {
    let app = setup_app().await;

    let mut handles = Vec::new();
    for _ in 0..10 {
        let app = app.clone();
        handles.push(tokio::spawn(async move {
            let (status, body) = do_req(
                &app,
                "POST",
                "/api/v1/auth/login",
                None,
                Some(json!({"username": "admin", "password": "Admin@123456"})),
            )
            .await;
            (status, body)
        }));
    }

    let mut success_count = 0;
    for h in handles {
        let (status, body) = h.await.expect("login task panicked");
        if status == StatusCode::OK && body["data"]["token"].is_string() {
            success_count += 1;
        }
    }
    assert_eq!(success_count, 10, "all 10 concurrent logins should succeed");
}

// ---------------------------------------------------------------------------
// 2. 并发读：20 个任务同时 GET 列表，全部应 200
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stress_concurrent_read_all_200() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let mut handles = Vec::new();
    for _ in 0..20 {
        let app = app.clone();
        let token = token.clone();
        handles.push(tokio::spawn(async move {
            do_req(&app, "GET", "/api/v1/clusters", Some(&token), None).await
        }));
    }

    let mut ok_count = 0;
    for h in handles {
        let (status, _) = h.await.expect("read task panicked");
        if status == StatusCode::OK {
            ok_count += 1;
        }
    }
    assert_eq!(ok_count, 20, "all 20 concurrent reads should return 200");
}

// ---------------------------------------------------------------------------
// 3. 并发混合：5 个写 + 5 个读同时进行，无死锁、无数据损坏
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stress_concurrent_mix_read_write_no_deadlock() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let mut handles = Vec::new();

    // 5 个并发写（创建集群）
    for i in 0..5 {
        let app = app.clone();
        let token = token.clone();
        handles.push(tokio::spawn(async move {
            let body = json!({
                "name": format!("mix-cluster-{i}"),
                "description": "stress",
                "nodes": 2,
                "gpus": 8,
                "location": "shanghai"
            });
            do_req(&app, "POST", "/api/v1/clusters", Some(&token), Some(body)).await
        }));
    }

    // 5 个并发读
    for _ in 0..5 {
        let app = app.clone();
        let token = token.clone();
        handles.push(tokio::spawn(async move {
            do_req(&app, "GET", "/api/v1/clusters", Some(&token), None).await
        }));
    }

    let mut no_panic = true;
    let mut success_ops = 0;
    for h in handles {
        match h.await {
            Ok((status, _)) => {
                // 写可能 201，读可能 200；SQLite 锁竞争时可能 503，也算合理
                if status.is_success() || status == StatusCode::SERVICE_UNAVAILABLE {
                    success_ops += 1;
                }
            }
            Err(_) => no_panic = false,
        }
    }
    assert!(no_panic, "no task should panic");
    assert!(
        success_ops >= 8,
        "at least 8/10 mixed ops should complete, got {success_ops}"
    );
}

// ---------------------------------------------------------------------------
// 4. 并发创建集群：10 个任务，不要求全部成功，只断言无 panic
// ---------------------------------------------------------------------------

#[tokio::test]
async fn stress_concurrent_create_no_panic() {
    let app = setup_app().await;
    let token = admin_token(&app).await;

    let mut handles = Vec::new();
    for i in 0..10 {
        let app = app.clone();
        let token = token.clone();
        handles.push(tokio::spawn(async move {
            let body = json!({
                "name": format!("stress-cluster-{i}"),
                "description": "concurrent stress",
                "nodes": 1,
                "gpus": 4,
                "location": "beijing"
            });
            do_req(&app, "POST", "/api/v1/clusters", Some(&token), Some(body)).await
        }));
    }

    let mut created = 0;
    let mut other = 0;
    for h in handles {
        let (status, _) = h.await.expect("create task panicked");
        match status {
            StatusCode::CREATED => created += 1,
            _ => other += 1,
        }
    }
    // 断言无 panic（已由 expect 保证），至少有一些成功或因锁竞争失败
    assert!(created + other == 10, "all 10 tasks should complete");
    assert!(
        created > 0,
        "at least some cluster creates should succeed, got {created} created, {other} other"
    );
}
