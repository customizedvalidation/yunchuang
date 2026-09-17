//! Redis 连接管理 + [`Cache`] 抽象 + 命中率计数（P3-01）。
//!
//! 与 Go 版 `models/redis.go` 对照：
//! - Go 用 `go-redis` 的 `*redis.Client`（PoolSize=10，自动重连）；
//! - Rust 用 [`redis::aio::ConnectionManager`]（tokio 异步，单连接多路复用 + 自动重连）。
//!
//! Go 版的 max_open=100 / idle=20 / lifetime=300s 对齐的是 **数据库** 连接池
//! （`models/db.go` 的 `SetMaxOpenConns`），不是 Redis；这里保持 redis-rs 的默认重连语义。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::future::{BoxFuture, FutureExt};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;

use crate::config::Config;
use crate::error::{AppError, AppResult, ErrorCode};

/// 全局 key 命名空间前缀（对齐任务约定）。
///
/// 业务层传逻辑 key（如 `session:<jti>`），由 [`RedisCache`] 统一拼上此前缀，
/// 最终落盘形如 `metaclouds:session:<jti>`。
pub const PREFIX: &str = "metaclouds:";

/// 把逻辑 key 加上全局前缀（纯函数，便于单测）。
///
/// 若调用方已经传了带前缀的 key，不二次叠加。
pub fn with_prefix(key: &str) -> String {
    if let Some(rest) = key.strip_prefix(PREFIX) {
        return format!("{PREFIX}{rest}");
    }
    format!("{PREFIX}{key}")
}

/// 把 TTL 转成秒供 redis `SET EX` 使用。0/负数按 0 处理（不设过期）。
pub fn ttl_seconds(ttl: Duration) -> u64 {
    ttl.as_secs()
}

/// 原子命中率计数器（RedisCache 内部使用，跨线程安全）。
#[derive(Debug, Default)]
struct CacheCounters {
    hits: AtomicU64,
    misses: AtomicU64,
}

/// 一次缓存统计快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
}

impl CacheStats {
    /// 命中率 = hits / (hits + misses)；尚无请求时约定返回 1.0（无 miss）。
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits.saturating_add(self.misses);
        if total == 0 {
            return 1.0;
        }
        self.hits as f64 / total as f64
    }
}

/// 缓存抽象层。所有方法返回 [`BoxFuture`] 以支持 `Arc<dyn Cache>` 对象安全。
///
/// 业务层只依赖本 trait；运行时根据配置注入 [`RedisCache`] 或 [`NoopCache`]。
pub trait Cache: Send + Sync {
    /// 读缓存；未命中返回 `Ok(None)`。
    fn get(&self, key: &str) -> BoxFuture<'_, AppResult<Option<Vec<u8>>>>;
    /// 写缓存并设置 TTL。
    fn set(&self, key: &str, value: Vec<u8>, ttl: Duration) -> BoxFuture<'_, AppResult<()>>;
    /// 删除 key。
    fn delete(&self, key: &str) -> BoxFuture<'_, AppResult<()>>;
    /// 判断 key 是否存在。
    fn exists(&self, key: &str) -> BoxFuture<'_, AppResult<bool>>;
    /// 是否为空实现（Noop）。用于降级判定与测试断言，默认 false。
    fn is_noop(&self) -> bool {
        false
    }
}

/// 为 P3-03 Prometheus 集成预留的指标钩子。
///
/// 本阶段只维护原子计数与命中率；P3-03 可在 [`RedisCache`] 上额外挂
/// `prometheus::IntCounter`，不在此处引入 Prometheus 类型。
pub trait CacheMetrics: Send + Sync {
    /// 记录一次命中。
    fn record_hit(&self);
    /// 记录一次未命中。
    fn record_miss(&self);
    /// 读取原子快照。
    fn snapshot(&self) -> CacheStats;
}

/// 空缓存实现：Redis 禁用或连接失败时的降级方案。
///
/// 语义对齐 Go 版 `RedisClient == nil` 时的内存降级路径——但 NoopCache 不
/// 维护任何状态，确保"关闭 Redis 即关闭缓存"，业务层回源 DB。
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopCache;

impl Cache for NoopCache {
    fn get(&self, _key: &str) -> BoxFuture<'_, AppResult<Option<Vec<u8>>>> {
        async { Ok(None) }.boxed()
    }

    fn set(&self, _key: &str, _value: Vec<u8>, _ttl: Duration) -> BoxFuture<'_, AppResult<()>> {
        async { Ok(()) }.boxed()
    }

    fn delete(&self, _key: &str) -> BoxFuture<'_, AppResult<()>> {
        async { Ok(()) }.boxed()
    }

    fn exists(&self, _key: &str) -> BoxFuture<'_, AppResult<bool>> {
        async { Ok(false) }.boxed()
    }

    fn is_noop(&self) -> bool {
        true
    }
}

impl CacheMetrics for NoopCache {
    fn record_hit(&self) {}
    fn record_miss(&self) {}
    fn snapshot(&self) -> CacheStats {
        CacheStats { hits: 0, misses: 0 }
    }
}

/// 真实 Redis 缓存（基于 [`ConnectionManager`]）。
///
/// 内部维护原子命中/未命中计数；任何命令错误都向上冒泡为 [`AppError`]，
/// 但 [`build_cache`] 在构造阶段就已经决定"用 Redis 还是 Noop"，运行期
/// 一旦连接断开，redis-rs 的 ConnectionManager 会自动重连，无需本层重试。
#[derive(Clone)]
pub struct RedisCache {
    conn: ConnectionManager,
    counters: Arc<CacheCounters>,
}

impl RedisCache {
    /// 用已建立的连接管理器构造（测试与生产共用）。
    pub fn new(conn: ConnectionManager) -> Self {
        Self {
            conn,
            counters: Arc::new(CacheCounters::default()),
        }
    }

    /// 当前命中率快照（线程安全）。
    pub fn stats(&self) -> CacheStats {
        CacheMetrics::snapshot(self)
    }
}

impl CacheMetrics for RedisCache {
    fn record_hit(&self) {
        self.counters.hits.fetch_add(1, Ordering::Relaxed);
    }

    fn record_miss(&self) {
        self.counters.misses.fetch_add(1, Ordering::Relaxed);
    }

    fn snapshot(&self) -> CacheStats {
        CacheStats {
            hits: self.counters.hits.load(Ordering::Relaxed),
            misses: self.counters.misses.load(Ordering::Relaxed),
        }
    }
}

impl Cache for RedisCache {
    fn get(&self, key: &str) -> BoxFuture<'_, AppResult<Option<Vec<u8>>>> {
        let key = with_prefix(key);
        let mut conn = self.conn.clone();
        let counters = self.counters.clone();
        Box::pin(async move {
            let result: Option<Vec<u8>> = conn.get(&key).await.map_err(|e| {
                AppError::with_source(ErrorCode::InternalServerError, "redis get failed", e)
            })?;
            match &result {
                Some(_) => {
                    counters.hits.fetch_add(1, Ordering::Relaxed);
                }
                None => {
                    counters.misses.fetch_add(1, Ordering::Relaxed);
                }
            }
            Ok(result)
        })
    }

    fn set(&self, key: &str, value: Vec<u8>, ttl: Duration) -> BoxFuture<'_, AppResult<()>> {
        let key = with_prefix(key);
        let mut conn = self.conn.clone();
        Box::pin(async move {
            let secs = ttl_seconds(ttl);
            if secs > 0 {
                conn.set_ex::<_, _, ()>(&key, value, secs)
                    .await
                    .map_err(|e| {
                        AppError::with_source(
                            ErrorCode::InternalServerError,
                            "redis set_ex failed",
                            e,
                        )
                    })?;
            } else {
                conn.set::<_, _, ()>(&key, value).await.map_err(|e| {
                    AppError::with_source(ErrorCode::InternalServerError, "redis set failed", e)
                })?;
            }
            Ok(())
        })
    }

    fn delete(&self, key: &str) -> BoxFuture<'_, AppResult<()>> {
        let key = with_prefix(key);
        let mut conn = self.conn.clone();
        Box::pin(async move {
            let _: i64 = conn.del(&key).await.map_err(|e| {
                AppError::with_source(ErrorCode::InternalServerError, "redis del failed", e)
            })?;
            Ok(())
        })
    }

    fn exists(&self, key: &str) -> BoxFuture<'_, AppResult<bool>> {
        let key = with_prefix(key);
        let mut conn = self.conn.clone();
        Box::pin(async move {
            let n: i64 = conn.exists(&key).await.map_err(|e| {
                AppError::with_source(ErrorCode::InternalServerError, "redis exists failed", e)
            })?;
            Ok(n > 0)
        })
    }
}

/// 按配置构建缓存实例；任何失败都降级为 [`NoopCache`]，绝不 panic。
///
/// 对齐 Go `models/redis.go` 的 `InitRedis` 语义：
/// - 未启用 → 直接返回 nil（这里返回 [`NoopCache`]）；
/// - Ping 失败 → 记 warn 日志并回退内存降级（这里回退 [`NoopCache`]）。
pub async fn build_cache(config: &Config) -> Arc<dyn Cache> {
    if !config.redis_enabled {
        tracing::info!("redis disabled via config; using NoopCache");
        return Arc::new(NoopCache);
    }

    let url = config.get_redis_url();
    tracing::info!(%url, "redis enabled; connecting");

    match connect_redis(&url).await {
        Ok(conn) => {
            tracing::info!("redis connection established; using RedisCache");
            Arc::new(RedisCache::new(conn))
        }
        Err(e) => {
            tracing::warn!(error = %e, "redis connection failed; falling back to NoopCache");
            Arc::new(NoopCache)
        }
    }
}

/// 建立到 Redis 的连接管理器（含首次连接验证）。
async fn connect_redis(url: &str) -> AppResult<ConnectionManager> {
    let client = redis::Client::open(url).map_err(|e| {
        AppError::with_source(ErrorCode::InternalServerError, "invalid redis url", e)
    })?;
    // 对齐 Go `ReadTimeout: 5s / WriteTimeout: 5s`：首次连接最多等 5 秒，
    // 超时即降级，不在启动期无限挂起。
    let conn = tokio::time::timeout(Duration::from_secs(5), client.get_connection_manager())
        .await
        .map_err(|_| {
            AppError::new(
                ErrorCode::InternalServerError,
                "redis connection timed out after 5s",
            )
        })?
        .map_err(|e| {
            AppError::with_source(ErrorCode::InternalServerError, "redis connection failed", e)
        })?;
    Ok(conn)
}
