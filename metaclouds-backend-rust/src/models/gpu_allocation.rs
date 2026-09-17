//! GPUAllocation GPU 分配记录模型（对齐 Go `models/gpu_allocation.go`）。
//!
//! 落库表为 `migrations/006_b3_jobs.sql` 建的 `gpu_allocations`，字段与 Go GORM 模型逐列对齐：
//! device_id / job_id / tenant_id / user_id / fraction / memory_gb / mig_profile /
//! status(active|released|failed) / started_at / ended_at。
//!
//! 分配记录不对外暴露 `deleted_at`（软删除列保留以对齐 GORM，但业务上直接标记 released）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::AppResult;
use crate::orm::{PaginatedResult, PaginationParams};

/// 分配状态常量（对齐 Go：active, released, failed）。
pub mod status {
    pub const ACTIVE: &str = "active";
    pub const RELEASED: &str = "released";
    pub const FAILED: &str = "failed";
}

/// `gpu_allocations` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GpuAllocation {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub device_id: i64,
    pub job_id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub fraction: f64,
    pub memory_gb: i64,
    pub mig_profile: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
}

/// 对外分配记录视图（剔除软删除列 `deleted_at`）。
#[derive(utoipa::ToSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpuAllocationResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub device_id: i64,
    pub job_id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub fraction: f64,
    pub memory_gb: i64,
    pub mig_profile: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
}

impl From<GpuAllocation> for GpuAllocationResponse {
    fn from(a: GpuAllocation) -> Self {
        Self {
            id: a.id,
            created_at: a.created_at,
            updated_at: a.updated_at,
            device_id: a.device_id,
            job_id: a.job_id,
            tenant_id: a.tenant_id,
            user_id: a.user_id,
            fraction: a.fraction,
            memory_gb: a.memory_gb,
            mig_profile: a.mig_profile,
            status: a.status,
            started_at: a.started_at,
            ended_at: a.ended_at,
        }
    }
}

/// 按 id 查询（分配记录不做软删除过滤，按 id 精确取）。
pub async fn get_by_id(pool: &SqlitePool, id: i64) -> AppResult<Option<GpuAllocation>> {
    Ok(sqlx::query_as("SELECT * FROM gpu_allocations WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?)
}

/// 过滤条件。
#[derive(Debug, Clone, Default)]
pub struct GpuAllocationFilter<'a> {
    pub job_id: Option<i64>,
    pub user_id: Option<i64>,
    pub status: Option<&'a str>,
}

/// 分页列表（可按 job_id/user_id/status 过滤）。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    filter: GpuAllocationFilter<'_>,
) -> AppResult<PaginatedResult<GpuAllocation>> {
    let params = params.normalize();

    let mut where_parts: Vec<&str> = vec!["1=1"];
    if filter.job_id.is_some() {
        where_parts.push("job_id = ?");
    }
    if filter.user_id.is_some() {
        where_parts.push("user_id = ?");
    }
    if filter.status.is_some() {
        where_parts.push("status = ?");
    }
    let where_clause = where_parts.join(" AND ");

    let mut binds: Vec<String> = Vec::new();
    if let Some(v) = filter.job_id {
        binds.push(v.to_string());
    }
    if let Some(v) = filter.user_id {
        binds.push(v.to_string());
    }
    if let Some(v) = filter.status {
        binds.push(v.to_string());
    }

    let count_sql = format!("SELECT COUNT(*) FROM gpu_allocations WHERE {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for b in &binds {
        count_q = count_q.bind(b);
    }
    let total: i64 = count_q.fetch_one(pool).await?;

    let list_sql = format!(
        "SELECT * FROM gpu_allocations WHERE {where_clause} ORDER BY id ASC LIMIT ? OFFSET ?"
    );
    let mut list_q = sqlx::query_as::<_, GpuAllocation>(&list_sql);
    for b in &binds {
        list_q = list_q.bind(b);
    }
    list_q = list_q.bind(params.limit()).bind(params.offset());
    let rows: Vec<GpuAllocation> = list_q.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}
