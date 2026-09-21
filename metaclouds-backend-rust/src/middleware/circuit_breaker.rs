//! 熔断器中间件（简化版状态机）。
//!
//! 对齐任务要求的三态语义：
//! - `Closed`（正常）：放行请求并统计失败；
//! - `Open`（熔断）：直接拒绝，下游故障期间不再打过去；
//! - `HalfOpen`（半开探测）：冷却时间过后放行少量探测请求，成功则恢复
//!   `Closed`，失败则回到 `Open`。
//!
//! 实现约束（按任务要求）：
//! - 不引入新 crate，内存态计数，不持久化；
//! - Rust 版当前下游（K8s 等）走 mock，无真实外部调用，因此中间件主要为
//!   未来扩展预留；核心价值是可单元测试的状态机 [`CircuitBreaker`]。
//! - 中间件 **默认禁用**（`CIRCUIT_BREAKER_ENABLED` 未设或非 `true` 时直接
//!   放行），避免影响现有 296 个测试；熔断打开时返回
//!   `503 CIRCUIT_BREAKER_OPEN`。

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::AppError;

/// 熔断器状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// 正常放行，统计失败。
    Closed,
    /// 熔断打开，直接拒绝。
    Open,
    /// 半开：放行少量探测请求。
    HalfOpen,
}

/// 内存态熔断器（按进程全局单例）。
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// 连续失败达到该阈值后打开熔断。
    failure_threshold: u32,
    /// 打开后保持拒绝的冷却时长，到期转半开。
    open_timeout: Duration,
    state: State,
    failures: u32,
    opened_at: Option<Instant>,
}

impl CircuitBreaker {
    /// 构造一个关闭态熔断器。
    pub fn new(failure_threshold: u32, open_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            open_timeout,
            state: State::Closed,
            failures: 0,
            opened_at: None,
        }
    }

    /// 当前状态（不触发转换）。
    pub fn state(&self) -> State {
        self.state
    }

    /// 当前连续失败计数（诊断/测试用）。
    pub fn failures(&self) -> u32 {
        self.failures
    }

    /// 是否允许本次调用进入下游。
    ///
    /// - `Closed`：允许；
    /// - `Open`：若距打开已超过 `open_timeout`，转为 `HalfOpen` 并放行一次探测；
    ///   否则拒绝；
    /// - `HalfOpen`：放行探测。
    pub fn call_permitted(&mut self, now: Instant) -> bool {
        match self.state {
            State::Closed => true,
            State::HalfOpen => true,
            State::Open => match self.opened_at {
                Some(opened) if now.duration_since(opened) >= self.open_timeout => {
                    self.state = State::HalfOpen;
                    true
                }
                _ => false,
            },
        }
    }

    /// 记录一次成功：半开探测成功则恢复关闭并清零失败；关闭态也清零失败计数。
    pub fn record_success(&mut self, _now: Instant) {
        match self.state {
            State::HalfOpen | State::Closed => {
                self.state = State::Closed;
                self.failures = 0;
                self.opened_at = None;
            }
            State::Open => { /* 熔断期不接受成功回调 */ }
        }
    }

    /// 记录一次失败。
    ///
    /// - `Closed`：失败计数累加，达到阈值则打开熔断；
    /// - `HalfOpen`：探测失败，立刻回到 `Open` 并重置冷却起点；
    /// - `Open`：保持打开。
    pub fn record_failure(&mut self, now: Instant) {
        match self.state {
            State::Closed => {
                self.failures += 1;
                if self.failures >= self.failure_threshold {
                    self.trip_open(now);
                }
            }
            State::HalfOpen => {
                // 探测失败：重新打开并重置冷却计时。
                self.trip_open(now);
            }
            State::Open => { /* 保持 */ }
        }
    }

    fn trip_open(&mut self, now: Instant) {
        self.state = State::Open;
        self.opened_at = Some(now);
    }
}

/// 读取熔断开关：仅 `CIRCUIT_BREAKER_ENABLED=true`（或 `1`）时启用。默认关闭。
fn enabled() -> bool {
    matches!(
        std::env::var("CIRCUIT_BREAKER_ENABLED").ok().as_deref(),
        Some("true") | Some("1")
    )
}

fn config() -> (u32, Duration) {
    let threshold = std::env::var("CIRCUIT_BREAKER_THRESHOLD")
        .ok()
        .and_then(|v| v.parse::<u32>().ok())
        .unwrap_or(10);
    let secs = std::env::var("CIRCUIT_BREAKER_TIMEOUT_SECONDS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30);
    (threshold, Duration::from_secs(secs))
}

fn global() -> &'static Mutex<CircuitBreaker> {
    static CB: OnceLock<Mutex<CircuitBreaker>> = OnceLock::new();
    CB.get_or_init(|| {
        let (t, to) = config();
        Mutex::new(CircuitBreaker::new(t, to))
    })
}

/// 熔断中间件：开启后据下游响应码记录成功/失败，熔断打开时短路 503。
pub async fn circuit_breaker_middleware(request: Request, next: Next) -> Response {
    // 默认关闭：未显式开启时直接放行，保证现有测试零回归。
    if !enabled() {
        return next.run(request).await;
    }

    let permitted = {
        let now = Instant::now();
        let mut cb = global().lock().unwrap_or_else(|e| e.into_inner());
        cb.call_permitted(now)
    };

    if !permitted {
        tracing::warn!("circuit breaker open - rejecting request");
        return AppError::circuit_breaker_open("service temporarily unavailable").into_response();
    }

    let response = next.run(request).await;

    // 5xx 视为下游失败；其余视为成功。
    let is_failure = response.status().is_server_error();
    {
        let now = Instant::now();
        let mut cb = global().lock().unwrap_or_else(|e| e.into_inner());
        if is_failure {
            cb.record_failure(now);
        } else {
            cb.record_success(now);
        }
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_opens_after_threshold_failures() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        let now = Instant::now();
        assert_eq!(cb.state(), State::Closed);
        cb.record_failure(now);
        cb.record_failure(now);
        assert_eq!(cb.state(), State::Closed); // 第 2 次未达阈值
        cb.record_failure(now);
        assert_eq!(cb.state(), State::Open); // 第 3 次打开
                                             // 打开期间不放行。
        assert!(!cb.call_permitted(now));
    }

    #[test]
    fn open_half_opens_after_timeout() {
        let mut cb = CircuitBreaker::new(1, Duration::from_millis(30));
        let t0 = Instant::now();
        cb.record_failure(t0);
        assert_eq!(cb.state(), State::Open);
        assert!(!cb.call_permitted(t0)); // 未到冷却时间
                                         // 模拟时间推进：用更晚的 now。
        let t1 = t0 + Duration::from_millis(60);
        assert!(cb.call_permitted(t1)); // 冷却到，转半开
        assert_eq!(cb.state(), State::HalfOpen);
    }

    #[test]
    fn half_open_success_recovers_to_closed() {
        let mut cb = CircuitBreaker::new(1, Duration::from_millis(30));
        let t0 = Instant::now();
        cb.record_failure(t0);
        let t1 = t0 + Duration::from_millis(60);
        assert!(cb.call_permitted(t1));
        assert_eq!(cb.state(), State::HalfOpen);
        cb.record_success(t1);
        assert_eq!(cb.state(), State::Closed);
        assert_eq!(cb.failures(), 0);
        // 恢复后再次失败要重新累计阈值。
        cb.record_failure(t1);
        assert_eq!(cb.state(), State::Open);
    }

    #[test]
    fn half_open_failure_reopens() {
        let mut cb = CircuitBreaker::new(2, Duration::from_millis(30));
        let t0 = Instant::now();
        cb.record_failure(t0);
        cb.record_failure(t0);
        assert_eq!(cb.state(), State::Open);
        let t1 = t0 + Duration::from_millis(60);
        assert!(cb.call_permitted(t1));
        assert_eq!(cb.state(), State::HalfOpen);
        // 探测失败：回到 Open 并重置冷却。
        cb.record_failure(t1);
        assert_eq!(cb.state(), State::Open);
        // 刚重新打开，t1 仍在冷却内 → 拒绝。
        assert!(!cb.call_permitted(t1));
    }
}
