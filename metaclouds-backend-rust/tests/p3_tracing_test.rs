//! P3-04 tracing + OpenTelemetry 对齐集成测试。
//!
//! 自构路由（不依赖 routes.rs / JWT / CSRF），仅挂载 trace 中间件，验证：
//! - init_tracing 在 JSON 格式下不 panic；OTel disabled 时正常运行。
//! - 响应头包含 `X-Trace-Id`，且格式为 32 位 hex。
//! - `traceparent`（W3C TraceContext）上游传入时继承其中的 trace_id。
//! - 日志为 JSON 且顶层包含 `trace_id` 字段（捕获全局 subscriber 输出验证）。
//! - shutdown_tracing 优雅关闭不 panic。

use std::io;
use std::sync::{Arc, Mutex, OnceLock};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use tower::ServiceExt;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::EnvFilter;

use metaclouds_backend_rust::config::Config;
use metaclouds_backend_rust::middleware::tracing::trace_middleware;
use metaclouds_backend_rust::tracing::{init_tracing, shutdown_tracing};
use metaclouds_backend_rust::TestConfig;

// ── 捕获用 writer：把 JSON 日志写进共享 buffer，便于断言 ────────────────

#[derive(Clone, Default)]
struct Capture(Arc<Mutex<Vec<u8>>>);

impl<'a> MakeWriter<'a> for Capture {
    type Writer = LockedWriter;
    fn make_writer(&'a self) -> Self::Writer {
        LockedWriter(self.0.clone())
    }
}

struct LockedWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for LockedWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

static BUF: OnceLock<Arc<Mutex<Vec<u8>>>> = OnceLock::new();

/// 安装一个 JSON 格式的全局 subscriber，输出到共享 buffer（仅安装一次）。
/// 后续 init_tracing 会因“已设置全局 subscriber”而优雅跳过。
fn setup() {
    BUF.get_or_init(|| {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = Capture(buf.clone());
        let _ = tracing_subscriber::fmt()
            .with_writer(writer)
            .with_env_filter(EnvFilter::new("info"))
            .json()
            .with_target(true)
            .try_init();
        buf
    });
}

/// 读取已捕获的全部日志字节。
fn captured_logs() -> String {
    let buf = BUF.get().expect("capture subscriber installed");
    String::from_utf8(buf.lock().unwrap().clone()).unwrap()
}

// ── 自构测试路由 ──────────────────────────────────────────────────────

async fn ok_handler() -> (StatusCode, axum::Json<serde_json::Value>) {
    (StatusCode::OK, axum::Json(serde_json::json!({"ok": true})))
}

fn test_app() -> Router {
    Router::new()
        .route("/test", axum::routing::get(ok_handler))
        .layer(axum::middleware::from_fn(trace_middleware))
}

/// 发一个 GET /test 请求，返回响应。
async fn send_get(app: &mut Router, headers: &[(String, String)]) -> axum::response::Response {
    let mut builder = Request::builder().method("GET").uri("/test");
    for (k, v) in headers {
        builder = builder.header(k, v);
    }
    let req = builder.body(Body::empty()).unwrap();
    app.oneshot(req).await.unwrap()
}

/// 是否为 32 位 hex 字符串。
fn is_32hex(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

// ── 测试用例 ────────────────────────────────────────────────────────────

#[test]
fn p3_tracing_init_json_does_not_panic() {
    setup();
    // 默认配置：otel_enabled=false，不应连接 collector，不应 panic。
    let cfg: Config = TestConfig::default().into();
    init_tracing(&cfg).expect("init_tracing should succeed when otel disabled");
    shutdown_tracing();
}

#[test]
fn p3_tracing_otel_disabled_runs_without_collector() {
    setup();
    let mut cfg: Config = TestConfig::default().into();
    cfg.otel_enabled = false;
    // 即便 endpoint 指向一个不可达地址，disabled 分支也不应尝试连接或 panic。
    cfg.otel_endpoint = "http://127.0.0.1:1".to_string();
    init_tracing(&cfg).expect("init_tracing must not panic when otel disabled");
    shutdown_tracing();
}

#[tokio::test]
async fn p3_response_has_x_trace_id_header() {
    setup();
    let mut app = test_app();
    let resp = send_get(&mut app, &[]).await;
    assert_eq!(resp.status(), StatusCode::OK);
    let tid = resp
        .headers()
        .get("x-trace-id")
        .expect("response must contain X-Trace-Id header")
        .to_str()
        .unwrap();
    assert!(is_32hex(tid), "trace_id must be 32 hex, got: {tid}");
}

#[tokio::test]
async fn p3_trace_id_format_is_32hex() {
    setup();
    let mut app = test_app();
    // 连续两次请求，trace_id 均应为 32 位 hex。
    for _ in 0..2 {
        let resp = send_get(&mut app, &[]).await;
        let tid = resp
            .headers()
            .get("x-trace-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        assert!(is_32hex(&tid), "trace_id not 32 hex: {tid}");
        assert_ne!(tid, "00000000000000000000000000000000");
    }
}

#[tokio::test]
async fn p3_traceparent_header_is_inherited() {
    setup();
    let trace_id = "4bf92f3577b34da6a3ce929d0e0e4736";
    let parent_id = "00f067aa0ba902b7";
    let traceparent = format!("00-{trace_id}-{parent_id}-01");

    let mut app = test_app();
    let resp = send_get(&mut app, &[("traceparent".to_string(), traceparent)]).await;

    let out = resp
        .headers()
        .get("x-trace-id")
        .expect("X-Trace-Id must be present")
        .to_str()
        .unwrap();
    assert_eq!(
        out, trace_id,
        "X-Trace-Id must inherit trace_id from traceparent"
    );
}

#[tokio::test]
async fn p3_legacy_x_trace_id_header_is_respected() {
    setup();
    // 兼容 Go 版：直接传 X-Trace-Id 也应被采用（无 traceparent 时）。
    let trace_id = "0af7651916cd43dd8448eb211c80319c";
    let mut app = test_app();
    let resp = send_get(
        &mut app,
        &[("x-trace-id".to_string(), trace_id.to_string())],
    )
    .await;
    let out = resp.headers().get("x-trace-id").unwrap().to_str().unwrap();
    assert_eq!(out, trace_id);
}

#[tokio::test]
async fn p3_log_output_contains_trace_id_field() {
    setup();
    let mut app = test_app();
    let resp = send_get(&mut app, &[]).await;
    let tid = resp
        .headers()
        .get("x-trace-id")
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    // 中间件在返回响应前已同步写出 "request handled" 日志。
    let logs = captured_logs();
    let line = logs
        .lines()
        .find(|l| l.contains(&tid))
        .unwrap_or_else(|| panic!("no log line contains trace_id {tid};\nlogs=\n{logs}"));

    let v: serde_json::Value = serde_json::from_str(line)
        .unwrap_or_else(|e| panic!("log line is not valid JSON: {e}\nline={line}"));
    // tracing-subscriber JSON 把事件字段放在 `fields` 下；message 也在其中。
    let fields = v.get("fields").and_then(|f| f.as_object());
    let fields = fields.expect("JSON log missing `fields` object");
    assert!(
        fields.contains_key("trace_id"),
        "JSON log missing trace_id field: {line}"
    );
    assert_eq!(fields["trace_id"].as_str().unwrap(), tid);
    assert!(fields.contains_key("span_id"), "missing span_id: {line}");
    assert!(v.get("timestamp").is_some(), "missing timestamp: {line}");
    assert!(v.get("level").is_some(), "missing level: {line}");
}

#[test]
fn p3_shutdown_tracing_does_not_panic() {
    setup();
    // 未初始化 OTel（disabled）时调用 shutdown 不应 panic。
    shutdown_tracing();
}
