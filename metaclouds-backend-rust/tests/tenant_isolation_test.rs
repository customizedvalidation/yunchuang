//! 多租户隔离回归测试（跨租户越权读 / 越权写）。
//!
//! # 背景
//!
//! 此前只有 job 域做了租户隔离（`services::job::Actor`）。其余域的列表接口把
//! 查询串里的 `?tenant_id=` 原样透给服务层，写接口也直接用 body 里的
//! `tenant_id` —— 于是：
//! - 非管理员传 `?tenant_id=<别人的租户>` 就能读到别人的数据；
//! - 不传则看全量；
//! - 创建时可以把资源挂到任意租户名下。
//!
//! 现在统一由 `authz::{tenant_filter, tenant_for_write, user_for_write}` 收敛，
//! 本文件锁定这些语义，防止回退。

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
use metaclouds_backend_rust::auth::password::hash_password;
use metaclouds_backend_rust::handlers::dataset::{create_dataset, list_datasets};

/// 起一条最小路由：仅数据集的列表 + 创建（租户隔离最典型的两个面）。
///
/// 选 dataset 是因为 `dataset:read/write` 对 user 角色也开放，能真正验证
/// "非管理员"路径；告警当前是 admin 专属权限，用它测会先撞在 RBAC 403 上。
///
/// 用户表：
/// - `admin`（tenant_id=1，role=admin）
/// - `tenantb`（tenant_id=2，role=manager）—— 另一个租户、且**非管理员**
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
        jwt_secret: "tenant-isolation-secret-at-least-32-chars".to_string(),
        jwt_expires_secs: 3600,
        server_port: 0,
        server_host: "127.0.0.1".to_string(),
        log_level: "warn".to_string(),
    };
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .expect("seed admin");

    // 插入租户 2 的用户：users.tenant_id 直接写 2（不依赖 tenants 表外键）。
    // role=manager 而不是 user：`dataset:write` 只对 manager/admin 开放，
    // 用 user 会先撞在 RBAC 403 上，测不到租户收敛这一步。manager 依然是
    // 非管理员，租户隔离逻辑照样对它生效。
    let hash = hash_password("TenantB@123456").expect("hash password");
    sqlx::query(
        "INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) \
         VALUES (?, ?, ?, ?, ?, 'manager', 2)",
    )
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now())
    .bind("tenantb")
    .bind("tenantb@example.com")
    .bind(&hash)
    .execute(&pool)
    .await
    .expect("insert tenant-b user");

    let state = AppState {
        pool,
        config: Arc::new(config.into()),
    };

    let read = Router::new()
        .route("/datasets", get(list_datasets))
        .route_layer(axum::middleware::from_fn_with_state(
            "dataset:read".to_string(),
            require_permission,
        ));
    let write = Router::new()
        .route("/datasets", post(create_dataset))
        .route_layer(axum::middleware::from_fn_with_state(
            "dataset:write".to_string(),
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
    body["data"]["tenant_id"]
        .as_i64()
        .or_else(|| body["data"]["user"]["tenant_id"].as_i64())
        .expect("login response should carry tenant_id");
    body["data"]["token"].as_str().unwrap().to_string()
}

/// 管理员在租户 2 下造一个数据集，返回其 id。
async fn seed_dataset_in_tenant_b(app: &mut Router, admin: &str) -> i64 {
    let (status, body) = do_req(
        app,
        "POST",
        "/api/v1/datasets",
        Some(admin),
        Some(json!({
            "name": "tenant-b-only",
            "description": "belongs to tenant 2",
            "type": "private",
            "tenant_id": 2,
        })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "admin 应能指定租户创建：{body}"
    );
    body["data"]["id"].as_i64().unwrap()
}

#[tokio::test]
async fn non_admin_cannot_read_other_tenant_via_query_param() {
    let mut app = setup_app().await;
    let admin = login_as(&mut app, "admin", "Admin@123456").await;
    let id = seed_dataset_in_tenant_b(&mut app, &admin).await;

    // 租户 2 的非管理员：显式指定自己租户 → 能看到
    let b = login_as(&mut app, "tenantb", "TenantB@123456").await;
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/datasets?tenant_id=2",
        Some(&b),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<i64> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|a| a["id"].as_i64())
        .collect();
    assert!(ids.contains(&id), "本租户数据应可见：{ids:?}");

    // 关键点：租户 2 的用户指定 ?tenant_id=1（别人的租户）→ 必须仍然只看到自己租户
    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/datasets?tenant_id=1",
        Some(&b),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let ids: Vec<i64> = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|a| a["id"].as_i64())
        .collect();
    assert!(
        ids.contains(&id),
        "tenant_id 查询参数必须被收敛到调用方自己的租户：{ids:?}"
    );
}

#[tokio::test]
async fn non_admin_cannot_create_resource_in_other_tenant() {
    let mut app = setup_app().await;
    let b = login_as(&mut app, "tenantb", "TenantB@123456").await;

    // 非管理员在 body 里指定 tenant_id=1（别人的租户）→ 应被收敛为自己的 2
    let (status, body) = do_req(
        &mut app,
        "POST",
        "/api/v1/datasets",
        Some(&b),
        Some(json!({
            "name": "smuggle",
            "description": "try to land in tenant 1",
            "type": "private",
            "tenant_id": 1,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(
        body["data"]["tenant_id"],
        json!(2),
        "非管理员创建资源必须落到自己的租户，而不是 body 指定的租户"
    );
}
