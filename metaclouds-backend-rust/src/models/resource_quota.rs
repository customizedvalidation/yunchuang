//! ResourceQuota 多维度资源配额模型（对齐 WP-P2-B4 调度域规格）。
//!
//! 落库表为 `migrations/007_b4_scheduler.sql` 建的 `resource_quotas`：
//! tenant_id 外键 / partition_id 可空 / gpu·cpu·memory·storage 各自的 limit/used /
//! status(active|suspended)。
//!
//! `deleted_at` 在对外视图中剔除（对齐 GORM `gorm.DeletedAt` + `json:"-"`）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, PaginatedResult, PaginationParams, SoftDelete,
};

/// 配额状态枚举常量（对齐 status：active / suspended）。
pub mod quota_status {
    pub const ACTIVE: &str = "active";
    pub const SUSPENDED: &str = "suspended";
}

/// `resource_quotas` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub tenant_id: i64,
    pub partition_id: Option<i64>,
    pub gpu_limit: i64,
    pub gpu_used: i64,
    pub cpu_limit: f64,
    pub cpu_used: f64,
    pub memory_limit_gb: f64,
    pub memory_used_gb: f64,
    pub storage_limit_gb: f64,
    pub storage_used_gb: f64,
    pub status: String,
}

impl HasTimestamps for ResourceQuota {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for ResourceQuota {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外配额视图（剔除软删除列 `deleted_at`）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceQuotaResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub description: String,
    pub tenant_id: i64,
    pub partition_id: Option<i64>,
    pub gpu_limit: i64,
    pub gpu_used: i64,
    pub cpu_limit: f64,
    pub cpu_used: f64,
    pub memory_limit_gb: f64,
    pub memory_used_gb: f64,
    pub storage_limit_gb: f64,
    pub storage_used_gb: f64,
    pub status: String,
}

impl From<ResourceQuota> for ResourceQuotaResponse {
    fn from(q: ResourceQuota) -> Self {
        Self {
            id: q.id,
            created_at: q.created_at,
            updated_at: q.updated_at,
            name: q.name,
            description: q.description,
            tenant_id: q.tenant_id,
            partition_id: q.partition_id,
            gpu_limit: q.gpu_limit,
            gpu_used: q.gpu_used,
            cpu_limit: q.cpu_limit,
            cpu_used: q.cpu_used,
            memory_limit_gb: q.memory_limit_gb,
            memory_used_gb: q.memory_used_gb,
            storage_limit_gb: q.storage_limit_gb,
            storage_used_gb: q.storage_used_gb,
            status: q.status,
        }
    }
}

/// INSERT 配额入参。
pub struct NewQuota {
    pub name: String,
    pub description: String,
    pub tenant_id: i64,
    pub partition_id: Option<i64>,
    pub gpu_limit: i64,
    pub cpu_limit: f64,
    pub memory_limit_gb: f64,
    pub storage_limit_gb: f64,
    pub status: String,
}

/// INSERT 配额（自动时间戳；used 字段初始化为 0）。
pub async fn create(pool: &SqlitePool, input: NewQuota) -> AppResult<ResourceQuota> {
    let now = Utc::now();
    let res = sqlx::query(
        "INSERT INTO resource_quotas (created_at, updated_at, name, description, tenant_id, \
         partition_id, gpu_limit, gpu_used, cpu_limit, cpu_used, memory_limit_gb, memory_used_gb, \
         storage_limit_gb, storage_used_gb, status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, 0, ?9, 0, ?10, 0, ?11)",
    )
    .bind(now)
    .bind(now)
    .bind(&input.name)
    .bind(&input.description)
    .bind(input.tenant_id)
    .bind(input.partition_id)
    .bind(input.gpu_limit)
    .bind(input.cpu_limit)
    .bind(input.memory_limit_gb)
    .bind(input.storage_limit_gb)
    .bind(&input.status)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted resource quota not found"))
}

/// 按 id 查询（默认排除软删除）。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<ResourceQuota>> {
    let sql = if include_deleted {
        "SELECT * FROM resource_quotas WHERE id = ?1"
    } else {
        "SELECT * FROM resource_quotas WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("resource_quotas"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

/// 分页列表（动态 WHERE：软删除 + 可选 tenant_id / partition_id / status）。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    tenant_id: Option<i64>,
    partition_id: Option<i64>,
    status: Option<&str>,
) -> AppResult<PaginatedResult<ResourceQuota>> {
    let params = params.normalize();

    let mut where_parts: Vec<&str> = vec!["deleted_at IS NULL"];
    if tenant_id.is_some() {
        where_parts.push("tenant_id = ?");
    }
    if partition_id.is_some() {
        where_parts.push("partition_id = ?");
    }
    if status.is_some() {
        where_parts.push("status = ?");
    }
    let where_clause = where_parts.join(" AND ");

    let mut filter_binds: Vec<String> = Vec::new();
    if let Some(t) = tenant_id {
        filter_binds.push(t.to_string());
    }
    if let Some(p) = partition_id {
        filter_binds.push(p.to_string());
    }
    if let Some(s) = status {
        filter_binds.push(s.to_string());
    }

    let count_sql = format!("SELECT COUNT(*) FROM resource_quotas WHERE {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for b in &filter_binds {
        count_q = count_q.bind(b);
    }
    let total: i64 = count_q.fetch_one(pool).await?;

    let list_sql = format!(
        "SELECT * FROM resource_quotas WHERE {where_clause} ORDER BY id ASC LIMIT ? OFFSET ?"
    );
    let mut list_q = sqlx::query_as::<_, ResourceQuota>(&list_sql);
    for b in &filter_binds {
        list_q = list_q.bind(b);
    }
    list_q = list_q.bind(params.limit()).bind(params.offset());
    let rows: Vec<ResourceQuota> = list_q.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}
