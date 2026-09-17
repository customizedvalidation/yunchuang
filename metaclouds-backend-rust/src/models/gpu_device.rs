//! GPUDevice GPU 细粒度设备模型（对齐 Go `models/gpu_device.go`）。
//!
//! 落库表为 `migrations/006_b3_jobs.sql` 建的 `gpu_devices`，字段与 Go GORM 模型逐列对齐：
//! cluster_id / node_name / vendor / model / gpu_index(JSON "index") / 显存三件套
//! (total/allocatable/used) / MIG / driver·cuda 版本 / status / 利用率遥测。
//!
//! `deleted_at` 在对外视图中剔除（对齐 Go `gorm.DeletedAt` + `json:"-"`）。
//! `gpu_index` 列名规避 SQL 歧义，对外 JSON 仍为 `"index"`。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, PaginatedResult, PaginationParams, SoftDelete,
};

/// GPU 设备状态常量（对齐 Go：available, allocated, maintenance, fault）。
pub mod status {
    pub const AVAILABLE: &str = "available";
    pub const ALLOCATED: &str = "allocated";
    pub const MAINTENANCE: &str = "maintenance";
    pub const FAULT: &str = "fault";
}

/// `gpu_devices` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct GpuDevice {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub cluster_id: i64,
    pub node_name: String,
    pub vendor: String,
    pub model: String,
    /// 节点上的 GPU 序号；DB 列名 `gpu_index`，对外 JSON 为 `"index"`。
    #[sqlx(rename = "gpu_index")]
    #[serde(rename = "index")]
    pub index: i64,
    pub total_memory_gb: i64,
    pub allocatable_memory_gb: i64,
    pub used_memory_gb: i64,
    pub mig_enabled: bool,
    pub mig_profiles: String,
    pub driver_version: String,
    pub cuda_version: String,
    pub status: String,
    pub utilization: f64,
    pub temperature: i64,
    pub power_draw: i64,
    pub details: String,
}

impl HasTimestamps for GpuDevice {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for GpuDevice {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外 GPU 设备视图（剔除软删除列）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpuDeviceResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub cluster_id: i64,
    pub node_name: String,
    pub vendor: String,
    pub model: String,
    #[serde(rename = "index")]
    pub index: i64,
    pub total_memory_gb: i64,
    pub allocatable_memory_gb: i64,
    pub used_memory_gb: i64,
    pub mig_enabled: bool,
    pub mig_profiles: String,
    pub driver_version: String,
    pub cuda_version: String,
    pub status: String,
    pub utilization: f64,
    pub temperature: i64,
    pub power_draw: i64,
    pub details: String,
}

impl From<GpuDevice> for GpuDeviceResponse {
    fn from(d: GpuDevice) -> Self {
        Self {
            id: d.id,
            created_at: d.created_at,
            updated_at: d.updated_at,
            cluster_id: d.cluster_id,
            node_name: d.node_name,
            vendor: d.vendor,
            model: d.model,
            index: d.index,
            total_memory_gb: d.total_memory_gb,
            allocatable_memory_gb: d.allocatable_memory_gb,
            used_memory_gb: d.used_memory_gb,
            mig_enabled: d.mig_enabled,
            mig_profiles: d.mig_profiles,
            driver_version: d.driver_version,
            cuda_version: d.cuda_version,
            status: d.status,
            utilization: d.utilization,
            temperature: d.temperature,
            power_draw: d.power_draw,
            details: d.details,
        }
    }
}

/// INSERT GPU 设备入参。
#[derive(Debug, Clone)]
pub struct NewGpuDevice<'a> {
    pub cluster_id: i64,
    pub node_name: &'a str,
    pub vendor: &'a str,
    pub model: &'a str,
    pub index: i64,
    pub total_memory_gb: i64,
    pub allocatable_memory_gb: i64,
    pub used_memory_gb: i64,
    pub mig_enabled: bool,
    pub mig_profiles: &'a str,
    pub driver_version: &'a str,
    pub cuda_version: &'a str,
    pub status: &'a str,
    pub utilization: f64,
    pub temperature: i64,
    pub power_draw: i64,
    pub details: &'a str,
}

/// INSERT GPU 设备（自动时间戳）。
pub async fn create(pool: &SqlitePool, input: NewGpuDevice<'_>) -> AppResult<GpuDevice> {
    let now = Utc::now();
    let res = sqlx::query(
        "INSERT INTO gpu_devices (created_at, updated_at, cluster_id, node_name, vendor, model, \
         gpu_index, total_memory_gb, allocatable_memory_gb, used_memory_gb, mig_enabled, \
         mig_profiles, driver_version, cuda_version, status, utilization, temperature, \
         power_draw, details) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(now)
    .bind(now)
    .bind(input.cluster_id)
    .bind(input.node_name)
    .bind(input.vendor)
    .bind(input.model)
    .bind(input.index)
    .bind(input.total_memory_gb)
    .bind(input.allocatable_memory_gb)
    .bind(input.used_memory_gb)
    .bind(input.mig_enabled)
    .bind(input.mig_profiles)
    .bind(input.driver_version)
    .bind(input.cuda_version)
    .bind(input.status)
    .bind(input.utilization)
    .bind(input.temperature)
    .bind(input.power_draw)
    .bind(input.details)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted gpu device not found"))
}

/// 按 id 查询（默认排除软删除）。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<GpuDevice>> {
    let sql = if include_deleted {
        "SELECT * FROM gpu_devices WHERE id = ?"
    } else {
        "SELECT * FROM gpu_devices WHERE id = ? AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 过滤条件。
#[derive(Debug, Clone, Default)]
pub struct GpuDeviceFilter<'a> {
    pub cluster_id: Option<i64>,
    pub vendor: Option<&'a str>,
    pub status: Option<&'a str>,
}

/// 分页列表（可按 cluster_id/vendor/status 过滤）。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    filter: GpuDeviceFilter<'_>,
) -> AppResult<PaginatedResult<GpuDevice>> {
    let params = params.normalize();

    let mut where_parts: Vec<&str> = vec!["deleted_at IS NULL"];
    if filter.cluster_id.is_some() {
        where_parts.push("cluster_id = ?");
    }
    if filter.vendor.is_some() {
        where_parts.push("vendor = ?");
    }
    if filter.status.is_some() {
        where_parts.push("status = ?");
    }
    let where_clause = where_parts.join(" AND ");

    let mut binds: Vec<String> = Vec::new();
    if let Some(v) = filter.cluster_id {
        binds.push(v.to_string());
    }
    if let Some(v) = filter.vendor {
        binds.push(v.to_string());
    }
    if let Some(v) = filter.status {
        binds.push(v.to_string());
    }

    let count_sql = format!("SELECT COUNT(*) FROM gpu_devices WHERE {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for b in &binds {
        count_q = count_q.bind(b);
    }
    let total: i64 = count_q.fetch_one(pool).await?;

    let list_sql =
        format!("SELECT * FROM gpu_devices WHERE {where_clause} ORDER BY id ASC LIMIT ? OFFSET ?");
    let mut list_q = sqlx::query_as::<_, GpuDevice>(&list_sql);
    for b in &binds {
        list_q = list_q.bind(b);
    }
    list_q = list_q.bind(params.limit()).bind(params.offset());
    let rows: Vec<GpuDevice> = list_q.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("gpu_devices"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
