//! 认证服务（对齐 Go `services/auth_service.go`）。
//!
//! 与 Go 版一致：
//! - 连续失败 5 次锁定 15 分钟，锁定状态保存在进程内（非 DB 列）；
//! - 用户不存在时也执行一次等价耗时的哑哈希比较，防止用户名枚举；
//! - 密码校验支持 argon2（新）与 bcrypt（Go 遗留）双读；
//! - 登录/刷新成功签发 JWT，并记录 `last_login_at`。
//!
//! 本层不感知 HTTP：返回结构化结果，Cookie 由 handler 维护。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use chrono::Utc;
use sqlx::SqlitePool;

use crate::auth::jwt::issue_token;
use crate::auth::password::{hash_password, verify_password};
use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::user::{self, User, UserResponse};

// ---------------------------------------------------------------------------
// 登录失败锁定（移植 Go auth_service.go 的 isLocked/recordFailure/recordSuccess）
// ---------------------------------------------------------------------------

/// 连续失败多少次后锁定。
const MAX_FAILED_LOGINS: i32 = 5;
/// 锁定时长（秒）：15 分钟。
const LOCKOUT_SECS: i64 = 15 * 60;

#[derive(Default, Clone, Copy)]
struct LoginAttempt {
    failures: i32,
    /// 锁定到期时间（unix 秒）；None 表示未锁定。
    locked_until: Option<i64>,
}

/// 进程内失败计数表。语义与 Go `AuthService.attempts` 一致。
fn login_attempts() -> &'static Mutex<HashMap<String, LoginAttempt>> {
    static ATTEMPTS: OnceLock<Mutex<HashMap<String, LoginAttempt>>> = OnceLock::new();
    ATTEMPTS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// 报告账户当前是否锁定，并清理已过期的记录。
fn is_locked(username: &str) -> bool {
    let now = Utc::now().timestamp();
    let mut map = login_attempts().lock().unwrap();
    if let Some(att) = map.get_mut(username) {
        if let Some(until) = att.locked_until {
            if until > now {
                return true;
            }
            att.locked_until = None;
            att.failures = 0;
        }
    }
    false
}

fn record_failure(username: &str) {
    let now = Utc::now().timestamp();
    let mut map = login_attempts().lock().unwrap();
    let att = map.entry(username.to_string()).or_default();
    att.failures += 1;
    if att.failures >= MAX_FAILED_LOGINS {
        att.locked_until = Some(now + LOCKOUT_SECS);
        att.failures = 0;
    }
}

fn record_success(username: &str) {
    login_attempts().lock().unwrap().remove(username);
}

/// 与任何账户都不对应的哑哈希。用户不存在时用它做一次等价耗时的校验，
/// 防止通过响应时间枚举用户名。
fn dummy_hash() -> &'static str {
    static DUMMY: OnceLock<String> = OnceLock::new();
    DUMMY.get_or_init(|| hash_password("metaclouds-dummy-timing").unwrap_or_default())
}

// ---------------------------------------------------------------------------
// 出参
// ---------------------------------------------------------------------------

/// 登录/刷新的统一出参：令牌 + 用户视图 + 过期时间。
pub struct LoginOutput {
    pub user: UserResponse,
    pub token: String,
    pub expires_at: i64,
}

/// 从活跃用户行签发 JWT。
fn issue(config: &Config, user: &User) -> AppResult<(String, i64)> {
    issue_token(
        config.jwt_secret.as_bytes(),
        user.id,
        &user.username,
        &user.email,
        &user.role,
        user.tenant_id,
        config.jwt_expires.as_secs(),
    )
}

// ---------------------------------------------------------------------------
// 业务方法
// ---------------------------------------------------------------------------

/// 登录：校验密码、检查锁定、签发 JWT、记录 last_login_at。
pub async fn login(
    pool: &SqlitePool,
    config: &Config,
    username: &str,
    password: &str,
) -> AppResult<LoginOutput> {
    // 先检查锁定，避免对已锁定账户继续做昂贵的密码校验。
    if is_locked(username) {
        return Err(AppError::rate_limit(
            "account temporarily locked due to repeated failed logins",
        ));
    }

    let row: Option<User> =
        sqlx::query_as("SELECT * FROM users WHERE username = ?1 AND deleted_at IS NULL")
            .bind(username)
            .fetch_optional(pool)
            .await?;

    // 用户不存在时用哑哈希做一次等价耗时校验，降低用户名枚举风险。
    let stored_hash = row.as_ref().map(|u| u.password_hash.clone());
    let ok = verify_password(
        password,
        stored_hash.as_deref().unwrap_or_else(|| dummy_hash()),
    );

    let user = match row {
        Some(u) if ok => u,
        _ => {
            record_failure(username);
            return Err(AppError::unauthorized("invalid username or password"));
        }
    };

    record_success(username);

    // 记录最近一次成功登录时间（失败不阻断登录主流程）。
    if let Err(e) = user::touch_last_login(pool, user.id).await {
        tracing::warn!(error = %e, user_id = user.id, "failed to record last_login_at");
    }

    let (token, expires_at) = issue(config, &user)?;
    Ok(LoginOutput {
        user: user.into(),
        token,
        expires_at,
    })
}

/// 登出：无状态 JWT，服务层无 DB 副作用；仅清理该账户的失败计数。
pub async fn logout(pool: &SqlitePool, user_id: i64) -> AppResult<()> {
    if let Some(user) = user::get_by_id(pool, user_id, false).await? {
        record_success(&user.username);
    }
    Ok(())
}

/// 刷新：按 user_id 取活跃用户并重签 JWT（对齐 Go Refresh）。
pub async fn refresh(pool: &SqlitePool, config: &Config, user_id: i64) -> AppResult<LoginOutput> {
    let user = user::get_by_id(pool, user_id, false)
        .await?
        .ok_or_else(|| AppError::not_found("user not found"))?;
    let (token, expires_at) = issue(config, &user)?;
    Ok(LoginOutput {
        user: user.into(),
        token,
        expires_at,
    })
}

/// 修改密码：校验旧密码 → argon2 哈希新密码 → 落库。
pub async fn change_password(
    pool: &SqlitePool,
    user_id: i64,
    old_password: &str,
    new_password: &str,
) -> AppResult<()> {
    let user = user::get_by_id(pool, user_id, false)
        .await?
        .ok_or_else(|| AppError::not_found("user not found"))?;

    if !verify_password(old_password, &user.password_hash) {
        return Err(AppError::unauthorized("old password is incorrect"));
    }

    let new_hash = hash_password(new_password)?;
    let now = Utc::now();
    sqlx::query("UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(new_hash)
        .bind(now)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 获取当前用户资料（数据取自 DB，不使用 JWT claims 的陈旧快照）。
pub async fn get_profile(pool: &SqlitePool, user_id: i64) -> AppResult<UserResponse> {
    let user = user::get_by_id(pool, user_id, false)
        .await?
        .ok_or_else(|| AppError::not_found("user not found"))?;
    Ok(user.into())
}
