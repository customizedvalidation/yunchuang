//! Tenant 模型（对齐 Go models/tenant.go）。
//!
//! 字段：id / name(unique) / description / status(active) / 四项配额，
//! 以及 created_at / updated_at / deleted_at 软删除三件套。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, PaginatedResult, PaginationParams, SoftDelete,
};

/// `tenants` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Tenant {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub status: String,
    pub gpu_quota: i64,
    pub cpu_quota: i64,
    pub memory_quota: i64,
    pub storage_quota: i64,
}

impl HasTimestamps for Tenant {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for Tenant {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外租户视图（对齐 Go `models.Tenant` 的 JSON tag：`deleted_at` 为 `json:"-"`）。
///
/// 与 [`Tenant`] 的区别：剔除软删除列 `deleted_at`，绝不回传给客户端。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TenantResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub description: String,
    pub status: String,
    pub gpu_quota: i64,
    pub cpu_quota: i64,
    pub memory_quota: i64,
    pub storage_quota: i64,
}

impl From<Tenant> for TenantResponse {
    fn from(t: Tenant) -> Self {
        Self {
            id: t.id,
            created_at: t.created_at,
            updated_at: t.updated_at,
            name: t.name,
            description: t.description,
            status: t.status,
            gpu_quota: t.gpu_quota,
            cpu_quota: t.cpu_quota,
            memory_quota: t.memory_quota,
            storage_quota: t.storage_quota,
        }
    }
}

/// 创建租户入参。
pub struct NewTenant<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub status: &'a str,
    pub gpu_quota: i64,
    pub cpu_quota: i64,
    pub memory_quota: i64,
    pub storage_quota: i64,
}

/// INSERT 租户（自动时间戳）。
pub async fn create(pool: &SqlitePool, input: NewTenant<'_>) -> AppResult<Tenant> {
    let mut tenant = Tenant {
        id: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        name: input.name.to_string(),
        description: input.description.to_string(),
        status: input.status.to_string(),
        gpu_quota: input.gpu_quota,
        cpu_quota: input.cpu_quota,
        memory_quota: input.memory_quota,
        storage_quota: input.storage_quota,
    };
    tenant.before_insert();

    let res = sqlx::query(
        "INSERT INTO tenants (created_at, updated_at, name, description, status, \
         gpu_quota, cpu_quota, memory_quota, storage_quota) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(tenant.created_at)
    .bind(tenant.updated_at)
    .bind(&tenant.name)
    .bind(&tenant.description)
    .bind(&tenant.status)
    .bind(tenant.gpu_quota)
    .bind(tenant.cpu_quota)
    .bind(tenant.memory_quota)
    .bind(tenant.storage_quota)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted tenant not found"))
}

/// 按 id 查询（默认排除软删除）。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<Tenant>> {
    let sql = if include_deleted {
        "SELECT * FROM tenants WHERE id = ?1"
    } else {
        "SELECT * FROM tenants WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 分页列表。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    include_deleted: bool,
) -> AppResult<PaginatedResult<Tenant>> {
    let params = params.normalize();
    let where_clause = if include_deleted {
        ""
    } else {
        " WHERE deleted_at IS NULL"
    };

    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM tenants{where_clause}"))
        .fetch_one(pool)
        .await?;

    let rows: Vec<Tenant> = sqlx::query_as(&format!(
        "SELECT * FROM tenants{where_clause} ORDER BY id ASC LIMIT ?1 OFFSET ?2"
    ))
    .bind(params.limit())
    .bind(params.offset())
    .fetch_all(pool)
    .await?;

    Ok(PaginatedResult::new(rows, total, params))
}

/// 更新租户状态与配额（自动刷 updated_at）。
pub async fn update(
    pool: &SqlitePool,
    id: i64,
    status: Option<&str>,
    gpu_quota: Option<i64>,
) -> AppResult<Option<Tenant>> {
    let mut existing = match get_by_id(pool, id, false).await? {
        Some(t) => t,
        None => return Ok(None),
    };
    if let Some(s) = status {
        existing.status = s.to_string();
    }
    if let Some(q) = gpu_quota {
        existing.gpu_quota = q;
    }
    existing.before_update();

    sqlx::query("UPDATE tenants SET status = ?1, gpu_quota = ?2, updated_at = ?3 WHERE id = ?4")
        .bind(&existing.status)
        .bind(existing.gpu_quota)
        .bind(existing.updated_at)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(Some(existing))
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("tenants"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
