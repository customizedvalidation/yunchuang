//! 横切 HTTP 中间件族。
//!
//! 复刻 Go 侧 `middlewares/stack.go` 的 `ApplyCoreStack`：把请求 ID、访问日志、
//! 计时、安全头、统一错误处理、panic 兜底按固定顺序组合成一条中间件栈。
//!
//! axum 的 `.layer()` 是“后注册者位于最外层”，因此这里按期望运行时顺序
//! 的逆序调用，使得请求的实际流向为：
//!
//! ```text
//! request_id → request_logger → timing → security_headers → error_handler → panic_recover → handler
//! ```

use axum::middleware::from_fn;
use axum::Router;

pub mod error_handler;
pub mod panic_recover;
pub mod request_id;
pub mod request_logger;
pub mod security_headers;
pub mod timing;

/// 把核心中间件栈应用到给定路由器，返回新路由器。
///
/// 对齐 Go `ApplyCoreStack`：在最外层关联请求 ID 与访问日志，逐层向内
/// 计时、加安全头、把路由错误改写为信封，最后由 panic 兜底包住 handler。
///
/// 泛型参数 `S` 保留路由器的状态类型，使得 `.with_state()` 可在栈后调用。
pub fn apply_core_stack<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    router
        // 先注册者更靠近 handler（最内层）：panic 兜底包在最里层，直接守护 handler。
        .layer(from_fn(panic_recover::catch_panics))
        .layer(from_fn(error_handler::error_handler))
        .layer(from_fn(security_headers::security_headers))
        .layer(from_fn(timing::timing))
        .layer(from_fn(request_logger::request_logger))
        // 最后注册者位于最外层：最先拿到请求，分配并透传请求 ID。
        .layer(from_fn(request_id::set_request_id))
}
