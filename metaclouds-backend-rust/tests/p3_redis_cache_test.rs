//! P3-01 Redis 缓存层集成测试。
//!
//! 本机无 Redis，全部用例走 NoopCache / 纯函数断言 / 不可达地址降级路径，
//! 不依赖真实 Redis 服务。`build_cache` 在不可达地址上会优雅降级到 NoopCache。

use std::sync::Arc;
use std::time::Duration;

use metaclouds_backend_rust::cache::{
    build_cache, with_prefix, Cache, CacheStats, NoopCache, SessionStore, PREFIX,
};
use metaclouds_backend_rust::config::Config;

/// 构造一个仅用于 Redis 开关测试的 Config（不跑 validate）。
fn config_with(redis_enabled: bool, redis_url: &str) -> Config {
    Config {
        redis_enabled,
        redis_url: redis_url.to_string(),
        ..Config::default()
    }
}

// ---- NoopCache 行为 -------------------------------------------------------

#[tokio::test]
async fn noop_cache_get_always_none() {
    let cache = NoopCache;
    assert!(cache
        .get("anything")
        .await
        .expect("noop get must not error")
        .is_none());
}

#[tokio::test]
async fn noop_cache_set_is_noop() {
    let cache = NoopCache;
    cache
        .set("k", b"v".to_vec(), Duration::from_secs(60))
        .await
        .expect("noop set must not error");
    // 写后读仍为空。
    assert!(cache.get("k").await.unwrap().is_none());
}

#[tokio::test]
async fn noop_cache_delete_is_noop() {
    let cache = NoopCache;
    cache.delete("k").await.expect("noop delete must not error");
}

#[tokio::test]
async fn noop_cache_exists_always_false() {
    let cache = NoopCache;
    assert!(!cache.exists("k").await.unwrap());
    assert!(cache.is_noop(), "NoopCache must report is_noop()=true");
}

// ---- build_cache 开关与降级 ----------------------------------------------

#[tokio::test]
async fn build_cache_disabled_returns_noop() {
    let cfg = config_with(false, "redis://localhost:6379/0");
    let cache = build_cache(&cfg).await;
    assert!(cache.is_noop(), "disabled redis must yield NoopCache");
}

#[tokio::test]
async fn build_cache_unreachable_redis_degrades_to_noop() {
    // 端口 1 在本机几乎必然关闭，连接应立即被拒绝 → 优雅降级。
    let cfg = config_with(true, "redis://127.0.0.1:1/0");
    let cache = build_cache(&cfg).await;
    assert!(
        cache.is_noop(),
        "unreachable redis must degrade to NoopCache, not panic"
    );
    // 降级后行为正常：get 不 panic、不报错。
    assert!(cache.get("probe").await.unwrap().is_none());
}

#[tokio::test]
async fn build_cache_invalid_url_degrades_to_noop() {
    // 非 redis scheme，Client::open 解析失败 → 降级。
    let cfg = config_with(true, "http://localhost:6379");
    let cache = build_cache(&cfg).await;
    assert!(
        cache.is_noop(),
        "invalid redis url must degrade to NoopCache"
    );
}

// ---- key 前缀 -------------------------------------------------------------

#[test]
fn with_prefix_adds_namespace() {
    assert_eq!(with_prefix("session:abc"), "metaclouds:session:abc");
    assert_eq!(with_prefix("token:xyz"), "metaclouds:token:xyz");
    assert_eq!(PREFIX, "metaclouds:");
}

#[test]
fn with_prefix_does_not_double_prefix() {
    assert_eq!(
        with_prefix("metaclouds:session:abc"),
        "metaclouds:session:abc"
    );
}

// ---- 命中率统计 -----------------------------------------------------------

#[test]
fn cache_stats_hit_rate_math() {
    let stats = CacheStats { hits: 7, misses: 3 };
    assert!((stats.hit_rate() - 0.7).abs() < f64::EPSILON);
    assert_eq!(stats.hits, 7);
    assert_eq!(stats.misses, 3);
}

#[test]
fn cache_stats_empty_hit_rate_is_one() {
    let stats = CacheStats { hits: 0, misses: 0 };
    assert!((stats.hit_rate() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn noop_cache_stats_are_zero() {
    use metaclouds_backend_rust::cache::CacheMetrics;
    let s = NoopCache.snapshot();
    assert_eq!(s.hits, 0);
    assert_eq!(s.misses, 0);
}

// ---- TTL 转换 -------------------------------------------------------------

#[test]
fn ttl_seconds_conversion() {
    use metaclouds_backend_rust::cache::redis::ttl_seconds;
    assert_eq!(ttl_seconds(Duration::from_secs(3600)), 3600);
    assert_eq!(ttl_seconds(Duration::from_secs(0)), 0);
}

// ---- 会话 jti 路径 --------------------------------------------------------

#[tokio::test]
async fn session_store_jti_lifecycle_on_noop() {
    let cache: Arc<dyn Cache> = Arc::new(NoopCache);
    let store = SessionStore::new(cache);

    store
        .store_jti("jti-lifecycle", 42, Duration::from_secs(3600))
        .await
        .expect("store_jti must not error on NoopCache");
    // NoopCache 不存数据，校验返回 false（调用方据此决定是否放行）。
    assert!(!store.is_jti_valid("jti-lifecycle").await.unwrap());
    store
        .revoke_jti("jti-lifecycle")
        .await
        .expect("revoke_jti must not error on NoopCache");
    assert!(!store.is_jti_valid("jti-lifecycle").await.unwrap());
}

// ---- Redis URL 拼装 -------------------------------------------------------

#[test]
fn redis_url_builder_from_components() {
    let cfg = Config {
        redis_host: "cache.internal".to_string(),
        redis_port: "6380".to_string(),
        redis_password: "s3cret".to_string(),
        redis_db: 2,
        ..Config::default()
    };
    assert_eq!(cfg.get_redis_url(), "redis://:s3cret@cache.internal:6380/2");
}

#[test]
fn redis_url_explicit_overrides_components() {
    let cfg = Config {
        redis_url: "redis://override:6379/1".to_string(),
        redis_host: "cache.internal".to_string(),
        redis_port: "6380".to_string(),
        ..Config::default()
    };
    assert_eq!(cfg.get_redis_url(), "redis://override:6379/1");
}
