//! NodeTopology 节点拓扑模型（WP-P2-B2）。
//!
//! 落库表为 `migrations/004_b2_resources.sql` 新建的 `topology_nodes`。
//! 字段：cluster_id 外键 / hostname / ip / role(worker|master) /
//! cpu_cores / memory_gb / gpu_count / gpu_model / status / labels(JSON TEXT)。
//!
//! `labels` 使用 [`crate::orm::Json`] 包装（SQLite 落库为 TEXT JSON）。
//! `deleted_at` 对外视图剔除（对齐 GORM `gorm.DeletedAt` + `json:"-"`）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, Json, PaginatedResult, PaginationParams, SoftDelete,
};

/// 节点角色枚举（worker / master）。
pub mod node_role {
    pub const MASTER: &str = "master";
    pub const WORKER: &str = "worker";
}

/// `topology_nodes` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct NodeTopology {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub cluster_id: i64,
    pub hostname: String,
    pub ip: String,
    pub role: String,
    pub cpu_cores: i64,
    pub memory_gb: i64,
    pub gpu_count: i64,
    pub gpu_model: String,
    pub status: String,
    /// 节点标签（K8s labels 语义），以 JSON TEXT 落库。
    pub labels: Json<serde_json::Value>,
}

impl HasTimestamps for NodeTopology {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for NodeTopology {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外节点视图（剔除 `deleted_at`）。
#[derive(utoipa::ToSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TopologyResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub cluster_id: i64,
    pub hostname: String,
    pub ip: String,
    pub role: String,
    pub cpu_cores: i64,
    pub memory_gb: i64,
    pub gpu_count: i64,
    pub gpu_model: String,
    pub status: String,
    pub labels: serde_json::Value,
}

impl From<NodeTopology> for TopologyResponse {
    fn from(n: NodeTopology) -> Self {
        Self {
            id: n.id,
            created_at: n.created_at,
            updated_at: n.updated_at,
            cluster_id: n.cluster_id,
            hostname: n.hostname,
            ip: n.ip,
            role: n.role,
            cpu_cores: n.cpu_cores,
            memory_gb: n.memory_gb,
            gpu_count: n.gpu_count,
            gpu_model: n.gpu_model,
            status: n.status,
            labels: n.labels.0,
        }
    }
}

/// INSERT 节点入参。
pub struct NewNode<'a> {
    pub cluster_id: i64,
    pub hostname: &'a str,
    pub ip: &'a str,
    pub role: &'a str,
    pub cpu_cores: i64,
    pub memory_gb: i64,
    pub gpu_count: i64,
    pub gpu_model: &'a str,
    pub status: &'a str,
    pub labels: serde_json::Value,
}

/// INSERT 节点（自动时间戳）。
pub async fn create(pool: &SqlitePool, input: NewNode<'_>) -> AppResult<NodeTopology> {
    let mut node = NodeTopology {
        id: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        cluster_id: input.cluster_id,
        hostname: input.hostname.to_string(),
        ip: input.ip.to_string(),
        role: input.role.to_string(),
        cpu_cores: input.cpu_cores,
        memory_gb: input.memory_gb,
        gpu_count: input.gpu_count,
        gpu_model: input.gpu_model.to_string(),
        status: input.status.to_string(),
        labels: Json(input.labels),
    };
    node.before_insert();

    let res = sqlx::query(
        "INSERT INTO topology_nodes (created_at, updated_at, cluster_id, hostname, ip, role, \
         cpu_cores, memory_gb, gpu_count, gpu_model, status, labels) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )
    .bind(node.created_at)
    .bind(node.updated_at)
    .bind(node.cluster_id)
    .bind(&node.hostname)
    .bind(&node.ip)
    .bind(&node.role)
    .bind(node.cpu_cores)
    .bind(node.memory_gb)
    .bind(node.gpu_count)
    .bind(&node.gpu_model)
    .bind(&node.status)
    .bind(&node.labels)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted node not found"))
}

/// 按 id 查询（默认排除软删除）。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<NodeTopology>> {
    let sql = if include_deleted {
        "SELECT * FROM topology_nodes WHERE id = ?1"
    } else {
        "SELECT * FROM topology_nodes WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 分页列表（可按 cluster_id / role 过滤）。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    cluster_id: Option<i64>,
    role: Option<&str>,
) -> AppResult<PaginatedResult<NodeTopology>> {
    let params = params.normalize();

    let mut where_parts: Vec<&str> = vec!["deleted_at IS NULL"];
    if cluster_id.is_some() {
        where_parts.push("cluster_id = ?");
    }
    if role.is_some() {
        where_parts.push("role = ?");
    }
    let where_clause = where_parts.join(" AND ");

    let mut filter_binds: Vec<String> = Vec::new();
    if let Some(c) = cluster_id {
        filter_binds.push(c.to_string());
    }
    if let Some(r) = role {
        filter_binds.push(r.to_string());
    }

    let count_sql = format!("SELECT COUNT(*) FROM topology_nodes WHERE {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for b in &filter_binds {
        count_q = count_q.bind(b);
    }
    let total: i64 = count_q.fetch_one(pool).await?;

    let list_sql = format!(
        "SELECT * FROM topology_nodes WHERE {where_clause} ORDER BY id ASC LIMIT ? OFFSET ?"
    );
    let mut list_q = sqlx::query_as::<_, NodeTopology>(&list_sql);
    for b in &filter_binds {
        list_q = list_q.bind(b);
    }
    list_q = list_q.bind(params.limit()).bind(params.offset());
    let rows: Vec<NodeTopology> = list_q.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("topology_nodes"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
