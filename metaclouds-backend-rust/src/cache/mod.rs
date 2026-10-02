//! Redis 会话/缓存层（Phase 3 P3-01）。
//!
//! # 设计目标
//!
//! 对齐 Go 版 `models/redis.go` 的缓存语义，但用 Rust 的异步连接管理器封装：
//! - [`Cache`] trait 抽象底层存储，业务层只依赖该 trait；
//! - [`RedisCache`] 基于 [`redis::aio::ConnectionManager`]（tokio 异步、自动重连）；
//! - [`NoopCache`] 在 Redis 禁用或连接失败时使用，保证功能不受影响；
//! - [`SessionStore`] 负责 JWT `jti` 的写入 / 撤销 / 校验。
//!
//! # Key 命名空间
//!
//! 所有 key 统一加 `metaclouds:` 前缀（对齐任务约定）。业务层只传逻辑 key
//! （如 `session:<jti>`），由 [`RedisCache`] 统一加前缀，最终落盘形如
//! `metaclouds:session:<jti>`。
//!
//! # 优雅降级
//!
//! [`build_cache`] 在以下情况返回 [`NoopCache`]，绝不 panic：
//! 1. 配置 `redis_enabled = false`；
//! 2. Redis 地址解析失败 / 连接失败 / ping 失败。
//!
//! # 指标预留
//!
//! [`CacheMetrics`] trait 与 [`CacheStats`] 仅暴露原子计数与命中率计算，
//! 不在本阶段注册 Prometheus 指标（P3-03 范围）。

pub mod redis;
pub mod session;

use std::sync::{Arc, OnceLock};

pub use redis::{
    build_cache, with_prefix, Cache, CacheMetrics, CacheStats, NoopCache, RedisCache, PREFIX,
};
pub use session::SessionStore;

/// 进程级缓存实例（由 `main.rs` 在启动时 `install_cache` 注入）。
///
/// 为什么是全局而不是 `AppState` 字段：`AppState { pool, config }` 在 28 个集成
/// 测试里逐字构造，加字段要改 30 处构造点；而缓存是真正的**进程级单例**
/// （Redis 连接池本就该全局复用）。未安装时返回 [`NoopCache`]，因此单测与
/// 未启用 Redis 的部署行为与过去完全一致（优雅降级）。
static CACHE: OnceLock<Arc<dyn Cache>> = OnceLock::new();

/// 安装进程级缓存实现。重复调用只生效第一次。
pub fn install_cache(cache: Arc<dyn Cache>) {
    if CACHE.set(cache).is_err() {
        tracing::warn!("install_cache called more than once; keeping the first instance");
    }
}

/// 取进程级缓存；未安装时返回 [`NoopCache`]。
pub fn global_cache() -> Arc<dyn Cache> {
    CACHE
        .get()
        .cloned()
        .unwrap_or_else(|| Arc::new(NoopCache) as Arc<dyn Cache>)
}

/// 是否已安装了非 Noop 的缓存实现（Redis 真正可用）。
pub fn redis_active() -> bool {
    CACHE.get().map(|c| !c.is_noop()).unwrap_or(false)
}

/// 取绑定在全局缓存上的会话存储。
pub fn global_session() -> SessionStore {
    SessionStore::new(global_cache())
}
