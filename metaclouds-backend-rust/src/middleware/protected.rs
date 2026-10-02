//! 运维端点保护：`/metrics` 与 Swagger 文档的生产鉴权。
//!
//! 背景：这两个端点为了兼容 Prometheus 抓取与本地调试，历史上完全不鉴权。
//! 生产环境下 `/metrics` 会暴露请求量、错误率、GPU/作业等业务指标，
//! `/swagger-ui` 与 `/api-docs/openapi.json` 会暴露全部接口签名——两者都
//! 属于“内部信息”，公网可达即为信息泄露面。
//!
//! 策略（默认最小惊讶 + 失败关闭）：
//! - 非生产环境（`SERVER_ENV != production`）完全放行，保持本地/CI 体验不变；
//! - 生产环境放行条件二选一：
//!   1. 直连对端落在 `METRICS_TRUSTED_CIDRS`（缺省回退 `TRUSTED_PROXIES`）内
//!      —— 集群内 Prometheus / Ingress 网段直连，或 `kubectl port-forward`；
//!   2. 请求带 `Authorization: Bearer <token>` 或 `X-Api-Token: <token>`，
//!      且以常数时间比较等于 `PROTECTED_ENDPOINTS_TOKEN`；
//! - 生产环境若既未配置 token 又不在可信网段，返回
//!   `503 SERVICE_UNAVAILABLE`（配置缺失视为不可对外服务），不静默放行。
//!
//! Prometheus 侧对应配置（`prometheus.yml`）：
//! ```yaml
//! scrape_configs:
//!   - job_name: metaclouds-backend
//!     authorization:
//!       type: Bearer
//!       credentials: <PROTECTED_ENDPOINTS_TOKEN>
//! ```

use std::sync::Arc;

use axum::extract::ConnectInfo;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::error::{AppError, ErrorCode};
use crate::middleware::client_ip;

/// 运维端点守卫配置，作为中间件 state 注入。
#[derive(Clone, Debug)]
pub struct EndpointGuard {
    /// 是否启用保护（生产环境自动启用）。
    pub enabled: bool,
}

impl EndpointGuard {
    /// 按运行环境构造守卫：`SERVER_ENV=production`（忽略大小写）时启用。
    pub fn from_environment(environment: &str) -> Self {
        Self {
            enabled: environment.eq_ignore_ascii_case("production"),
        }
    }
}

/// 中间件：生产环境下保护 `/metrics`、`/swagger-ui`、`/api-docs`。
pub async fn protect_endpoints(
    axum::extract::State(guard): axum::extract::State<Arc<EndpointGuard>>,
    request: Request,
    next: Next,
) -> Response {
    if !guard.enabled {
        return next.run(request).await;
    }

    let peer = request
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip());

    // 1) 集群内可信网段（Prometheus / Ingress / port-forward）直连放行。
    let cidrs = trusted_cidrs();
    if client_ip::ip_is_trusted(peer, &cidrs) {
        return next.run(request).await;
    }

    // 2) 显式 token：Bearer 或 X-Api-Token。
    let expected = std::env::var("PROTECTED_ENDPOINTS_TOKEN").unwrap_or_default();
    if !expected.is_empty() {
        let ok = presented_token(request.headers())
            .map(|t| constant_time_eq(t.as_bytes(), expected.as_bytes()))
            .unwrap_or(false);
        if ok {
            return next.run(request).await;
        }
        return AppError::new(ErrorCode::Unauthorized, "invalid or missing endpoint token")
            .into_response();
    }

    // 3) 生产环境未配置 token 且来源不可信 → 失败关闭，绝不静默放行。
    tracing::error!(
        peer = ?peer,
        "protected endpoint requested in production but PROTECTED_ENDPOINTS_TOKEN is unset \
         and peer is not within METRICS_TRUSTED_CIDRS; refusing"
    );
    (
        StatusCode::SERVICE_UNAVAILABLE,
        "protected endpoint is not configured for this deployment",
    )
        .into_response()
}

/// 可信抓取网段：`METRICS_TRUSTED_CIDRS`，缺省回退 `TRUSTED_PROXIES`。
fn trusted_cidrs() -> Vec<String> {
    let explicit = client_ip::trusted_from_env_key("METRICS_TRUSTED_CIDRS");
    if explicit.is_empty() {
        client_ip::trusted_from_env().to_vec()
    } else {
        explicit
    }
}

/// 从 `Authorization: Bearer ...`（优先）或 `X-Api-Token` 取出令牌。
fn presented_token(headers: &axum::http::HeaderMap) -> Option<String> {
    if let Some(v) = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        if let Some(rest) = v
            .strip_prefix("Bearer ")
            .or_else(|| v.strip_prefix("bearer "))
        {
            let t = rest.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    headers
        .get("x-api-token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// 常数时间字符串比较，避免通过响应耗时逐字节爆破 token。
///
/// 长度不等时仍会走一遍与等长路径相同的循环（用自身比较）后再返回 false，
/// 使得分支耗时不随“前 N 字节是否匹配”线性变化。
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let max = a.len().max(b.len());
    let mut acc = a.len() ^ b.len();
    for i in 0..max {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        acc |= (x ^ y) as usize;
    }
    acc == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guard_enabled_only_in_production() {
        assert!(!EndpointGuard::from_environment("development").enabled);
        assert!(!EndpointGuard::from_environment("staging").enabled);
        assert!(EndpointGuard::from_environment("production").enabled);
        assert!(EndpointGuard::from_environment("Production").enabled);
    }

    #[test]
    fn constant_time_eq_matches_semantics() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
        assert!(!constant_time_eq(b"", b"a"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn parses_bearer_and_x_api_token() {
        let mut h = axum::http::HeaderMap::new();
        h.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer s3cret".parse().unwrap(),
        );
        assert_eq!(presented_token(&h).as_deref(), Some("s3cret"));

        let mut h2 = axum::http::HeaderMap::new();
        h2.insert("x-api-token", " t2 ".parse().unwrap());
        assert_eq!(presented_token(&h2).as_deref(), Some("t2"));

        let h3 = axum::http::HeaderMap::new();
        assert_eq!(presented_token(&h3), None);
    }
}
