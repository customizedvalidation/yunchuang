//! Resource 资源模型（对齐 Go `models/resource.go`）。
//!
//! 落库表为 `migrations/002_tenants_clusters_resources.sql` 已建的 `resources`，
//! 字段与 Go GORM 模型逐列对齐：cluster_id 外键 / type(gpu, cpu, memory,
//! storage, network) / status / total·used·available·utilization /
//! 多 GPU 厂商与显存字段（vendor / gpu_model / vram_* / mig_enabled）。
//!
//! `deleted_at` 在对外视图中剔除（对齐 Go `gorm.DeletedAt` + `json:"-"`）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, PaginatedResult, PaginationParams, SoftDelete,
};

/// 资源类型枚举（对齐 Go `Type` 注释：gpu, cpu, memory, storage, network）。
pub mod resource_type {
    pub const GPU: &str = "gpu";
    pub const CPU: &str = "cpu";
    pub const MEMORY: &str = "memory";
    pub const STORAGE: &str = "storage";
    pub const NETWORK: &str = "network";
}

/// `resources` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Resource {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub cluster_id: i64,
    /// DB 列名为 `type`（SQL 关键字），Rust 字段名用 `kind` 规避；FromRow 按列名映射。
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub status: String,
    pub total: i64,
    pub used: i64,
    pub available: i64,
    pub utilization: f64,
    pub details: String,
    pub vendor: String,
    pub gpu_model: String,
    pub vram_total_mb: i64,
    pub vram_used_mb: i64,
    pub vram_oversubscription_ratio: f64,
    pub mig_enabled: bool,
}

impl HasTimestamps for Resource {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for Resource {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外资源视图（剔除软删除列 `deleted_at`，对齐 Go `json:"-"`）。
#[derive(utoipa::ToSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub cluster_id: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    pub status: String,
    pub total: i64,
    pub used: i64,
    pub available: i64,
    pub utilization: f64,
    pub details: String,
    pub vendor: String,
    pub gpu_model: String,
    pub vram_total_mb: i64,
    pub vram_used_mb: i64,
    pub vram_oversubscription_ratio: f64,
    pub mig_enabled: bool,
}

impl From<Resource> for ResourceResponse {
    fn from(r: Resource) -> Self {
        Self {
            id: r.id,
            created_at: r.created_at,
            updated_at: r.updated_at,
            cluster_id: r.cluster_id,
            kind: r.kind,
            name: r.name,
            status: r.status,
            total: r.total,
            used: r.used,
            available: r.available,
            utilization: r.utilization,
            details: r.details,
            vendor: r.vendor,
            gpu_model: r.gpu_model,
            vram_total_mb: r.vram_total_mb,
            vram_used_mb: r.vram_used_mb,
            vram_oversubscription_ratio: r.vram_oversubscription_ratio,
            mig_enabled: r.mig_enabled,
        }
    }
}

/// INSERT 资源入参。
pub struct NewResource<'a> {
    pub cluster_id: i64,
    pub kind: &'a str,
    pub name: &'a str,
    pub status: &'a str,
    pub total: i64,
    pub used: i64,
    pub available: i64,
    pub utilization: f64,
    pub details: &'a str,
    pub vendor: &'a str,
    pub gpu_model: &'a str,
    pub vram_total_mb: i64,
    pub vram_used_mb: i64,
    pub vram_oversubscription_ratio: f64,
    pub mig_enabled: bool,
}

/// INSERT 资源（自动时间戳）。
pub async fn create(pool: &SqlitePool, input: NewResource<'_>) -> AppResult<Resource> {
    let mut resource = Resource {
        id: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        cluster_id: input.cluster_id,
        kind: input.kind.to_string(),
        name: input.name.to_string(),
        status: input.status.to_string(),
        total: input.total,
        used: input.used,
        available: input.available,
        utilization: input.utilization,
        details: input.details.to_string(),
        vendor: input.vendor.to_string(),
        gpu_model: input.gpu_model.to_string(),
        vram_total_mb: input.vram_total_mb,
        vram_used_mb: input.vram_used_mb,
        vram_oversubscription_ratio: input.vram_oversubscription_ratio,
        mig_enabled: input.mig_enabled,
    };
    resource.before_insert();

    let res = sqlx::query(
        "INSERT INTO resources (created_at, updated_at, cluster_id, type, name, status, \
         total, used, available, utilization, details, vendor, gpu_model, \
         vram_total_mb, vram_used_mb, vram_oversubscription_ratio, mig_enabled) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
    )
    .bind(resource.created_at)
    .bind(resource.updated_at)
    .bind(resource.cluster_id)
    .bind(&resource.kind)
    .bind(&resource.name)
    .bind(&resource.status)
    .bind(resource.total)
    .bind(resource.used)
    .bind(resource.available)
    .bind(resource.utilization)
    .bind(&resource.details)
    .bind(&resource.vendor)
    .bind(&resource.gpu_model)
    .bind(resource.vram_total_mb)
    .bind(resource.vram_used_mb)
    .bind(resource.vram_oversubscription_ratio)
    .bind(resource.mig_enabled)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted resource not found"))
}

/// 按 id 查询（默认排除软删除）。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<Resource>> {
    let sql = if include_deleted {
        "SELECT * FROM resources WHERE id = ?1"
    } else {
        "SELECT * FROM resources WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 分页列表（可按 type / cluster_id 过滤，可按 name 搜索）。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    kind: Option<&str>,
    cluster_id: Option<i64>,
    search: Option<&str>,
) -> AppResult<PaginatedResult<Resource>> {
    let params = params.normalize();

    // 动态 WHERE：软删除过滤 + 可选 type / cluster_id / name LIKE。
    // 使用匿名 `?` 占位符，bind 顺序与 where_parts 顺序一致（最后再 bind limit/offset）。
    let mut where_parts: Vec<&str> = vec!["deleted_at IS NULL"];
    if kind.is_some() {
        where_parts.push("type = ?");
    }
    if cluster_id.is_some() {
        where_parts.push("cluster_id = ?");
    }
    if search.is_some() {
        where_parts.push("name LIKE ?");
    }
    let where_clause = where_parts.join(" AND ");

    // 收集过滤绑定值，保持与 where_parts 中可选项的顺序一致。
    let mut filter_binds: Vec<String> = Vec::new();
    if let Some(k) = kind {
        filter_binds.push(k.to_string());
    }
    if let Some(c) = cluster_id {
        filter_binds.push(c.to_string());
    }
    if let Some(s) = search {
        filter_binds.push(format!("%{s}%"));
    }

    let count_sql = format!("SELECT COUNT(*) FROM resources WHERE {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for b in &filter_binds {
        count_q = count_q.bind(b);
    }
    let total: i64 = count_q.fetch_one(pool).await?;

    let list_sql =
        format!("SELECT * FROM resources WHERE {where_clause} ORDER BY id ASC LIMIT ? OFFSET ?");
    let mut list_q = sqlx::query_as::<_, Resource>(&list_sql);
    for b in &filter_binds {
        list_q = list_q.bind(b);
    }
    list_q = list_q.bind(params.limit()).bind(params.offset());
    let rows: Vec<Resource> = list_q.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("resources"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
