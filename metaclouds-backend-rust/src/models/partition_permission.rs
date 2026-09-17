//! PartitionPermission 分区授权模型（对齐 WP-P2-B4 调度域规格）。
//!
//! 落库表为 `migrations/007_b4_scheduler.sql` 建的 `partition_permissions`：
//! partition_id/user_id/tenant_id 外键 / permission_type(read|write|admin) /
//! granted_by→users / granted_at / expires_at 可空。**无软删除列**（授权直接物理删除）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};

/// 授权类型枚举常量（对齐 permission_type：read / write / admin）。
pub mod permission_type {
    pub const READ: &str = "read";
    pub const WRITE: &str = "write";
    pub const ADMIN: &str = "admin";
}

/// `partition_permissions` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PartitionPermission {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub partition_id: i64,
    pub user_id: i64,
    pub tenant_id: i64,
    pub permission_type: String,
    pub granted_by: i64,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// 对外授权视图（与行同形，授权记录无软删除列需要剔除）。
#[derive(utoipa::ToSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartitionPermissionResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub partition_id: i64,
    pub user_id: i64,
    pub tenant_id: i64,
    pub permission_type: String,
    pub granted_by: i64,
    pub granted_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<PartitionPermission> for PartitionPermissionResponse {
    fn from(p: PartitionPermission) -> Self {
        Self {
            id: p.id,
            created_at: p.created_at,
            updated_at: p.updated_at,
            partition_id: p.partition_id,
            user_id: p.user_id,
            tenant_id: p.tenant_id,
            permission_type: p.permission_type,
            granted_by: p.granted_by,
            granted_at: p.granted_at,
            expires_at: p.expires_at,
        }
    }
}

/// INSERT 授权入参。
pub struct NewPermission {
    pub partition_id: i64,
    pub user_id: i64,
    pub tenant_id: i64,
    pub permission_type: String,
    pub granted_by: i64,
    pub expires_at: Option<DateTime<Utc>>,
}

/// INSERT 授权（自动时间戳）。
pub async fn create(pool: &SqlitePool, input: NewPermission) -> AppResult<PartitionPermission> {
    let now = Utc::now();
    let res = sqlx::query(
        "INSERT INTO partition_permissions (created_at, updated_at, partition_id, user_id, \
         tenant_id, permission_type, granted_by, granted_at, expires_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(now)
    .bind(now)
    .bind(input.partition_id)
    .bind(input.user_id)
    .bind(input.tenant_id)
    .bind(&input.permission_type)
    .bind(input.granted_by)
    .bind(now)
    .bind(input.expires_at)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id)
        .await?
        .ok_or_else(|| AppError::internal("inserted partition permission not found"))
}

/// 按 id 查询授权。
pub async fn get_by_id(pool: &SqlitePool, id: i64) -> AppResult<Option<PartitionPermission>> {
    Ok(
        sqlx::query_as("SELECT * FROM partition_permissions WHERE id = ?1")
            .bind(id)
            .fetch_optional(pool)
            .await?,
    )
}

/// 按 partition_id 列出授权。
pub async fn list_by_partition(
    pool: &SqlitePool,
    partition_id: i64,
) -> AppResult<Vec<PartitionPermission>> {
    Ok(sqlx::query_as(
        "SELECT * FROM partition_permissions WHERE partition_id = ?1 ORDER BY id ASC",
    )
    .bind(partition_id)
    .fetch_all(pool)
    .await?)
}

/// 按 user_id 列出授权。
pub async fn list_by_user(pool: &SqlitePool, user_id: i64) -> AppResult<Vec<PartitionPermission>> {
    Ok(
        sqlx::query_as("SELECT * FROM partition_permissions WHERE user_id = ?1 ORDER BY id ASC")
            .bind(user_id)
            .fetch_all(pool)
            .await?,
    )
}

/// 物理删除授权（rows_affected > 0 表示命中）。
pub async fn delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let res = sqlx::query("DELETE FROM partition_permissions WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
