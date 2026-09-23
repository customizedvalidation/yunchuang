//! 2026-09-23 P1 round3 真实 DB 实现回归测试。
//!
//! 验证两个上轮用 mock 占位的端点已落到真实 DB 聚合：
//!   - GET /api/v1/jobs/:id/status   （返回真实 created_at/updated_at/cluster_name/gpu_allocations）
//!   - GET /api/v1/quotas/usage      （按 scope 从 gpu_devices + gpu_allocations 聚合）
//!
//! 断言关键：返回体不得再含 "mock" 字样，且聚合数字由插入的种子数据驱动（非写死 0）。
//! Bearer 通道跳过 CSRF；admin 短路放行权限校验。

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
use metaclouds_backend_rust::auth::middleware::{jwt_auth, AppState};
use metaclouds_backend_rust::handlers::job::{create_job, get_job_status};
use metaclouds_backend_rust::handlers::quota::get_quota_usage;

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
        jwt_secret: "p1-round3-secret-at-least-32-characters-long-xxxxx".to_string(),
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
        .route("/jobs/{id}/status", get(get_job_status))
        .route("/quotas/usage", get(get_quota_usage));
    let write = Router::new().route("/jobs", post(create_job));

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
    assert_eq!(status, StatusCode::OK, "admin login failed");
    body["data"]["token"].as_str().unwrap().to_string()
}

/// 插入一个集群，返回 cluster_id。
async fn insert_cluster(pool: &sqlx::SqlitePool, name: &str) -> i64 {
    let now = chrono::Utc::now();
    let res =
        sqlx::query("INSERT INTO clusters (created_at, updated_at, name) VALUES (?1, ?2, ?3)")
            .bind(now)
            .bind(now)
            .bind(name)
            .execute(pool)
            .await
            .unwrap();
    res.last_insert_rowid()
}

/// 插入一个 GPU 设备，返回 device_id。
#[allow(clippy::too_many_arguments)]
async fn insert_gpu_device(
    pool: &sqlx::SqlitePool,
    cluster_id: i64,
    vendor: &str,
    model: &str,
    gpu_index: i64,
    status: &str,
) -> i64 {
    let now = chrono::Utc::now();
    let res = sqlx::query(
        "INSERT INTO gpu_devices \
         (created_at, updated_at, cluster_id, node_name, vendor, model, gpu_index, \
          total_memory_gb, allocatable_memory_gb, used_memory_gb, mig_enabled, mig_profiles, \
          driver_version, cuda_version, status, utilization, temperature, power_draw, details) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 80, 80, 0, 0, '[]', '535', '12.1', ?8, 0, 35, 250, '')",
    )
    .bind(now)
    .bind(now)
    .bind(cluster_id)
    .bind(format!("node-{cluster_id}-{gpu_index}"))
    .bind(vendor)
    .bind(model)
    .bind(gpu_index)
    .bind(status)
    .execute(pool)
    .await
    .unwrap();
    res.last_insert_rowid()
}

/// 插入一条 active 的 GPU 分配记录。
async fn insert_allocation(
    pool: &sqlx::SqlitePool,
    device_id: i64,
    job_id: i64,
    tenant_id: i64,
    user_id: i64,
) {
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO gpu_allocations \
         (created_at, updated_at, device_id, job_id, tenant_id, user_id, fraction, \
          memory_gb, mig_profile, status, started_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1.0, 80, '', 'active', ?1)",
    )
    .bind(now)
    .bind(now)
    .bind(device_id)
    .bind(job_id)
    .bind(tenant_id)
    .bind(user_id)
    .execute(pool)
    .await
    .unwrap();
}

#[tokio::test]
async fn round3_job_status_returns_real_db_fields() {
    let (mut app, pool) = setup_app().await;
    let token = admin_token(&mut app).await;

    // 先建集群，再以该 cluster_id 创建作业。
    let cluster_id = insert_cluster(&pool, "round3-job-cluster").await;
    let (status, created) = do_req(
        &mut app,
        "POST",
        "/api/v1/jobs",
        Some(&token),
        Some(json!({
            "name": "round3-status-job",
            "gpus": 2,
            "cluster_id": cluster_id,
            "priority": 1,
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    let job_id = created["data"]["id"].as_i64().unwrap();

    // 给该作业绑定一块已分配的 GPU（nvidia A100）。
    let dev_id = insert_gpu_device(&pool, cluster_id, "nvidia", "A100", 0, "allocated").await;
    insert_allocation(&pool, dev_id, job_id, 1, 1).await;

    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/jobs/{job_id}/status"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let data = &body["data"];

    // 不再含 mock 字样。
    let msg = data["message"].as_str().unwrap_or("");
    assert!(
        !msg.to_lowercase().contains("mock"),
        "job status message still looks mock: {msg}"
    );

    // 真实 DB 字段。
    assert_eq!(data["job_id"], json!(job_id));
    assert_eq!(data["name"], json!("round3-status-job"));
    assert!(data["created_at"].is_string(), "created_at missing");
    assert!(data["updated_at"].is_string(), "updated_at missing");
    assert_eq!(data["cluster_id"], json!(cluster_id));
    assert_eq!(data["cluster_name"], json!("round3-job-cluster"));
    assert_eq!(data["gpu_requested"], json!(2));

    // phase 由 status 派生（新建作业为 pending -> Pending）。
    assert_eq!(data["status"], json!("pending"));
    assert_eq!(data["phase"], json!("Pending"));

    // 真实 GPU 分配列表。
    let allocs = data["gpu_allocations"].as_array().unwrap();
    assert_eq!(allocs.len(), 1);
    assert_eq!(allocs[0]["gpu_device_id"], json!(dev_id));
    assert_eq!(allocs[0]["vendor"], json!("nvidia"));
    assert_eq!(allocs[0]["model"], json!("A100"));
    assert_eq!(allocs[0]["allocation_status"], json!("active"));
}

#[tokio::test]
async fn round3_quota_usage_aggregates_real_devices_cluster_scope() {
    let (mut app, pool) = setup_app().await;
    let token = admin_token(&mut app).await;

    let cluster_id = insert_cluster(&pool, "round3-usage-cluster").await;
    // 4 块设备：2 available(nvidia) + 1 allocated(nvidia) + 1 allocated(amd)。
    insert_gpu_device(&pool, cluster_id, "nvidia", "A100", 0, "available").await;
    insert_gpu_device(&pool, cluster_id, "nvidia", "A100", 1, "available").await;
    let nvidia_alloc = insert_gpu_device(&pool, cluster_id, "nvidia", "A100", 2, "allocated").await;
    let amd_alloc = insert_gpu_device(&pool, cluster_id, "amd", "MI300", 3, "allocated").await;

    let (status, body) = do_req(
        &mut app,
        "GET",
        &format!("/api/v1/quotas/usage?scope_type=cluster&scope_id={cluster_id}"),
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let data = &body["data"];

    assert_eq!(data["scope_type"], json!("cluster"));
    assert_eq!(data["scope_id"], json!(cluster_id));
    // 真实聚合数字，非写死 0。
    assert_eq!(data["gpu_total"], json!(4));
    assert_eq!(data["gpu_allocated"], json!(2));
    assert_eq!(data["gpu_available"], json!(2));
    // 前端兼容字段。
    assert_eq!(data["gpu_used"], json!(2));

    // by_status 含 available=2 / allocated=2。
    let by_status = data["by_status"].as_array().unwrap();
    let avail = by_status
        .iter()
        .find(|r| r["status"] == json!("available"))
        .unwrap();
    let alloc = by_status
        .iter()
        .find(|r| r["status"] == json!("allocated"))
        .unwrap();
    assert_eq!(avail["count"], json!(2));
    assert_eq!(alloc["count"], json!(2));

    // by_vendor：nvidia total=3 allocated=1；amd total=1 allocated=1。
    let by_vendor = data["by_vendor"].as_array().unwrap();
    let nvidia_row = by_vendor
        .iter()
        .find(|r| r["vendor"] == json!("nvidia"))
        .unwrap();
    let amd_row = by_vendor
        .iter()
        .find(|r| r["vendor"] == json!("amd"))
        .unwrap();
    assert_eq!(nvidia_row["total"], json!(3));
    assert_eq!(nvidia_row["allocated"], json!(1));
    assert_eq!(amd_row["total"], json!(1));
    assert_eq!(amd_row["allocated"], json!(1));

    // cluster 维度无配额行 -> gpu_limit=0，用量百分比 0。
    assert_eq!(data["gpu_limit"], json!(0));
    assert_eq!(data["gpu_used_percent"], json!(0.0));
    // 不保留 mock message。
    assert!(data.get("message").is_none());
    let _ = (nvidia_alloc, amd_alloc);
}

#[tokio::test]
async fn round3_quota_usage_tenant_scope_filters_by_active_allocations() {
    let (mut app, pool) = setup_app().await;
    let token = admin_token(&mut app).await;

    let cluster_id = insert_cluster(&pool, "round3-tenant-cluster").await;
    // 租户 1 活跃占用 2 块（1 nvidia available 设备 + 1 nvidia allocated 设备）。
    let dev1 = insert_gpu_device(&pool, cluster_id, "nvidia", "A100", 0, "available").await;
    let dev2 = insert_gpu_device(&pool, cluster_id, "nvidia", "A100", 1, "allocated").await;
    // 另一块设备不属于租户 1（不插 allocation）。
    insert_gpu_device(&pool, cluster_id, "amd", "MI300", 2, "allocated").await;

    // 为租户 1 建配额上限 100。
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO resource_quotas \
         (created_at, updated_at, name, description, tenant_id, gpu_limit, gpu_used, \
          cpu_limit, cpu_used, memory_limit_gb, memory_used_gb, storage_limit_gb, \
          storage_used_gb, status) \
         VALUES (?1, ?2, 't1-quota', '', 1, 100, 0, 0, 0, 0, 0, 0, 0, 'active')",
    )
    .bind(now)
    .bind(now)
    .execute(&pool)
    .await
    .unwrap();

    // 两个 allocation 都挂在租户 1 下，job 用占位 0。
    insert_allocation(&pool, dev1, 0, 1, 1).await;
    insert_allocation(&pool, dev2, 0, 1, 1).await;

    let (status, body) = do_req(
        &mut app,
        "GET",
        "/api/v1/quotas/usage?scope_type=tenant&scope_id=1",
        Some(&token),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let data = &body["data"];

    assert_eq!(data["scope_type"], json!("tenant"));
    // 租户 1 经 active allocation 反查到 2 块设备；其中 status=allocated 的 1 块计入已分配。
    assert_eq!(data["gpu_total"], json!(2));
    assert_eq!(data["gpu_allocated"], json!(1));
    assert_eq!(data["gpu_available"], json!(1));
    // 配额上限来自 resource_quotas。
    assert_eq!(data["gpu_limit"], json!(100));
    assert_eq!(data["gpu_used"], json!(1));
    // 1/100 = 1.0%。
    assert_eq!(data["gpu_used_percent"], json!(1.0));
}
