//! Auth HTTP handlers: login, logout, profile.

use axum::extract::State;
use axum::Json;
use serde::{Deserialize, Serialize};
use tower_cookies::Cookies;
use validator::Validate;

use crate::auth::jwt::issue_token;
use crate::auth::jwt::Claims;
use crate::auth::middleware::AppState;
use crate::auth::password::verify_password;
use crate::error::{AppError, AppResult};
use crate::models::user::{User, UserResponse};
use crate::response::{ApiResponse, WithStatus};

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

/// `POST /api/v1/auth/login`
pub async fn login(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(body): Json<LoginRequest>,
) -> AppResult<WithStatus<LoginResponse>> {
    body.validate()?;

    let user: User = sqlx::query_as("SELECT * FROM users WHERE username = ?1")
        .bind(&body.username)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::unauthorized("invalid username or password"))?;

    if !verify_password(&body.password, &user.password_hash) {
        return Err(AppError::unauthorized("invalid username or password"));
    }

    let (token, expires_at) = issue_token(
        state.config.jwt_secret.as_bytes(),
        user.id,
        &user.username,
        &user.email,
        &user.role,
        user.tenant_id,
        state.config.jwt_expires.as_secs(),
    )?;

    // Set cookies: access_token (httpOnly) + csrf_token (readable by JS).
    let access = tower_cookies::Cookie::build(("access_token", token.clone()))
        .http_only(true)
        .same_site(tower_cookies::cookie::SameSite::Lax)
        .path("/")
        .build();
    let csrf = tower_cookies::Cookie::build(("csrf_token", uuid::Uuid::new_v4().to_string()))
        .http_only(false)
        .same_site(tower_cookies::cookie::SameSite::Lax)
        .path("/")
        .build();
    cookies.add(access);
    cookies.add(csrf);

    Ok(WithStatus {
        status: axum::http::StatusCode::OK,
        inner: ApiResponse::success(LoginResponse {
            token,
            user: user.into(),
            expires_at,
        }),
    })
}

/// `POST /api/v1/auth/logout` — clear auth cookies.
pub async fn logout(cookies: Cookies) -> AppResult<Json<ApiResponse<()>>> {
    let mut access = tower_cookies::Cookie::from("access_token");
    access.set_path("/");
    access.set_http_only(true);
    let mut csrf = tower_cookies::Cookie::from("csrf_token");
    csrf.set_path("/");
    cookies.remove(access);
    cookies.remove(csrf);
    Ok(Json(ApiResponse::success(())))
}

/// `GET /api/v1/auth/profile` — return the currently authenticated user.
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
