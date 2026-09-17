//! Cluster 模型（对齐 Go models/cluster.go）。
//!
//! 字段：id / name(unique) / description / status / 节点与 GPU 规格 / 网络位置 /
//! 多 GPU 厂商与联邦字段。`gpu_vendors` / `scheduler_types` 使用 [`crate::orm::Json`]
//! 包装（对齐 GORM `type:json` tag）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, Json, PaginatedResult, PaginationParams, SoftDelete,
};

/// `clusters` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Cluster {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub status: String,
    pub nodes: i64,
    pub gpus: i64,
    pub cpus: i64,
    pub memory: i64,
    pub storage: i64,
    pub network_type: String,
    pub location: String,
    /// GPU 厂商列表，以 JSON TEXT 落库（如 `["nvidia","amd"]`）。
    pub gpu_vendors: Json<Vec<String>>,
    /// 调度器类型列表，以 JSON TEXT 落库。
    pub scheduler_types: Json<Vec<String>>,
    pub multi_cluster_enabled: bool,
    pub federation_id: String,
}

impl HasTimestamps for Cluster {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for Cluster {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外集群视图（剔除软删除列 `deleted_at`，对齐 Go `json:"-"`）。
///
/// `gpu_vendors` / `scheduler_types` 解包为裸 JSON（对齐 Go 侧本就是 JSON 字符串字段）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub description: String,
    pub status: String,
    pub nodes: i64,
    pub gpus: i64,
    pub cpus: i64,
    pub memory: i64,
    pub storage: i64,
    pub network_type: String,
    pub location: String,
    pub gpu_vendors: Vec<String>,
    pub scheduler_types: Vec<String>,
    pub multi_cluster_enabled: bool,
    pub federation_id: String,
}

impl From<Cluster> for ClusterResponse {
    fn from(c: Cluster) -> Self {
        Self {
            id: c.id,
            created_at: c.created_at,
            updated_at: c.updated_at,
            name: c.name,
            description: c.description,
            status: c.status,
            nodes: c.nodes,
            gpus: c.gpus,
            cpus: c.cpus,
            memory: c.memory,
            storage: c.storage,
            network_type: c.network_type,
            location: c.location,
            gpu_vendors: c.gpu_vendors.0,
            scheduler_types: c.scheduler_types.0,
            multi_cluster_enabled: c.multi_cluster_enabled,
            federation_id: c.federation_id,
        }
    }
}

/// 创建集群入参。
pub struct NewCluster<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub status: &'a str,
    pub nodes: i64,
    pub gpus: i64,
    pub cpus: i64,
    pub memory: i64,
    pub storage: i64,
    pub network_type: &'a str,
    pub location: &'a str,
    pub gpu_vendors: Vec<String>,
    pub scheduler_types: Vec<String>,
    pub multi_cluster_enabled: bool,
    pub federation_id: &'a str,
}

/// INSERT 集群（自动时间戳）。
pub async fn create(pool: &SqlitePool, input: NewCluster<'_>) -> AppResult<Cluster> {
    let mut cluster = Cluster {
        id: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        name: input.name.to_string(),
        description: input.description.to_string(),
        status: input.status.to_string(),
        nodes: input.nodes,
        gpus: input.gpus,
        cpus: input.cpus,
        memory: input.memory,
        storage: input.storage,
        network_type: input.network_type.to_string(),
        location: input.location.to_string(),
        gpu_vendors: Json(input.gpu_vendors),
        scheduler_types: Json(input.scheduler_types),
        multi_cluster_enabled: input.multi_cluster_enabled,
        federation_id: input.federation_id.to_string(),
    };
    cluster.before_insert();

    let res = sqlx::query(
        "INSERT INTO clusters (created_at, updated_at, name, description, status, \
         nodes, gpus, cpus, memory, storage, network_type, location, \
         gpu_vendors, scheduler_types, multi_cluster_enabled, federation_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
    )
    .bind(cluster.created_at)
    .bind(cluster.updated_at)
    .bind(&cluster.name)
    .bind(&cluster.description)
    .bind(&cluster.status)
    .bind(cluster.nodes)
    .bind(cluster.gpus)
    .bind(cluster.cpus)
    .bind(cluster.memory)
    .bind(cluster.storage)
    .bind(&cluster.network_type)
    .bind(&cluster.location)
    .bind(&cluster.gpu_vendors)
    .bind(&cluster.scheduler_types)
    .bind(cluster.multi_cluster_enabled)
    .bind(&cluster.federation_id)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted cluster not found"))
}

/// 按 id 查询（默认排除软删除）。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<Cluster>> {
    let sql = if include_deleted {
        "SELECT * FROM clusters WHERE id = ?1"
    } else {
        "SELECT * FROM clusters WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 分页列表。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    include_deleted: bool,
) -> AppResult<PaginatedResult<Cluster>> {
    let params = params.normalize();
    let where_clause = if include_deleted {
        ""
    } else {
        " WHERE deleted_at IS NULL"
    };

    let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM clusters{where_clause}"))
        .fetch_one(pool)
        .await?;

    let rows: Vec<Cluster> = sqlx::query_as(&format!(
        "SELECT * FROM clusters{where_clause} ORDER BY id ASC LIMIT ?1 OFFSET ?2"
    ))
    .bind(params.limit())
    .bind(params.offset())
    .fetch_all(pool)
    .await?;

    Ok(PaginatedResult::new(rows, total, params))
}

/// 更新集群状态（自动刷 updated_at）。
pub async fn update_status(pool: &SqlitePool, id: i64, status: &str) -> AppResult<Option<Cluster>> {
    let mut existing = match get_by_id(pool, id, false).await? {
        Some(c) => c,
        None => return Ok(None),
    };
    existing.status = status.to_string();
    existing.before_update();

    sqlx::query("UPDATE clusters SET status = ?1, updated_at = ?2 WHERE id = ?3")
        .bind(&existing.status)
        .bind(existing.updated_at)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(Some(existing))
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("clusters"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
