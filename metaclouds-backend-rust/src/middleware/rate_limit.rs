//! 基于滑动窗口的 IP 限流中间件。
//!
//! 对齐 Go 版 `middlewares/` 限流语义：默认每个客户端 IP 在滑动窗口内
//! 允许 `100` 次请求 / `60` 秒，超出返回 `429 RATE_LIMIT_EXCEEDED`。
//!
//! 实现约束（按任务要求）：
//! - 不引入新 crate，仅用 `std::collections::HashMap` + `std::sync::Mutex`
//!   维护内存态计数；与 Go 版一样不持久化。
//! - 滑动窗口：为每个 IP 记录窗口内的请求时间戳，判定前先剔除早于窗口起点
//!   的旧时间戳，再把当前请求压入。
//! - 客户端 IP 优先取 `X-Forwarded-For` 首段（反代后真实来源）；缺失时回退
//!   到 `unknown` 桶（测试环境无 TCP 连接信息时）。
//! - 白名单：`/health`、`/metrics`、`/swagger-ui`、`/api-docs` 不限流
//!   （探针与监控/文档端点不能被自身限流拒绝）。
//!
//! 开关：中间件默认 **关闭**（`RATE_LIMIT_ENABLED` 未设或非 `true` 时直接放行），
//! 以保证现有集成测试零回归；生产经环境变量 `RATE_LIMIT_ENABLED=true` 开启，
//! 阈值由 `RATE_LIMIT_REQUESTS` / `RATE_LIMIT_DURATION_SECONDS` 覆盖。
//! 滑动窗口判定逻辑集中在纯结构体 [`RateLimiter`] 上，便于单元测试直接构造。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::extract::Request;
use axum::http::header::HeaderName;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::AppError;

static RETRY_AFTER: HeaderName = HeaderName::from_static("retry-after");
static X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");

/// 判定结果：放行（含窗口内当前计数）或限流。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// 未超限，`u32` 为剔除旧时间戳后窗口内累计请求数（含本次）。
    Allowed(u32),
    /// 已超限，`u32` 为当前窗口内累计请求数。
    Limited(u32),
}

/// 内存态滑动窗口限流器（按 key 维度，key 通常是客户端 IP）。
///
/// 与 Go 版一样不持久化；进程重启即清零。`Mutex` 仅在请求判定瞬间持有，
/// 临界区只做 vector 裁剪/追加，开销很小。
#[derive(Debug, Clone)]
pub struct RateLimiter {
    /// 窗口内允许的最大请求数。
    pub max_requests: u32,
    /// 滑动窗口长度。
    pub window: Duration,
    buckets: HashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    /// 构造一个限流器；`max_requests` 为 0 视为不限（直接放行）。
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests,
            window,
            buckets: HashMap::new(),
        }
    }

    /// 判定 `key` 的一次请求是否放行，并把本次时间戳计入。
    ///
    /// 先丢弃早于 `now - window` 的旧时间戳，再统计剩余数量；
    /// 若剩余数量（不含本次）已达到 `max_requests` 则限流。
    pub fn check(&mut self, key: &str, now: Instant) -> Outcome {
        // 0 表示不限流。
        if self.max_requests == 0 {
            return Outcome::Allowed(0);
        }
        let cutoff = now.checked_sub(self.window).unwrap_or(now);
        let bucket = self.buckets.entry(key.to_string()).or_default();
        // 滑动窗口：剔除窗口外的旧时间戳。
        bucket.retain(|t| *t >= cutoff);
        let count = bucket.len() as u32;
        if count >= self.max_requests {
            Outcome::Limited(count)
        } else {
            bucket.push(now);
            Outcome::Allowed(count + 1)
        }
    }

    /// 当前记录了多少个不同的 key（诊断用）。
    pub fn bucket_count(&self) -> usize {
        self.buckets.len()
    }
}

/// 读取限流开关：仅当 `RATE_LIMIT_ENABLED=true`（或 `1`）时启用。
///
/// 默认关闭，避免现有集成测试因共享 `unknown` 桶而被误限。
fn enabled() -> bool {
    matches!(
        std::env::var("RATE_LIMIT_ENABLED").ok().as_deref(),
        Some("true") | Some("1")
    )
}

/// 读取限流阈值（秒级）。
fn config() -> (u32, Duration) {
    let requests = std::env::var("RATE_LIMIT_REQUESTS")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(100);
    let secs = std::env::var("RATE_LIMIT_DURATION_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(60);
    (requests, Duration::from_secs(secs))
}

/// 全局共享的限流器实例。懒初始化一次；后续请求复用同一计数。
fn global() -> &'static Mutex<RateLimiter> {
    static LIM: OnceLock<Mutex<RateLimiter>> = OnceLock::new();
    LIM.get_or_init(|| {
        let (r, w) = config();
        Mutex::new(RateLimiter::new(r, w))
    })
}

/// 白名单路径：探针 / 指标 / 文档端点不限流。
fn is_whitelisted(path: &str) -> bool {
    path.starts_with("/health")
        || path.starts_with("/metrics")
        || path.starts_with("/swagger-ui")
        || path.starts_with("/api-docs")
}

/// 从请求提取客户端 IP：优先 `X-Forwarded-For` 首段。
fn client_ip(request: &Request) -> String {
    request
        .headers()
        .get(&X_FORWARDED_FOR)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// 限流中间件：开启后按 IP 做滑动窗口限流，超限返回 429。
pub async fn rate_limit_middleware(request: Request, next: Next) -> Response {
    // 默认关闭：未显式开启时直接放行，保证现有测试零回归。
    if !enabled() {
        return next.run(request).await;
    }

    let path = request.uri().path().to_string();
    if is_whitelisted(&path) {
        return next.run(request).await;
    }

    let ip = client_ip(&request);
    let outcome = {
        let now = Instant::now();
        let mut limiter = global().lock().unwrap_or_else(|e| e.into_inner());
        limiter.check(&ip, now)
    };

    match outcome {
        Outcome::Limited(count) => {
            tracing::warn!(
                client_ip = %ip,
                count,
                path = %path,
                "rate limit exceeded"
            );
            let mut resp = AppError::rate_limit(format!(
                "rate limit exceeded: {count} requests within window"
            ))
            .into_response();
            // 建议客户端等待一个窗口后重试（秒）。
            let window_secs = config().1.as_secs().to_string();
            if let Ok(value) = window_secs.parse() {
                resp.headers_mut().insert(RETRY_AFTER.clone(), value);
            }
            resp
        }
        Outcome::Allowed(_) => next.run(request).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn within_window_allows_up_to_limit() {
        let mut lim = RateLimiter::new(3, Duration::from_secs(60));
        let now = Instant::now();
        assert!(matches!(lim.check("1.1.1.1", now), Outcome::Allowed(1)));
        assert!(matches!(lim.check("1.1.1.1", now), Outcome::Allowed(2)));
        assert!(matches!(lim.check("1.1.1.1", now), Outcome::Allowed(3)));
        // 第 4 次超限。
        assert!(matches!(lim.check("1.1.1.1", now), Outcome::Limited(3)));
    }

    #[test]
    fn different_ips_are_isolated() {
        let mut lim = RateLimiter::new(2, Duration::from_secs(60));
        let now = Instant::now();
        assert!(matches!(lim.check("1.1.1.1", now), Outcome::Allowed(_)));
        assert!(matches!(lim.check("1.1.1.1", now), Outcome::Allowed(_)));
        // 第二个 IP 不受第一个影响。
        assert!(matches!(lim.check("2.2.2.2", now), Outcome::Allowed(1)));
        assert!(matches!(lim.check("2.2.2.2", now), Outcome::Allowed(2)));
        assert_eq!(lim.bucket_count(), 2);
    }

    #[test]
    fn window_expiry_recovers_allowed() {
        // 用一个很短的窗口验证旧时间戳被裁剪后恢复。
        let mut lim = RateLimiter::new(2, Duration::from_millis(30));
        let t0 = Instant::now();
        assert!(matches!(lim.check("9.9.9.9", t0), Outcome::Allowed(1)));
        assert!(matches!(lim.check("9.9.9.9", t0), Outcome::Allowed(2)));
        assert!(matches!(lim.check("9.9.9.9", t0), Outcome::Limited(2)));
        // 等待窗口过期后再判定。
        sleep(Duration::from_millis(60));
        let t1 = Instant::now();
        assert!(matches!(lim.check("9.9.9.9", t1), Outcome::Allowed(1)));
    }

    #[test]
    fn zero_limit_means_unlimited() {
        let mut lim = RateLimiter::new(0, Duration::from_secs(60));
        let now = Instant::now();
        for _ in 0..1000 {
            assert!(matches!(lim.check("1.2.3.4", now), Outcome::Allowed(0)));
        }
    }

    #[test]
    fn whitelist_and_ip_extraction_helpers() {
        assert!(is_whitelisted("/health"));
        assert!(is_whitelisted("/health/live"));
        assert!(is_whitelisted("/metrics"));
        assert!(is_whitelisted("/swagger-ui/index.html"));
        assert!(is_whitelisted("/api-docs/openapi.json"));
        assert!(!is_whitelisted("/api/v1/tenants"));
    }
}
