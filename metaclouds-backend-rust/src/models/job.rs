//! Job 作业模型（对齐 Go `models/job.go`）。
//!
//! 落库表为 `migrations/006_b3_jobs.sql` 建的 `jobs`，字段与 Go GORM 模型逐列对齐：
//! 核心资源请求（gpus/cpus/memory/duration/progress）+ 调度/分片字段（partition /
//! scheduler / gpu_fraction / qos / nodes_requested / node_selector / affinity /
//! tolerations）+ 弹性/容错/检查点字段（elastic_* / checkpoint_* / retries /
//! fault_tolerance_level / topology_* / network_requirement）。
//!
//! `type` 是 SQL 关键字：Rust 字段名用 `kind`，通过 `#[sqlx(rename = "type")]` 映射列，
//! 对外 JSON 仍序列化为 `"type"`（对齐 Go `json:"type"`）。
//! `deleted_at` 在对外视图中剔除（对齐 Go `gorm.DeletedAt` + `json:"-"`）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, PaginatedResult, PaginationParams, SoftDelete,
};

/// 作业状态常量（对齐 Go `Status` 注释：pending, running, completed, failed, cancelled）。
pub mod status {
    pub const PENDING: &str = "pending";
    pub const RUNNING: &str = "running";
    pub const COMPLETED: &str = "completed";
    pub const FAILED: &str = "failed";
    pub const CANCELLED: &str = "cancelled";
}

/// 作业类型常量（对齐 Go `Type` 注释：training, inference, batch）。
pub mod kind {
    pub const TRAINING: &str = "training";
    pub const INFERENCE: &str = "inference";
    pub const BATCH: &str = "batch";
}

/// `jobs` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Job {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub cluster_id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: String,
    pub status: String,
    /// DB 列名为 `type`（SQL 关键字），Rust 字段名用 `kind` 规避。
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub kind: String,
    pub priority: i64,
    pub gpus: i64,
    pub cpus: i64,
    pub memory: i64,
    pub duration: i64,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub progress: i64,
    pub output_path: String,
    pub error_msg: String,
    // P0/P1 对齐：调度 / 分片 / GPU 细粒度
    pub partition_id: i64,
    pub scheduler_type: String,
    pub scheduler_job_id: String,
    pub gpu_fraction: f64,
    pub gpu_memory_gb: i64,
    pub gpu_vendor: String,
    pub qos: String,
    pub nodes_requested: i64,
    pub node_selector: String,
    pub affinity: String,
    pub tolerations: String,
    // 弹性 / 容错 / 检查点
    pub elastic_enabled: bool,
    pub min_gpus: i64,
    pub max_gpus: i64,
    pub scaling_policy: String,
    pub checkpoint_enabled: bool,
    pub checkpoint_interval_minutes: i64,
    pub max_retries: i64,
    pub retry_count: i64,
    pub fault_tolerance_level: String,
    pub topology_affinity: String,
    pub topology_anti_affinity: bool,
    pub network_requirement: String,
}

impl HasTimestamps for Job {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for Job {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外作业视图（剔除软删除列 `deleted_at`，对齐 Go `json:"-"`）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub cluster_id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: String,
    pub status: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub priority: i64,
    pub gpus: i64,
    pub cpus: i64,
    pub memory: i64,
    pub duration: i64,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub progress: i64,
    pub output_path: String,
    pub error_msg: String,
    pub partition_id: i64,
    pub scheduler_type: String,
    pub scheduler_job_id: String,
    pub gpu_fraction: f64,
    pub gpu_memory_gb: i64,
    pub gpu_vendor: String,
    pub qos: String,
    pub nodes_requested: i64,
    pub node_selector: String,
    pub affinity: String,
    pub tolerations: String,
    pub elastic_enabled: bool,
    pub min_gpus: i64,
    pub max_gpus: i64,
    pub scaling_policy: String,
    pub checkpoint_enabled: bool,
    pub checkpoint_interval_minutes: i64,
    pub max_retries: i64,
    pub retry_count: i64,
    pub fault_tolerance_level: String,
    pub topology_affinity: String,
    pub topology_anti_affinity: bool,
    pub network_requirement: String,
}

impl From<Job> for JobResponse {
    fn from(j: Job) -> Self {
        Self {
            id: j.id,
            created_at: j.created_at,
            updated_at: j.updated_at,
            cluster_id: j.cluster_id,
            tenant_id: j.tenant_id,
            user_id: j.user_id,
            name: j.name,
            description: j.description,
            status: j.status,
            kind: j.kind,
            priority: j.priority,
            gpus: j.gpus,
            cpus: j.cpus,
            memory: j.memory,
            duration: j.duration,
            start_time: j.start_time,
            end_time: j.end_time,
            progress: j.progress,
            output_path: j.output_path,
            error_msg: j.error_msg,
            partition_id: j.partition_id,
            scheduler_type: j.scheduler_type,
            scheduler_job_id: j.scheduler_job_id,
            gpu_fraction: j.gpu_fraction,
            gpu_memory_gb: j.gpu_memory_gb,
            gpu_vendor: j.gpu_vendor,
            qos: j.qos,
            nodes_requested: j.nodes_requested,
            node_selector: j.node_selector,
            affinity: j.affinity,
            tolerations: j.tolerations,
            elastic_enabled: j.elastic_enabled,
            min_gpus: j.min_gpus,
            max_gpus: j.max_gpus,
            scaling_policy: j.scaling_policy,
            checkpoint_enabled: j.checkpoint_enabled,
            checkpoint_interval_minutes: j.checkpoint_interval_minutes,
            max_retries: j.max_retries,
            retry_count: j.retry_count,
            fault_tolerance_level: j.fault_tolerance_level,
            topology_affinity: j.topology_affinity,
            topology_anti_affinity: j.topology_anti_affinity,
            network_requirement: j.network_requirement,
        }
    }
}

/// INSERT 作业入参（核心字段；弹性/容错/检查点字段走默认值，后续由 update 调整）。
#[derive(Debug, Clone)]
pub struct NewJob {
    pub cluster_id: i64,
    pub tenant_id: i64,
    pub user_id: i64,
    pub name: String,
    pub description: String,
    pub kind: String,
    pub priority: i64,
    pub gpus: i64,
    pub cpus: i64,
    pub memory: i64,
    pub duration: i64,
}

/// INSERT 作业（自动时间戳，初始状态 pending）。
pub async fn create(pool: &SqlitePool, input: NewJob) -> AppResult<Job> {
    let now = Utc::now();
    let res = sqlx::query(
        "INSERT INTO jobs (created_at, updated_at, cluster_id, tenant_id, user_id, name, \
         description, status, type, priority, gpus, cpus, memory, duration, progress) \
         VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?, ?, ?, ?, ?, ?, 0)",
    )
    .bind(now)
    .bind(now)
    .bind(input.cluster_id)
    .bind(input.tenant_id)
    .bind(input.user_id)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.kind)
    .bind(input.priority)
    .bind(input.gpus)
    .bind(input.cpus)
    .bind(input.memory)
    .bind(input.duration)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted job not found"))
}

/// 按 id 查询（默认排除软删除）。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<Job>> {
    let sql = if include_deleted {
        "SELECT * FROM jobs WHERE id = ?"
    } else {
        "SELECT * FROM jobs WHERE id = ? AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 过滤条件（全部可选；None 表示不按该维度过滤）。
#[derive(Debug, Clone, Default)]
pub struct JobFilter<'a> {
    pub status: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub cluster_id: Option<i64>,
    pub user_id: Option<i64>,
    pub tenant_id: Option<i64>,
    pub search: Option<&'a str>,
}

/// 分页列表（可按 status/type/cluster_id/user_id/tenant_id 过滤，按 name 搜索）。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    filter: JobFilter<'_>,
) -> AppResult<PaginatedResult<Job>> {
    let params = params.normalize();

    // 动态 WHERE：软删除过滤 + 可选 status / type / cluster_id / user_id / tenant_id / name LIKE。
    // 全匿名 `?` 占位符，bind 顺序与 where_parts 顺序一致。
    let mut where_parts: Vec<&str> = vec!["deleted_at IS NULL"];
    if filter.status.is_some() {
        where_parts.push("status = ?");
    }
    if filter.kind.is_some() {
        where_parts.push("type = ?");
    }
    if filter.cluster_id.is_some() {
        where_parts.push("cluster_id = ?");
    }
    if filter.user_id.is_some() {
        where_parts.push("user_id = ?");
    }
    if filter.tenant_id.is_some() {
        where_parts.push("tenant_id = ?");
    }
    if filter.search.is_some() {
        where_parts.push("name LIKE ?");
    }
    let where_clause = where_parts.join(" AND ");

    let mut binds: Vec<String> = Vec::new();
    if let Some(v) = filter.status {
        binds.push(v.to_string());
    }
    if let Some(v) = filter.kind {
        binds.push(v.to_string());
    }
    if let Some(v) = filter.cluster_id {
        binds.push(v.to_string());
    }
    if let Some(v) = filter.user_id {
        binds.push(v.to_string());
    }
    if let Some(v) = filter.tenant_id {
        binds.push(v.to_string());
    }
    if let Some(s) = filter.search {
        binds.push(format!("%{s}%"));
    }

    let count_sql = format!("SELECT COUNT(*) FROM jobs WHERE {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for b in &binds {
        count_q = count_q.bind(b);
    }
    let total: i64 = count_q.fetch_one(pool).await?;

    let list_sql =
        format!("SELECT * FROM jobs WHERE {where_clause} ORDER BY id ASC LIMIT ? OFFSET ?");
    let mut list_q = sqlx::query_as::<_, Job>(&list_sql);
    for b in &binds {
        list_q = list_q.bind(b);
    }
    list_q = list_q.bind(params.limit()).bind(params.offset());
    let rows: Vec<Job> = list_q.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}

/// 按状态统计数量（过滤条件之外，跨状态聚合；用于 GET /jobs/stats）。
pub async fn count_by_status(
    pool: &SqlitePool,
    tenant_id: Option<i64>,
) -> AppResult<Vec<(String, i64)>> {
    let rows: Vec<(String, i64)> = if let Some(tid) = tenant_id {
        sqlx::query_as(
            "SELECT status, COUNT(*) FROM jobs WHERE deleted_at IS NULL AND tenant_id = ? \
             GROUP BY status",
        )
        .bind(tid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as("SELECT status, COUNT(*) FROM jobs WHERE deleted_at IS NULL GROUP BY status")
            .fetch_all(pool)
            .await?
    };
    Ok(rows)
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("jobs"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
