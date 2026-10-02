//! 登录会话缓存：JWT `jti` 的写入 / 撤销 / 校验（P3-01）。
//!
//! 对齐 Go 版 `models/redis.go` 的 `StoreSession` / `InvalidateSession`：
//! - 登录成功后把 `jti` 写入 Redis，TTL = JWT 剩余有效期；
//! - 登出时删除该 `jti`（令牌撤销）；
//! - 请求时校验 `jti` 是否仍在 Redis 中（可选，由调用方按配置开关决定是否启用）。
//!
//! 登录失败锁定计数当前仍在 Go/Rust 侧用内存态实现（见 `services::auth`），
//! 本模块只预留会话 key 接口，不接管锁定计数。

use std::sync::Arc;
use std::time::Duration;

use super::redis::Cache;
use crate::error::AppResult;

/// 会话逻辑 key 命名空间（与 Go `session:<sessionID>` 对齐）。
///
/// 外层 `metaclouds:` 前缀由 [`super::redis::RedisCache`] 统一拼接。
const SESSION_NS: &str = "session:";
/// 撤销名单命名空间（登出拉黑的 jti）。
const REVOKED_NS: &str = "revoked:";

/// JWT jti 会话存储，封装在 [`Cache`] 之上。
///
/// 业务层（auth 中间件 / 登录 handler）持有本结构，不直接拼 key。
pub struct SessionStore {
    cache: Arc<dyn Cache>,
}

impl SessionStore {
    /// 用任意 [`Cache`] 实现构造；生产环境由 `build_cache` 注入。
    pub fn new(cache: Arc<dyn Cache>) -> Self {
        Self { cache }
    }

    /// 拼逻辑 key：`session:<jti>`。
    fn key(jti: &str) -> String {
        format!("{SESSION_NS}{jti}")
    }

    /// 登录成功后写入 jti，TTL = JWT 剩余过期时间。
    ///
    /// `user_id` 作为 value 持久化，便于将来从会话反查用户（对齐 Go
    /// `StoreSession(sessionID, userID, expiration)`）。
    pub async fn store_jti(&self, jti: &str, user_id: u64, ttl: Duration) -> AppResult<()> {
        self.cache
            .set(&Self::key(jti), user_id.to_string().into_bytes(), ttl)
            .await
    }

    /// 登出时撤销 jti（删除会话，等价于 Go `InvalidateSession`）。
    pub async fn revoke_jti(&self, jti: &str) -> AppResult<()> {
        self.cache.delete(&Self::key(jti)).await
    }

    /// 【撤销名单】登出时把 jti 拉黑，TTL = 令牌剩余有效期。
    ///
    /// 与 [`Self::revoke_jti`]（白名单删除）是**两种模型**，不能混用：
    /// 白名单要求"jti 必须在 Redis 中存在"，Redis 重启/丢数据会导致全员掉线；
    /// 撤销名单只记录"被显式登出的 jti"，Redis 不可用时降级为"等令牌自然过期"，
    /// 不会误伤正常会话。生产默认走撤销名单。
    pub async fn blacklist_jti(&self, jti: &str, ttl: Duration) -> AppResult<()> {
        self.cache
            .set(&Self::blacklist_key(jti), b"1".to_vec(), ttl)
            .await
    }

    /// 查询 jti 是否已被显式登出。
    pub async fn is_jti_revoked(&self, jti: &str) -> AppResult<bool> {
        self.cache.exists(&Self::blacklist_key(jti)).await
    }

    fn blacklist_key(jti: &str) -> String {
        format!("{REVOKED_NS}{jti}")
    }

    /// 请求时校验 jti 是否仍然有效（在 Redis 中存在）。
    ///
    /// 注意：NoopCache 恒返回 false。这意味着"未启用 Redis 时不强制校验 jti"，
    /// 由调用方（auth 中间件）根据配置开关决定是否在该模式下放行——与 Go 版
    /// "RedisClient 未初始化时跳过黑名单校验"一致。
    pub async fn is_jti_valid(&self, jti: &str) -> AppResult<bool> {
        self.cache.exists(&Self::key(jti)).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::NoopCache;

    #[tokio::test]
    async fn session_store_with_noop_cache_does_not_panic() {
        let store = SessionStore::new(Arc::new(NoopCache));
        store
            .store_jti("jti-noop", 42, Duration::from_secs(3600))
            .await
            .expect("store_jti on NoopCache must not error");
        // NoopCache 不存数据，exists 恒 false。
        assert!(
            !store.is_jti_valid("jti-noop").await.unwrap(),
            "NoopCache should report jti invalid (not stored)"
        );
        store
            .revoke_jti("jti-noop")
            .await
            .expect("revoke_jti on NoopCache must not error");
    }
}
