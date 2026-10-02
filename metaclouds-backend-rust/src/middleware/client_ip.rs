//! 客户端真实 IP 解析（横切）。
//!
//! `X-Forwarded-For` 是**客户端可伪造**的请求头。若无条件取其首段，攻击者
//! 只要每次请求换一个 XFF 就能绕过按 IP 的限流，也会污染访问审计日志。
//!
//! 正确做法：只有当**直连对端**（TCP 连接的 peer 地址）位于 `TRUSTED_PROXIES`
//! （可信反向代理 / Ingress 网段）之内时，才采信 XFF；否则一律使用直连 IP。
//!
//! peer 地址来自 axum 的 `ConnectInfo` 扩展（由
//! `into_make_service_with_connect_info::<SocketAddr>()` 注入）；在本机单测等
//! 未注入该扩展的场景下退化为 `Option::None`，此时保守地不采信 XFF。

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Mutex, OnceLock};

use axum::extract::ConnectInfo;
use axum::http::HeaderMap;
use ipnet::IpNet;

/// `TRUSTED_PROXIES` 解析结果的进程级缓存。
///
/// 环境变量解析本身很便宜，但限流 / 访问日志每个请求都要取一次；用
/// `OnceLock` 固化一次，避免热路径上反复 split 字符串。
static TRUSTED: OnceLock<Vec<String>> = OnceLock::new();

/// 读取并缓存 `TRUSTED_PROXIES`（逗号分隔，支持 `1.2.3.4` 与 `10.0.0.0/8`）。
pub fn trusted_from_env() -> &'static [String] {
    TRUSTED
        .get_or_init(|| {
            std::env::var("TRUSTED_PROXIES")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .as_slice()
}

/// 同 [`trusted_from_env`]，但允许指定环境变量名（指标端点等独立白名单）。
pub fn trusted_from_env_key(key: &str) -> Vec<String> {
    static SLOTS: OnceLock<Mutex<HashMap<String, Vec<String>>>> = OnceLock::new();
    let slots = SLOTS.get_or_init(Mutex::default);
    let mut guard = slots.lock().unwrap_or_else(|e| e.into_inner());
    guard
        .entry(key.to_string())
        .or_insert_with(|| {
            std::env::var(key)
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .clone()
}

/// 判定给定 IP 是否落在 `trusted` 列表内（供 /metrics 等端点复用）。
pub fn ip_is_trusted(peer: Option<IpAddr>, trusted: &[String]) -> bool {
    peer_is_trusted(peer, trusted)
}

/// 判定直连对端是否位于可信代理列表内。
///
/// 列表项支持两种写法：精确 IP（`10.0.0.1`）或 CIDR（`10.0.0.0/8`）；
/// 无法解析的条目忽略（不因此放行）。
fn peer_is_trusted(peer: Option<IpAddr>, trusted: &[String]) -> bool {
    let Some(peer) = peer else {
        // 拿不到直连地址时一律不信任，避免伪造头生效。
        return false;
    };
    trusted.iter().any(|entry| {
        let entry = entry.trim();
        if entry.is_empty() {
            return false;
        }
        if let Ok(net) = entry.parse::<IpNet>() {
            net.contains(&peer)
        } else if let Ok(ip) = entry.parse::<IpAddr>() {
            ip == peer
        } else {
            false
        }
    })
}

/// 从请求中解析客户端 IP。
///
/// - `trusted` 为 `TRUSTED_PROXIES` 配置值；
/// - 对端可信且有合法 XFF → 取 XFF 首段；
/// - 否则 → 直连 IP（拿不到则 `"unknown"`）。
pub fn resolve(request: &axum::http::Request<axum::body::Body>, trusted: &[String]) -> String {
    let peer = request
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip());

    if peer_is_trusted(peer, trusted) {
        if let Some(ip) = forwarded_for(request.headers()).and_then(|s| s.parse::<IpAddr>().ok()) {
            return ip.to_string();
        }
    }

    peer.map(|p| p.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// 取 `X-Forwarded-For` 的首段（已 trim）。
fn forwarded_for(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::net::SocketAddr;

    fn req_with(xff: Option<&str>, peer: Option<&str>) -> Request<Body> {
        let mut r = Request::builder().uri("/").body(Body::empty()).unwrap();
        if let Some(x) = xff {
            r.headers_mut()
                .insert("x-forwarded-for", x.parse().unwrap());
        }
        if let Some(p) = peer {
            r.extensions_mut()
                .insert(ConnectInfo(SocketAddr::new(p.parse().unwrap(), 1234)));
        }
        r
    }

    #[test]
    fn untrusted_peer_cannot_spoof_xff() {
        let trusted = vec!["10.0.0.0/8".to_string()];
        // 直连 203.0.113.9 不在可信网段，XFF 必须被忽略
        let r = req_with(Some("1.2.3.4"), Some("203.0.113.9"));
        assert_eq!(resolve(&r, &trusted), "203.0.113.9");
    }

    #[test]
    fn trusted_peer_xff_is_used() {
        let trusted = vec!["10.0.0.0/8".to_string()];
        let r = req_with(Some("1.2.3.4, 10.0.0.1"), Some("10.0.0.1"));
        assert_eq!(resolve(&r, &trusted), "1.2.3.4");
    }

    #[test]
    fn no_connect_info_never_trusts_xff() {
        let trusted = vec!["10.0.0.0/8".to_string()];
        let r = req_with(Some("1.2.3.4"), None);
        assert_eq!(resolve(&r, &trusted), "unknown");
    }

    #[test]
    fn empty_trust_list_uses_peer() {
        let r = req_with(Some("1.2.3.4"), Some("198.51.100.7"));
        assert_eq!(resolve(&r, &[]), "198.51.100.7");
    }
}
