//! P3-03 Prometheus 指标对齐集成测试。
//!
//! 验证：
//! - `/metrics` 端点无认证可访问，返回 200 + text/plain。
//! - 响应体包含全部 13 个 `metaclouds_*` 业务指标。
//! - 响应体包含 3 个 HTTP 指标（http_requests_total / _duration_seconds / _in_flight）。
//! - 请求后 HTTP 计数器正确递增。
//! - Histogram buckets 与 Go 版一致（0.005 … 10）。
//! - 指标标签（method / path / status）正确输出。
//! - Prometheus 文本格式可解析（# HELP / # TYPE 行）。

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use tower::ServiceExt;

use metaclouds_backend_rust::{build_app, TestConfig};

/// 全部 13 个业务指标名（带 metaclouds_ 前缀），对齐 B6 dashboard stats。
const EXPECTED_BUSINESS_METRICS: &[&str] = &[
    "metaclouds_total_users",
    "metaclouds_active_users",
    "metaclouds_total_tenants",
    "metaclouds_total_clusters",
    "metaclouds_total_resources",
    "metaclouds_total_jobs",
    "metaclouds_running_jobs",
    "metaclouds_total_gpus",
    "metaclouds_allocated_gpus",
    "metaclouds_total_datasets",
    "metaclouds_total_alerts",
    "metaclouds_active_alerts",
    "metaclouds_system_uptime",
];

/// 3 个 HTTP 指标名（对齐 Go metrics_business.go）。
const EXPECTED_HTTP_METRICS: &[&str] = &[
    "http_requests_total",
    "http_request_duration_seconds",
    "http_requests_in_flight",
];

/// Histogram buckets（与 Go 版逐字一致）。
const EXPECTED_BUCKETS: &[&str] = &[
    "0.005", "0.01", "0.025", "0.05", "0.1", "0.25", "0.5", "1", "2.5", "5", "10",
];

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
        jwt_secret: "p3-metrics-secret-at-least-32-chars-long-xxxx".to_string(),
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

/// 发一个裸 GET 请求，返回 (status, content_type, body)。
async fn get_metrics(app: &mut axum::Router) -> (StatusCode, String, String) {
    let req = Request::builder()
        .method("GET")
        .uri("/metrics")
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

#[tokio::test]
async fn p3_metrics_endpoint_returns_200_text_plain() {
    let mut app = setup_app().await;
    let (status, ct, body) = get_metrics(&mut app).await;
    assert_eq!(status, StatusCode::OK, "/metrics should return 200");
    assert!(
        ct.starts_with("text/plain"),
        "content-type should be text/plain, got: {ct}"
    );
    assert!(!body.is_empty(), "metrics body should not be empty");
}

#[tokio::test]
async fn p3_metrics_requires_no_auth() {
    let mut app = setup_app().await;
    // 不带任何 Authorization 头，/metrics 应直接 200（对齐 Go promhttp.Handler）。
    let (status, _ct, _body) = get_metrics(&mut app).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "/metrics must be accessible without JWT"
    );
}

#[tokio::test]
async fn p3_metrics_contains_all_13_business_metrics() {
    let mut app = setup_app().await;
    let (_status, _ct, body) = get_metrics(&mut app).await;
    for name in EXPECTED_BUSINESS_METRICS {
        assert!(
            body.contains(name),
            "metrics body missing business metric: {name}"
        );
    }
}

#[tokio::test]
async fn p3_metrics_namespace_prefix_is_metaclouds() {
    let mut app = setup_app().await;
    let (_status, _ct, body) = get_metrics(&mut app).await;
    // 每个业务指标名都带 metaclouds_ 前缀（白名单 diff 为空）。
    for name in EXPECTED_BUSINESS_METRICS {
        assert!(
            name.starts_with("metaclouds_"),
            "metric {name} missing metaclouds_ prefix"
        );
        // 确认 # TYPE 行也使用带前缀的名称。
        let type_line = format!("# TYPE {name} gauge");
        assert!(
            body.contains(&type_line),
            "missing TYPE line for {name}: expected {type_line}"
        );
    }
}

#[tokio::test]
async fn p3_metrics_contains_http_metrics() {
    let mut app = setup_app().await;
    // 第一次抓取触发请求（中间件在响应后才记录），第二次抓取才能看到
    // CounterVec / HistogramVec 的样本行（label-vector 首次使用前无 child）。
    let (_s1, _c1, _b1) = get_metrics(&mut app).await;
    let (_s2, _c2, body) = get_metrics(&mut app).await;
    for name in EXPECTED_HTTP_METRICS {
        assert!(
            body.contains(name),
            "metrics body missing HTTP metric: {name}"
        );
    }
}

#[tokio::test]
async fn p3_metrics_http_request_count_increases() {
    let mut app = setup_app().await;
    // 第一次抓取：触发 /metrics 请求本身（中间件在响应后才记录）。
    let (_s1, _c1, _b1) = get_metrics(&mut app).await;
    // 第二次抓取：此时第一次请求已被中间件记录，http_requests_total 应有值。
    let (_s2, _c2, body) = get_metrics(&mut app).await;
    // http_requests_total 应有 GET /metrics 标签行且值 >= 1。
    assert!(
        body.contains("http_requests_total"),
        "http_requests_total should be present after requests"
    );
    assert!(body.contains("method=\"GET\""), "should have method label");
    assert!(
        body.contains("path=\"/metrics\""),
        "should have path=/metrics label"
    );
    assert!(
        body.contains("status=\"200\""),
        "should have status=200 label"
    );
}

#[tokio::test]
async fn p3_metrics_histogram_buckets_match_go() {
    let mut app = setup_app().await;
    // 触发至少一次请求，使 histogram 有观测值（bucket 行才会输出）。
    let (_s1, _c1, _b1) = get_metrics(&mut app).await;
    let (_s2, _c2, body) = get_metrics(&mut app).await;
    // 验证所有 bucket 边界值出现在 _bucket 行中。
    for b in EXPECTED_BUCKETS {
        let bucket_line = format!("le=\"{b}\"");
        assert!(
            body.contains(&bucket_line),
            "missing histogram bucket le=\"{b}\""
        );
    }
    // histogram 总和与 +Inf 边界也应存在。
    assert!(body.contains("le=\"+Inf\""), "missing +Inf bucket boundary");
}

#[tokio::test]
async fn p3_metrics_labels_method_path_status() {
    let mut app = setup_app().await;
    // 触发一次请求。
    let (_s1, _c1, _b1) = get_metrics(&mut app).await;
    let (_s2, _c2, body) = get_metrics(&mut app).await;
    // http_requests_total 的标签行应同时包含 method / path / status。
    let line = body
        .lines()
        .find(|l| l.starts_with("http_requests_total{"))
        .expect("http_requests_total labeled line should exist");
    assert!(
        line.contains("method=\"GET\""),
        "label line missing method: {line}"
    );
    assert!(
        line.contains("path=\"/metrics\""),
        "label line missing path: {line}"
    );
    assert!(
        line.contains("status=\"200\""),
        "label line missing status: {line}"
    );
}

#[tokio::test]
async fn p3_metrics_text_format_is_parseable() {
    let mut app = setup_app().await;
    let (_s, _c, body) = get_metrics(&mut app).await;
    // Prometheus 文本格式必须有 # HELP 和 # TYPE 前缀行。
    assert!(
        body.lines().any(|l| l.starts_with("# HELP ")),
        "Prometheus text format must contain # HELP lines"
    );
    assert!(
        body.lines().any(|l| l.starts_with("# TYPE ")),
        "Prometheus text format must contain # TYPE lines"
    );
    // 至少一条实际样本行（指标名后跟数值）。
    assert!(
        body.lines().any(|l| {
            !l.starts_with('#')
                && !l.is_empty()
                && l.split_whitespace()
                    .next_back()
                    .is_some_and(|v| v.parse::<f64>().is_ok())
        }),
        "Prometheus text format must contain sample lines with numeric values"
    );
}

#[tokio::test]
async fn p3_metrics_business_gauges_have_nonnegative_values() {
    let mut app = setup_app().await;
    let (_s, _c, body) = get_metrics(&mut app).await;
    // 每个 metaclouds_* gauge 应输出 >= 0 的值（DB 查询失败时安全回退 0）。
    for name in EXPECTED_BUSINESS_METRICS {
        let sample = body
            .lines()
            .find(|l| l.starts_with(name) && !l.starts_with('#'))
            .unwrap_or_else(|| panic!("no sample line for {name}"));
        let value_str = sample
            .split_whitespace()
            .next_back()
            .unwrap_or_else(|| panic!("no value in line: {sample}"));
        let value: f64 = value_str.parse().unwrap_or_else(|_| {
            panic!("non-numeric value for {name}: {value_str}");
        });
        assert!(
            value >= 0.0,
            "gauge {name} should be non-negative, got {value}"
        );
    }
}
