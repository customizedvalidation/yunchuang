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

pub use redis::{
    build_cache, with_prefix, Cache, CacheMetrics, CacheStats, NoopCache, RedisCache, PREFIX,
};
pub use session::SessionStore;
