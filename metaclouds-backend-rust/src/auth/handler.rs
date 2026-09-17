//! Auth HTTP handlers: login, logout, refresh, profile, csrf, change_password。
//!
//! 业务规则（密码校验、登录锁定、哑哈希时序抹平、JWT 签发、last_login_at、
//! 改密）下沉到 [`crate::services::auth`]。本文件只做 HTTP 收发与 Cookie 维护。
//!
//! Cookie 属性对齐 Go `controllers/auth_controller.go`：
//! - `access_token`：HttpOnly=true、Path=/、SameSite 由 `COOKIE_SAME_SITE` 决定
//!   （默认 lax）、生产环境 Secure=true、MaxAge=JWT 有效期；
//! - `csrf_token`：HttpOnly=false、其余与 access_token 一致，供同源 JS 读取；
//! - 登出时以 MaxAge=-1 清除两枚 Cookie。

use axum::extract::State;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tower_cookies::cookie::time::Duration as CookieDuration;
use tower_cookies::cookie::Cookie as RawCookie;
use tower_cookies::Cookies;
use validator::Validate;

use crate::auth::csrf::{generate_csrf_token, AUTH_COOKIE_NAME, CSRF_COOKIE_NAME};
use crate::auth::jwt::Claims;
use crate::auth::middleware::AppState;
use crate::error::{AppError, AppResult};
use crate::middleware::security_headers::{cookie_same_site, is_production};
use crate::models::user::UserResponse;
use crate::response::{ApiResponse, WithStatus};
use crate::services::auth as auth_service;

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

/// 写入 access_token + csrf_token 两枚 Cookie（登录/刷新共用）。
fn set_auth_cookies(state: &AppState, cookies: &Cookies, token: &str) {
    let max_age = max_age_secs(state);
    cookies.add(build_auth_cookie(token, max_age));
    cookies.add(build_csrf_cookie(&generate_csrf_token(), max_age));
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

#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(length(min = 1, message = "username is required"))]
    pub username: String,
    #[validate(length(min = 1, message = "password is required"))]
    pub password: String,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
    pub expires_at: i64,
}

#[derive(utoipa::ToSchema, Debug, Serialize)]
pub struct CsrfResponse {
    pub csrf_token: String,
}

/// `PUT /api/v1/auth/change-password` 请求体。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1, message = "old_password is required"))]
    pub old_password: String,
    #[validate(length(min = 8, message = "new_password must be at least 8 chars"))]
    pub new_password: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /api/v1/auth/login`
#[utoipa::path(post,path="/api/v1/auth/login",request_body=LoginRequest,tag="auth",responses((status=200,description="login ok",body=LoginResponse),(status=401,description="invalid credentials",body=crate::openapi::ErrorResponse)))]
pub async fn login(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(body): Json<LoginRequest>,
) -> AppResult<WithStatus<LoginResponse>> {
    body.validate()?;

    let out = auth_service::login(
        &state.pool,
        state.config.as_ref(),
        &body.username,
        &body.password,
    )
    .await?;

    set_auth_cookies(&state, &cookies, &out.token);

    Ok(WithStatus {
        status: axum::http::StatusCode::OK,
        inner: ApiResponse::success(LoginResponse {
            token: out.token,
            user: out.user,
            expires_at: out.expires_at,
        }),
    })
}

/// `POST /api/v1/auth/logout` — 清除认证 Cookie。
#[utoipa::path(post,path="/api/v1/auth/logout",tag="auth",responses((status=200,description="logged out")))]
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
#[utoipa::path(post,path="/api/v1/auth/refresh",tag="auth",responses((status=200,description="refreshed",body=LoginResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn refresh(
    State(state): State<AppState>,
    cookies: Cookies,
    claims: Claims,
) -> AppResult<WithStatus<LoginResponse>> {
    let out =
        auth_service::refresh(&state.pool, state.config.as_ref(), claims.user_id as i64).await?;

    set_auth_cookies(&state, &cookies, &out.token);

    Ok(WithStatus {
        status: axum::http::StatusCode::OK,
        inner: ApiResponse::success(LoginResponse {
            token: out.token,
            user: out.user,
            expires_at: out.expires_at,
        }),
    })
}

/// `GET /api/v1/auth/profile` — 返回当前登录用户（数据取自 DB）。
#[utoipa::path(get,path="/api/v1/auth/profile",tag="auth",responses((status=200,description="current user",body=crate::models::user::UserResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_profile(
    State(state): State<AppState>,
    claims: Claims,
) -> AppResult<Json<ApiResponse<UserResponse>>> {
    let user = auth_service::get_profile(&state.pool, claims.user_id as i64).await?;
    Ok(Json(ApiResponse::success(user)))
}

/// `PUT /api/v1/auth/change-password` — 修改当前用户密码（需 JWT）。
#[utoipa::path(put,path="/api/v1/auth/change-password",request_body=ChangePasswordRequest,tag="auth",responses((status=200,description="password changed"),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn change_password(
    State(state): State<AppState>,
    claims: Claims,
    Json(body): Json<ChangePasswordRequest>,
) -> AppResult<Json<ApiResponse<()>>> {
    body.validate()?;
    auth_service::change_password(
        &state.pool,
        claims.user_id as i64,
        &body.old_password,
        &body.new_password,
    )
    .await?;
    Ok(Json(ApiResponse::success(())))
}

/// `GET /api/v1/auth/csrf` — 从 csrf_token Cookie 读取并返回双提交令牌。
///
/// 未登录（无该 Cookie）时返回 401，引导前端重新登录。
#[utoipa::path(get,path="/api/v1/auth/csrf",tag="auth",responses((status=200,description="csrf token",body=CsrfResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse)))]
pub async fn get_csrf_token(cookies: Cookies) -> AppResult<Json<ApiResponse<CsrfResponse>>> {
    let token = cookies
        .get(CSRF_COOKIE_NAME)
        .map(|c| c.value().to_string())
        .ok_or_else(|| AppError::unauthorized("CSRF token not found, please login first"))?;
    Ok(Json(ApiResponse::success(CsrfResponse {
        csrf_token: token,
    })))
}
