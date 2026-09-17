//! Partition 分区模型（对齐 WP-P2-B4 调度域规格）。
//!
//! 落库表为 `migrations/007_b4_scheduler.sql` 建的 `partitions`：
//! cluster_id 外键 / partition_type(exclusive|shared|reserved) / gpu_count·cpu_cores·memory_gb /
//! status(active|inactive|maintenance) / node_selector·labels 以 JSON TEXT 落库 / tenant_id 可空。
//!
//! `deleted_at` 在对外视图中剔除（对齐 GORM `gorm.DeletedAt` + `json:"-"`）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, Json, PaginatedResult, PaginationParams, SoftDelete,
};
use serde_json::Value;

/// 分区类型枚举常量（对齐 partition_type：exclusive / shared / reserved）。
pub mod partition_type {
    pub const EXCLUSIVE: &str = "exclusive";
    pub const SHARED: &str = "shared";
    pub const RESERVED: &str = "reserved";
}

/// 分区状态枚举常量（对齐 status：active / inactive / maintenance）。
pub mod partition_status {
    pub const ACTIVE: &str = "active";
    pub const INACTIVE: &str = "inactive";
    pub const MAINTENANCE: &str = "maintenance";
}

/// `partitions` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Partition {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub cluster_id: i64,
    pub name: String,
    pub description: String,
    pub partition_type: String,
    pub gpu_count: i64,
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub status: String,
    /// 节点选择器，以 JSON TEXT 落库（如 `{"gpu":"nvidia"}`）。
    pub node_selector: Json<Value>,
    /// 分区标签，以 JSON TEXT 落库。
    pub labels: Json<Value>,
    pub tenant_id: Option<i64>,
}

impl HasTimestamps for Partition {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for Partition {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外分区视图（剔除软删除列 `deleted_at`，对齐 GORM `json:"-"`）。
#[derive(utoipa::ToSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PartitionResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub cluster_id: i64,
    pub name: String,
    pub description: String,
    pub partition_type: String,
    pub gpu_count: i64,
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub status: String,
    pub node_selector: Value,
    pub labels: Value,
    pub tenant_id: Option<i64>,
}

impl From<Partition> for PartitionResponse {
    fn from(p: Partition) -> Self {
        Self {
            id: p.id,
            created_at: p.created_at,
            updated_at: p.updated_at,
            cluster_id: p.cluster_id,
            name: p.name,
            description: p.description,
            partition_type: p.partition_type,
            gpu_count: p.gpu_count,
            cpu_cores: p.cpu_cores,
            memory_gb: p.memory_gb,
            status: p.status,
            node_selector: p.node_selector.0,
            labels: p.labels.0,
            tenant_id: p.tenant_id,
        }
    }
}

/// INSERT 分区入参。
pub struct NewPartition {
    pub cluster_id: i64,
    pub name: String,
    pub description: String,
    pub partition_type: String,
    pub gpu_count: i64,
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub status: String,
    pub node_selector: Value,
    pub labels: Value,
    pub tenant_id: Option<i64>,
}

/// INSERT 分区（自动时间戳）。
pub async fn create(pool: &SqlitePool, input: NewPartition) -> AppResult<Partition> {
    let now = Utc::now();
    let res = sqlx::query(
        "INSERT INTO partitions (created_at, updated_at, cluster_id, name, description, \
         partition_type, gpu_count, cpu_cores, memory_gb, status, node_selector, labels, tenant_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
    )
    .bind(now)
    .bind(now)
    .bind(input.cluster_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.partition_type)
    .bind(input.gpu_count)
    .bind(input.cpu_cores)
    .bind(input.memory_gb)
    .bind(&input.status)
    .bind(Json(input.node_selector))
    .bind(Json(input.labels))
    .bind(input.tenant_id)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted partition not found"))
}

/// 按 id 查询（默认排除软删除）。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<Partition>> {
    let sql = if include_deleted {
        "SELECT * FROM partitions WHERE id = ?1"
    } else {
        "SELECT * FROM partitions WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("partitions"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

/// 分页列表（动态 WHERE：软删除 + 可选 cluster_id / status / partition_type / name LIKE）。
#[allow(clippy::too_many_arguments)]
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    cluster_id: Option<i64>,
    status: Option<&str>,
    partition_type: Option<&str>,
    search: Option<&str>,
) -> AppResult<PaginatedResult<Partition>> {
    let params = params.normalize();

    let mut where_parts: Vec<&str> = vec!["deleted_at IS NULL"];
    if cluster_id.is_some() {
        where_parts.push("cluster_id = ?");
    }
    if status.is_some() {
        where_parts.push("status = ?");
    }
    if partition_type.is_some() {
        where_parts.push("partition_type = ?");
    }
    if search.is_some() {
        where_parts.push("name LIKE ?");
    }
    let where_clause = where_parts.join(" AND ");

    let mut filter_binds: Vec<String> = Vec::new();
    if let Some(c) = cluster_id {
        filter_binds.push(c.to_string());
    }
    if let Some(s) = status {
        filter_binds.push(s.to_string());
    }
    if let Some(t) = partition_type {
        filter_binds.push(t.to_string());
    }
    if let Some(s) = search {
        filter_binds.push(format!("%{s}%"));
    }

    let count_sql = format!("SELECT COUNT(*) FROM partitions WHERE {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for b in &filter_binds {
        count_q = count_q.bind(b);
    }
    let total: i64 = count_q.fetch_one(pool).await?;

    let list_sql =
        format!("SELECT * FROM partitions WHERE {where_clause} ORDER BY id ASC LIMIT ? OFFSET ?");
    let mut list_q = sqlx::query_as::<_, Partition>(&list_sql);
    for b in &filter_binds {
        list_q = list_q.bind(b);
    }
    list_q = list_q.bind(params.limit()).bind(params.offset());
    let rows: Vec<Partition> = list_q.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}
