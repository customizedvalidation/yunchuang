//! P3-05 OpenAPI（utoipa）对齐集成测试。
//!
//! 验证：
//! - `/api-docs/openapi.json` 返回 200 且 Content-Type 为 application/json。
//! - spec 为合法 OpenAPI 3.x（含 `openapi`/`info`/`paths` 字段）。
//! - spec 覆盖关键业务域（auth/users/tenants/clusters/resources/jobs/gpus）。
//! - spec 包含关键 components.schemas（User/Tenant/Cluster/Job 等）。
//! - Swagger UI 端点 `/swagger-ui/` 可访问（200 或 301）。
//! - spec 路径数 ≥ 28、方法数 ≥ 44（对齐 Go 版 routes.go 规模）。

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

use metaclouds_backend_rust::{build_app, TestConfig};

async fn setup_app() -> axum::Router {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("connect in-memory sqlite");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");

    let config = TestConfig {
        database_url: "sqlite::memory:".to_string(),
        jwt_secret: "p3-openapi-secret-at-least-32-chars-long-xxxxxx".to_string(),
        jwt_expires_secs: 3600,
        server_port: 0,
        server_host: "127.0.0.1".to_string(),
        log_level: "warn".to_string(),
    };
    metaclouds_backend_rust::seed_admin_if_empty(&pool)
        .await
        .expect("seed admin");

    build_app(pool, config)
}

async fn get(app: &mut axum::Router, uri: &str) -> (StatusCode, String, String) {
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let ct = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap_or("").to_string())
        .unwrap_or_default();
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = String::from_utf8(bytes.to_vec()).unwrap();
    (status, ct, body)
}

/// 1. openapi.json 返回 200 + application/json。
#[tokio::test]
async fn openapi_json_is_ok_and_json() {
    let mut app = setup_app().await;
    let (status, ct, body) = get(&mut app, "/api-docs/openapi.json").await;
    assert_eq!(status, StatusCode::OK, "openapi.json must return 200");
    assert!(
        ct.contains("application/json"),
        "content-type should be json, got {ct}"
    );
    assert!(!body.is_empty(), "body must not be empty");
}

/// 2. spec 覆盖关键业务域路径。
#[tokio::test]
async fn spec_covers_key_domains() {
    let mut app = setup_app().await;
    let (_status, _ct, body) = get(&mut app, "/api-docs/openapi.json").await;
    let spec: serde_json::Value = serde_json::from_str(&body).expect("valid openapi json");

    let openapi_ver = spec["openapi"].as_str().unwrap_or_default();
    assert!(
        openapi_ver.starts_with("3."),
        "openapi version should be 3.x, got {openapi_ver}"
    );
    let paths = spec["paths"].as_object().expect("paths object");

    let expected_prefixes = [
        "/api/v1/auth",
        "/api/v1/users",
        "/api/v1/tenants",
        "/api/v1/clusters",
        "/api/v1/resources",
        "/api/v1/jobs",
        "/api/v1/gpus",
    ];
    for prefix in expected_prefixes {
        let hit = paths.keys().any(|p| p.starts_with(prefix));
        assert!(hit, "spec missing paths under {prefix}");
    }
}

/// 3. spec 包含关键 components.schemas。
#[tokio::test]
async fn spec_contains_key_schemas() {
    let mut app = setup_app().await;
    let (_status, _ct, body) = get(&mut app, "/api-docs/openapi.json").await;
    let spec: serde_json::Value = serde_json::from_str(&body).expect("valid openapi json");

    let schemas = spec["components"]["schemas"]
        .as_object()
        .expect("components.schemas object");

    for name in [
        "UserResponse",
        "TenantResponse",
        "ClusterResponse",
        "JobResponse",
    ] {
        assert!(schemas.contains_key(name), "schema missing: {name}");
    }
    // GPU 设备 schema
    assert!(
        schemas.contains_key("GpuDeviceResponse"),
        "schema missing: GpuDeviceResponse"
    );
}

/// 4. Swagger UI 可访问。
#[tokio::test]
async fn swagger_ui_accessible() {
    let mut app = setup_app().await;
    let (status, _ct, _body) = get(&mut app, "/swagger-ui/").await;
    assert!(
        status == StatusCode::OK || status == StatusCode::MOVED_PERMANENTLY,
        "swagger-ui should be reachable, got {status}"
    );
}

/// 5. 路径数 ≥ 28、方法数 ≥ 44。
#[tokio::test]
async fn spec_path_and_method_counts() {
    let mut app = setup_app().await;
    let (_status, _ct, body) = get(&mut app, "/api-docs/openapi.json").await;
    let spec: serde_json::Value = serde_json::from_str(&body).expect("valid openapi json");

    let paths = spec["paths"].as_object().expect("paths object");
    let path_count = paths.len();
    let mut method_count = 0usize;
    for item in paths.values() {
        if let Some(obj) = item.as_object() {
            for m in ["get", "post", "put", "delete", "patch"] {
                if obj.contains_key(m) {
                    method_count += 1;
                }
            }
        }
    }
    assert!(
        path_count >= 28,
        "path count {path_count} < 28 (aligned to Go routes.go)"
    );
    assert!(
        method_count >= 44,
        "method count {method_count} < 44 (aligned to Go routes.go)"
    );
}
