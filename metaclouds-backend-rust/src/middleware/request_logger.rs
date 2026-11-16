//! 请求日志中间件。
//!
//! 在响应返回后以结构化字段记录一次请求的概要，字段对齐 Go 侧
//! `middlewares/request_logger.go`：method、path、status_code、duration_ms、
//! client_ip、request_id，若请求扩展中已注入 JWT Claims 则附带 user_id。
//!
//! 本中间件必须排在 `request_id` 之后运行，才能在日志中关联到请求 ID；
//! 它运行在路由级 `jwt_auth` 之外，因此 user_id 通常为空（仅在把本中间件
//! 挂在认证层内侧时才会有值），按“如果有则记录”处理。

use std::sync::OnceLock;
use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::jwt::Claims;
use crate::middleware::request_id::RequestId;

/// 解析客户端 IP。
///
/// 安全：审计日志曾无条件取 `X-Forwarded-For` 首段，任何人都能伪造源 IP
/// 污染审计记录（追责时无法定位真实来源）。现在与限流共用同一套判定：
/// 仅当直连对端落在 `TRUSTED_PROXIES`（可信反代/Ingress 网段）内才采信该头，
/// 详见 [`crate::middleware::client_ip`]。
fn client_ip(request: &Request) -> String {
    crate::middleware::client_ip::resolve(request, crate::middleware::client_ip::trusted_from_env())
}

/// 慢请求告警阈值（毫秒）。此前 `SLOW_REQUEST_THRESHOLD_MS` 只在 config 中定义/
/// 加载、从未被消费，生产无法通过它发现长尾请求。启动时由 `main` 注入；未注入时
/// 使用保守默认 2000ms，测试/库调用方无需任何改动。
static SLOW_REQUEST_THRESHOLD_MS: OnceLock<u64> = OnceLock::new();

/// 注入慢请求阈值（进程级，仅首次生效）。
pub fn configure_slow_request_threshold(ms: u64) {
    let _ = SLOW_REQUEST_THRESHOLD_MS.set(ms);
}

/// 中间件：计时并在响应后输出结构化访问日志。
pub async fn request_logger(request: Request, next: Next) -> Response {
    let start = Instant::now();

    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let client_ip = client_ip(&request);
    // 在消费 request 前，尽力取出请求 ID 与（可能已注入的）用户 ID / 租户 ID。
    let request_id = request.extensions().get::<RequestId>().map(|r| r.0.clone());
    let user_id = request.extensions().get::<Claims>().map(|c| c.user_id);
    let tenant_id = request.extensions().get::<Claims>().map(|c| c.tenant_id);

    let response = next.run(request).await;

    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    let status_code = response.status().as_u16();

    tracing::info!(
        method = %method,
        path = %path,
        status_code = status_code,
        duration_ms = duration_ms,
        client_ip = %client_ip,
        user_id = user_id,
        tenant_id = tenant_id,
        request_id = request_id.as_deref().unwrap_or("-"),
        "request completed"
    );

    // 超过阈值额外打 warn，便于生产告警规则直接按 "slow request" + threshold_ms 匹配。
    // 与上面 info 日志共用同一请求上下文，不新增依赖。
    let threshold_ms = *SLOW_REQUEST_THRESHOLD_MS.get().unwrap_or(&2000);
    if duration_ms > threshold_ms as f64 {
        tracing::warn!(
            method = %method,
            path = %path,
            status_code = status_code,
            duration_ms = duration_ms,
            threshold_ms = threshold_ms,
            request_id = request_id.as_deref().unwrap_or("-"),
            "slow request"
        );
    }

    response
}
