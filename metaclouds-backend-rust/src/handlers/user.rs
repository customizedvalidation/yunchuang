//! HTTP handlers for `/api/v1/users`.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::auth::password::hash_password;
use crate::error::{AppError, AppResult};
use crate::models::user::{CreateUserRequest, UpdateUserRequest, User, UserResponse};
use crate::response::{ApiResponse, WithStatus};

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Paginated<T> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub total_pages: u32,
}

/// `GET /api/v1/users` — paginated list, optional `?search=` filter.
pub async fn list_users(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> AppResult<Json<ApiResponse<Paginated<UserResponse>>>> {
    let page = q.page.unwrap_or(1).max(1);
    let page_size = q.page_size.unwrap_or(10).clamp(1, 100);
    let offset = (page - 1) * page_size;
    let like = q.search.map(|s| format!("%{}%", s));

    let total: i64 = match &like {
        Some(like) => {
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE username LIKE ?1 OR email LIKE ?1")
                .bind(like)
                .fetch_one(&state.pool)
                .await?
        }
        None => {
            sqlx::query_scalar("SELECT COUNT(*) FROM users")
                .fetch_one(&state.pool)
                .await?
        }
    };

    let rows: Vec<User> = match &like {
        Some(like) => {
            sqlx::query_as(
                "SELECT * FROM users \
                 WHERE username LIKE ?1 OR email LIKE ?1 \
                 ORDER BY id ASC LIMIT ?2 OFFSET ?3",
            )
            .bind(like)
            .bind(page_size)
            .bind(offset)
            .fetch_all(&state.pool)
            .await?
        }
        None => {
            sqlx::query_as("SELECT * FROM users ORDER BY id ASC LIMIT ?1 OFFSET ?2")
                .bind(page_size)
                .bind(offset)
                .fetch_all(&state.pool)
                .await?
        }
    };

    let total_pages = if page_size == 0 {
        0
    } else {
        let ps = page_size as i64;
        ((total + ps - 1) / ps) as u32
    };
    Ok(Json(ApiResponse::success(Paginated {
        data: rows.into_iter().map(UserResponse::from).collect(),
        total,
        page,
        page_size,
        total_pages,
    })))
}

/// `POST /api/v1/users`
pub async fn create_user(
    State(state): State<AppState>,
    Json(body): Json<CreateUserRequest>,
) -> AppResult<WithStatus<UserResponse>> {
    body.validate()?;

    let hash = hash_password(&body.password)?;
    let now = chrono::Utc::now();
    let role = body.role.unwrap_or_else(|| "user".to_string());
    let tenant_id = body.tenant_id.unwrap_or(1);

    let result = sqlx::query(
        "INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(now)
    .bind(now)
    .bind(&body.username)
    .bind(&body.email)
    .bind(&hash)
    .bind(&role)
    .bind(tenant_id)
    .execute(&state.pool)
    .await;

    if let Err(sqlx::Error::Database(ref db_err)) = result {
        if db_err.is_unique_violation() {
            return Err(AppError::conflict("username or email already exists"));
        }
    }
    let result = result?;

    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(result.last_insert_rowid())
        .fetch_one(&state.pool)
        .await?;

    Ok(WithStatus {
        status: axum::http::StatusCode::CREATED,
        inner: ApiResponse::success(user.into()),
    })
}

/// `GET /api/v1/users/:id`
pub async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<UserResponse>>> {
    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::not_found("user not found"))?;
    Ok(Json(ApiResponse::success(user.into())))
}

/// `PUT /api/v1/users/:id`
pub async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateUserRequest>,
) -> AppResult<Json<ApiResponse<UserResponse>>> {
    body.validate()?;

    // Load existing row; fail 404 early.
    let existing: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::not_found("user not found"))?;

    let username = body.username.unwrap_or(existing.username);
    let email = body.email.unwrap_or(existing.email);
    let role = body.role.unwrap_or(existing.role);
    let tenant_id = body.tenant_id.unwrap_or(existing.tenant_id);
    let password_hash = match body.password {
        Some(pw) => hash_password(&pw)?,
        None => existing.password_hash,
    };
    let now = chrono::Utc::now();

    let result = sqlx::query(
        "UPDATE users SET username = ?1, email = ?2, password_hash = ?3, role = ?4, tenant_id = ?5, updated_at = ?6 WHERE id = ?7",
    )
    .bind(&username)
    .bind(&email)
    .bind(&password_hash)
    .bind(&role)
    .bind(tenant_id)
    .bind(now)
    .bind(id)
    .execute(&state.pool)
    .await;
    if let Err(sqlx::Error::Database(ref db_err)) = result {
        if db_err.is_unique_violation() {
            return Err(AppError::conflict("username or email already exists"));
        }
    }
    result?;

    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = ?1")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    Ok(Json(ApiResponse::success(user.into())))
}

/// `DELETE /api/v1/users/:id`
pub async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<()>>> {
    let result = sqlx::query("DELETE FROM users WHERE id = ?1")
        .bind(id)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::not_found("user not found"));
    }
    Ok(Json(ApiResponse::success(())))
}
