//! User model, DB row and API request/response shapes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;
use validator::Validate;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, PaginatedResult, PaginationParams, SoftDelete,
};

/// Database row for the `users` table. `password_hash` is never serialized
/// to clients.
///
/// 公共字段名（id/username/email/role/tenant_id/created_at/updated_at）保持不变，
/// 认证与 handler 层直接依赖；`deleted_at` 是 Phase 1 新增的软删除列。
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
    pub tenant_id: i64,
    /// 软删除时间；None 表示未删除（对齐 GORM gorm.DeletedAt）。
    pub deleted_at: Option<DateTime<Utc>>,
    /// 最近一次成功登录时间。Go 版未持久化该列；B1 按任务要求记录，
    /// 登录成功时由服务层刷新，不对外暴露。
    pub last_login_at: Option<DateTime<Utc>>,
}

// GORM 行为显式化：插入自动填 created_at/updated_at，更新只刷 updated_at。
impl HasTimestamps for User {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }

    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for User {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }

    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// Public-facing user view (no password hash).
#[derive(utoipa::ToSchema, Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub tenant_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            email: u.email,
            role: u.role,
            tenant_id: u.tenant_id,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

/// Body of `POST /api/v1/users`.
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3, max = 64, message = "username must be 3..=64 chars"))]
    pub username: String,
    #[validate(email(message = "email must be a valid email"))]
    pub email: String,
    #[validate(length(min = 8, message = "password must be at least 8 chars"))]
    pub password: String,
    #[serde(default = "default_role")]
    pub role: Option<String>,
    #[serde(default = "default_tenant_id")]
    pub tenant_id: Option<i64>,
}

/// Body of `PUT /api/v1/users/:id`. All fields optional; password is optional.
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate, Default)]
pub struct UpdateUserRequest {
    #[validate(length(min = 3, max = 64, message = "username must be 3..=64 chars"))]
    pub username: Option<String>,
    #[validate(email(message = "email must be a valid email"))]
    pub email: Option<String>,
    #[validate(length(min = 8, message = "password must be at least 8 chars"))]
    pub password: Option<String>,
    pub role: Option<String>,
    pub tenant_id: Option<i64>,
}

fn default_role() -> Option<String> {
    Some("user".to_string())
}
fn default_tenant_id() -> Option<i64> {
    Some(1)
}

// ---------------------------------------------------------------------------
// Repository 层 CRUD（走 orm 层的时间戳 / 软删除 / 分页约定）
// ---------------------------------------------------------------------------

/// 创建用户所需的入参（密码哈希由调用方准备）。
pub struct NewUser<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password_hash: &'a str,
    pub role: &'a str,
    pub tenant_id: i64,
}

/// INSERT：自动填充 created_at / updated_at。
pub async fn create(pool: &SqlitePool, input: NewUser<'_>) -> AppResult<User> {
    let mut user = User {
        id: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        username: input.username.to_string(),
        email: input.email.to_string(),
        password_hash: input.password_hash.to_string(),
        role: input.role.to_string(),
        tenant_id: input.tenant_id,
        deleted_at: None,
        last_login_at: None,
    };
    // 对齐 GORM：插入前统一刷一次时间戳。
    user.before_insert();

    let res = sqlx::query(
        "INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )
    .bind(user.created_at)
    .bind(user.updated_at)
    .bind(&user.username)
    .bind(&user.email)
    .bind(&user.password_hash)
    .bind(&user.role)
    .bind(user.tenant_id)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted user not found"))
}

/// 按 id 查询；`include_deleted = true` 对应 GORM 的 Unscoped()。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<User>> {
    let sql = if include_deleted {
        "SELECT * FROM users WHERE id = ?1"
    } else {
        "SELECT * FROM users WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 分页列表（默认过滤软删除）。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    include_deleted: bool,
) -> AppResult<PaginatedResult<User>> {
    let params = params.normalize();
    let where_clause = if include_deleted {
        ""
    } else {
        " WHERE deleted_at IS NULL"
    };

    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM users{where_clause}"))
        .fetch_one(pool)
        .await?;

    let rows: Vec<User> = sqlx::query_as(&format!(
        "SELECT * FROM users{where_clause} ORDER BY id ASC LIMIT ?1 OFFSET ?2"
    ))
    .bind(params.limit())
    .bind(params.offset())
    .fetch_all(pool)
    .await?;

    Ok(PaginatedResult::new(rows, total, params))
}

/// 刷新 updated_at（任意字段更新后调用）。
pub async fn touch_updated_at(pool: &SqlitePool, id: i64) -> AppResult<()> {
    sqlx::query("UPDATE users SET updated_at = ?1 WHERE id = ?2")
        .bind(Utc::now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 记录一次成功登录时间（登录成功时调用，仅写 last_login_at）。
pub async fn touch_last_login(pool: &SqlitePool, id: i64) -> AppResult<()> {
    sqlx::query("UPDATE users SET last_login_at = ?1 WHERE id = ?2")
        .bind(Utc::now())
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 软删除：DELETE 改写为 UPDATE SET deleted_at = now()。
/// 返回是否真的命中了一行。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("users"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
