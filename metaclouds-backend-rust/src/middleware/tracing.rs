//! 链路追踪 HTTP 中间件：trace_id 贯穿日志、响应头与上游上下文。
//!
//! 对齐 Go 版 `pkg/tracing` 的 `GinMiddleware` 与 W3C TraceContext：
//! - 优先从 `traceparent` 请求头解析上游传入的 trace_id（格式
//!   `00-<32hex trace_id>-<16hex span_id>-<2hex flags>`）；
//! - 其次兼容 Go 版的 `X-Trace-Id` 请求头；
//! - 都没有时生成一个新的 32 位 hex trace_id。
//!
//! trace_id / span_id 同时注入：
//! - 当前 tracing span（下游所有日志均可关联）；
//! - 响应头 `X-Trace-Id`（对齐 Go 版，供客户端关联日志）；
//! - 请求扩展 [`TraceContext`]（handler 可直接读取）。
//!
//! 本中间件必须排在 `request_id` 之后、`error_handler` 之前，保证 request_id
//! 可与 trace_id 关联，且错误日志也能带上 trace_id。

use std::time::Instant;

use axum::extract::Request;
use axum::http::header::HeaderName;
use axum::middleware::Next;
use axum::response::Response;
use tracing::Instrument;

use crate::middleware::request_id::RequestId;

static TRACE_ID_HEADER: HeaderName = HeaderName::from_static("x-trace-id");
static TRACEPARENT_HEADER: HeaderName = HeaderName::from_static("traceparent");

/// 供 handler 提取的请求级 trace 上下文。
#[derive(Debug, Clone)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
}

/// 判断字符是否为 hex 字符。
fn is_hex_char(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

/// 校验字符串是否为指定长度的 hex。
fn is_hex(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(is_hex_char)
}

/// 把字节切片转成小写 hex 字符串。
fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

/// 生成新的 trace_id（16 随机字节 → 32 hex，W3C TraceId 长度）。
pub fn generate_trace_id() -> String {
    let mut bytes = [0u8; 16];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    to_hex(&bytes)
}

/// 生成新的 span_id（8 随机字节 → 16 hex，W3C SpanId 长度）。
pub fn generate_span_id() -> String {
    let mut bytes = [0u8; 8];
    rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut bytes);
    to_hex(&bytes)
}

/// 解析 W3C `traceparent` 头，返回 trace_id（32 hex）。
///
/// 格式：`version(2)-trace-id(32)-parent-id(16)-trace-flags(2)`。
/// 非法版本、长度不对、全零 trace_id 均视为无上游上下文。
fn parse_traceparent(value: &str) -> Option<String> {
    let parts: Vec<&str> = value.split('-').collect();
    if parts.len() != 4 {
        return None;
    }
    if parts[0] != "00" {
        return None;
    }
    let trace_id = parts[1];
    if !is_hex(trace_id, 32) || trace_id == "00000000000000000000000000000000" {
        return None;
    }
    Some(trace_id.to_string())
}

/// 从请求头解析上游 trace_id：先 W3C `traceparent`，再兼容 `X-Trace-Id`。
fn extract_trace_id(request: &Request) -> Option<String> {
    if let Some(tp) = request
        .headers()
        .get(&TRACEPARENT_HEADER)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(tid) = parse_traceparent(tp) {
            return Some(tid);
        }
    }
    request
        .headers()
        .get(&TRACE_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| is_hex(s, 32))
        .filter(|s| *s != "00000000000000000000000000000000")
        .map(|s| s.to_lowercase())
}

/// 中间件：为请求建立 trace 上下文并贯穿到日志与响应头。
pub async fn trace_middleware(mut request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // 上游传入或新生成 trace_id；span_id 总是新生成。
    let trace_id = extract_trace_id(&request).unwrap_or_else(generate_trace_id);
    let span_id = generate_span_id();

    // request_id 在本中间件外侧已由 request_id 中间件注入（栈序保证）。
    let request_id = request
        .extensions()
        .get::<RequestId>()
        .map(|r| r.0.clone())
        .unwrap_or_else(|| "-".to_string());

    // 把 trace 上下文放进扩展，供 handler / 下游中间件读取。
    request.extensions_mut().insert(TraceContext {
        trace_id: trace_id.clone(),
        span_id: span_id.clone(),
    });

    let span = tracing::info_span!(
        "http.request",
        trace_id = %trace_id,
        span_id = %span_id,
        request_id = %request_id,
        method = %method,
        path = %path,
        status = tracing::field::Empty,
        duration_ms = tracing::field::Empty,
    );

    let start = Instant::now();
    // .instrument 让整个 handler 调用链都处于该 span 内，
    // 下游 request_logger / error_handler 发出的日志都会关联 trace_id。
    let mut response = next.run(request).instrument(span.clone()).await;
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    let status = response.status().as_u16();
    span.record("status", status as i64);
    span.record("duration_ms", duration_ms);

    // 响应头回写 trace_id（对齐 Go 版 X-Trace-ID）。
    if let Ok(value) = trace_id.parse() {
        response
            .headers_mut()
            .insert(TRACE_ID_HEADER.clone(), value);
    }

    // 结构化完成日志：trace_id 作为顶层字段出现（对齐 Go logEntry）。
    tracing::info!(
        trace_id = %trace_id,
        span_id = %span_id,
        request_id = %request_id,
        method = %method,
        path = %path,
        status = status,
        duration_ms = duration_ms,
        "request handled"
    );

    response
}
