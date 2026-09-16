//! Auth HTTP handlers: login, logout, refresh, profile, csrf。
//!
//! Cookie 属性对齐 Go `controllers/auth_controller.go`：
//! - `access_token`：HttpOnly=true、Path=/、SameSite 由 `COOKIE_SAME_SITE` 决定
//!   （默认 lax）、生产环境 Secure=true、MaxAge=JWT 有效期；
//! - `csrf_token`：HttpOnly=false、其余与 access_token 一致，供同源 JS 读取；
//! - 登出时以 MaxAge=-1 清除两枚 Cookie。

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use axum::extract::State;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tower_cookies::cookie::time::Duration as CookieDuration;
use tower_cookies::cookie::Cookie as RawCookie;
use tower_cookies::Cookies;
use validator::Validate;

use crate::auth::csrf::{generate_csrf_token, AUTH_COOKIE_NAME, CSRF_COOKIE_NAME};
use crate::auth::jwt::issue_token;
use crate::auth::jwt::Claims;
use crate::auth::middleware::AppState;
use crate::auth::password::verify_password;
use crate::error::{AppError, AppResult};
use crate::middleware::security_headers::{cookie_same_site, is_production};
use crate::models::user::{User, UserResponse};
use crate::response::{ApiResponse, WithStatus};

// ---------------------------------------------------------------------------
// 登录失败锁定（移植 Go services/auth_service.go 的 isLocked/recordFailure/recordSuccess）
// ---------------------------------------------------------------------------

/// 连续失败多少次后锁定。
const MAX_FAILED_LOGINS: i32 = 5;
/// 锁定时长（秒）。
const LOCKOUT_SECS: i64 = 15 * 60;

#[derive(Default, Clone, Copy)]
struct LoginAttempt {
    failures: i32,
    /// 锁定到期时间（unix 秒）；None 表示未锁定。
    locked_until: Option<i64>,
}

/// 进程内失败计数表。AppState 不允许改动，这里用 OnceLock 惰性初始化全局表，
/// 语义与 Go `AuthService.attempts` 一致。
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
    DUMMY.get_or_init(|| {
        crate::auth::password::hash_password("metaclouds-dummy-timing").unwrap_or_default()
    })
}

// ---------------------------------------------------------------------------
// Cookie 构造
// ---------------------------------------------------------------------------

/// Cookie 最大存活秒数，对齐 Go `setAuthCookie/setCsrfCookie`：JWT 有效期，
/// 未配置时回退一天。
fn max_age_secs(state: &AppState) -> i64 {
    let s = state.config.jwt_expires.as_secs() as i64;
    if s <= 0 {
        86_400
    } else {
        s
    }
}

/// 构造 httpOnly 的 access_token Cookie。
fn build_auth_cookie(token: &str, max_age_secs: i64) -> RawCookie<'static> {
    RawCookie::build((AUTH_COOKIE_NAME, token.to_string()))
        .http_only(true)
        .secure(is_production())
        .same_site(cookie_same_site())
        .path("/")
        .max_age(CookieDuration::seconds(max_age_secs))
        .build()
}

/// 构造非 httpOnly 的 csrf_token Cookie（JS 需读取以完成双提交）。
fn build_csrf_cookie(token: &str, max_age_secs: i64) -> RawCookie<'static> {
    RawCookie::build((CSRF_COOKIE_NAME, token.to_string()))
        .http_only(false)
        .secure(is_production())
        .same_site(cookie_same_site())
        .path("/")
        .max_age(CookieDuration::seconds(max_age_secs))
        .build()
}

/// 清除两枚认证 Cookie（等价 MaxAge=-1）。
fn clear_auth_cookies(cookies: &Cookies) {
    let mut access = RawCookie::from(AUTH_COOKIE_NAME);
    access.set_path("/");
    access.set_http_only(true);
    let mut csrf = RawCookie::from(CSRF_COOKIE_NAME);
    csrf.set_path("/");
    cookies.remove(access);
    cookies.remove(csrf);
}

// ---------------------------------------------------------------------------
// 请求/响应结构
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, message = "username is required"))]
    pub username: String,
    #[validate(length(min = 1, message = "password is required"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
    pub expires_at: i64,
}

#[derive(Debug, Serialize)]
pub struct CsrfResponse {
    pub csrf_token: String,
}

/// 为给定用户签发 JWT 并写入两枚 Cookie，返回登录响应。
/// login 与 refresh 共用此逻辑。
fn issue_and_set_cookies(
    state: &AppState,
    cookies: &Cookies,
    user: &User,
) -> AppResult<(String, i64)> {
    let max_age = max_age_secs(state);
    let (token, expires_at) = issue_token(
        state.config.jwt_secret.as_bytes(),
        user.id,
        &user.username,
        &user.email,
        &user.role,
        user.tenant_id,
        state.config.jwt_expires.as_secs(),
    )?;

    cookies.add(build_auth_cookie(&token, max_age));
    cookies.add(build_csrf_cookie(&generate_csrf_token(), max_age));
    Ok((token, expires_at))
}

/// `POST /api/v1/auth/login`
pub async fn login(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(body): Json<LoginRequest>,
) -> AppResult<WithStatus<LoginResponse>> {
    body.validate()?;

    // 先检查锁定，避免对已锁定账户继续做昂贵的密码校验。
    if is_locked(&body.username) {
        return Err(AppError::unauthorized(
            "account temporarily locked due to repeated failed logins",
        ));
    }

    let user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE username = ?1")
        .bind(&body.username)
        .fetch_optional(&state.pool)
        .await?;

    // 用户不存在时用哑哈希做一次等价耗时校验，降低用户名枚举风险。
    let hash = user.as_ref().map(|u| u.password_hash.clone());
    let ok = verify_password(
        &body.password,
        hash.as_deref().unwrap_or_else(|| dummy_hash()),
    );

    let user = match user {
        Some(u) if ok => u,
        _ => {
            record_failure(&body.username);
            return Err(AppError::unauthorized("invalid username or password"));
        }
    };

    record_success(&body.username);

    let (token, expires_at) = issue_and_set_cookies(&state, &cookies, &user)?;

    Ok(WithStatus {
        status: axum::http::StatusCode::OK,
        inner: ApiResponse::success(LoginResponse {
            token,
            user: user.into(),
            expires_at,
        }),
    })
}

/// `POST /api/v1/auth/logout` — 清除认证 Cookie。
pub async fn logout(cookies: Cookies) -> AppResult<Json<ApiResponse<()>>> {
    clear_auth_cookies(&cookies);
    Ok(Json(ApiResponse {
        success: true,
        data: None,
        message: Some("logged out".to_string()),
        code: None,
        timestamp: Utc::now().timestamp(),
    }))
}

/// `POST /api/v1/auth/refresh` — 用【尚未过期】的令牌换取一份新令牌。
///
/// 路由挂在 `jwt_auth` 之后，因此只有仍有效的令牌能到达这里；
/// 这里用当前用户信息重新签发，刷新 Cookie 并返回 LoginResponse。
pub async fn refresh(
    State(state): State<AppState>,
    cookies: Cookies,
    claims: Claims,
) -> AppResult<WithStatus<LoginResponse>> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(claims.user_id as i64)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::not_found("user not found"))?;

    let (token, expires_at) = issue_and_set_cookies(&state, &cookies, &user)?;

    Ok(WithStatus {
        status: axum::http::StatusCode::OK,
        inner: ApiResponse::success(LoginResponse {
            token,
            user: user.into(),
            expires_at,
        }),
    })
}

/// `GET /api/v1/auth/profile` — 返回当前登录用户（数据取自 DB）。
pub async fn get_profile(
    State(state): State<AppState>,
    claims: Claims,
) -> AppResult<Json<ApiResponse<UserResponse>>> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(claims.user_id as i64)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::unauthorized("user no longer exists"))?;
    Ok(Json(ApiResponse::success(user.into())))
}

/// `GET /api/v1/auth/csrf` — 从 csrf_token Cookie 读取并返回双提交令牌。
///
/// 未登录（无该 Cookie）时返回 401，引导前端重新登录。
pub async fn get_csrf_token(cookies: Cookies) -> AppResult<Json<ApiResponse<CsrfResponse>>> {
    let token = cookies
        .get(CSRF_COOKIE_NAME)
        .map(|c| c.value().to_string())
        .ok_or_else(|| AppError::unauthorized("CSRF token not found, please login first"))?;
    Ok(Json(ApiResponse::success(CsrfResponse {
        csrf_token: token,
    })))
}
