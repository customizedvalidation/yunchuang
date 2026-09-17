//! DistributedTrainingConfig 模型。
//!
//! 字段：id / name / description / framework(pytorch|tensorflow|mpi) /
//! worker_replicas(i32) / gpu_per_worker(i32) / cpu_per_worker(i32) /
//! memory_per_worker_gb(i32) / entrypoint / env_vars(JSON TEXT) /
//! tenant_id(FK) / created_by(FK) / status / created_at / updated_at / deleted_at。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, Json, PaginatedResult, PaginationParams, SoftDelete,
};

/// `distributed_training_configs` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct DistributedTrainingConfig {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub framework: String,
    pub worker_replicas: i32,
    pub gpu_per_worker: i32,
    pub cpu_per_worker: i32,
    pub memory_per_worker_gb: i32,
    pub entrypoint: String,
    pub env_vars: Json<serde_json::Value>,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: String,
}

impl HasTimestamps for DistributedTrainingConfig {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for DistributedTrainingConfig {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外视图。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DistributedTrainingConfigResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub description: String,
    pub framework: String,
    pub worker_replicas: i32,
    pub gpu_per_worker: i32,
    pub cpu_per_worker: i32,
    pub memory_per_worker_gb: i32,
    pub entrypoint: String,
    pub env_vars: serde_json::Value,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: String,
}

impl From<DistributedTrainingConfig> for DistributedTrainingConfigResponse {
    fn from(c: DistributedTrainingConfig) -> Self {
        Self {
            id: c.id,
            created_at: c.created_at,
            updated_at: c.updated_at,
            name: c.name,
            description: c.description,
            framework: c.framework,
            worker_replicas: c.worker_replicas,
            gpu_per_worker: c.gpu_per_worker,
            cpu_per_worker: c.cpu_per_worker,
            memory_per_worker_gb: c.memory_per_worker_gb,
            entrypoint: c.entrypoint,
            env_vars: c.env_vars.0,
            tenant_id: c.tenant_id,
            created_by: c.created_by,
            status: c.status,
        }
    }
}

/// 创建入参。
pub struct NewDistributedTrainingConfig<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub framework: &'a str,
    pub worker_replicas: i32,
    pub gpu_per_worker: i32,
    pub cpu_per_worker: i32,
    pub memory_per_worker_gb: i32,
    pub entrypoint: &'a str,
    pub env_vars: serde_json::Value,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: &'a str,
}

/// INSERT。
pub async fn create(
    pool: &SqlitePool,
    input: NewDistributedTrainingConfig<'_>,
) -> AppResult<DistributedTrainingConfig> {
    let mut cfg = DistributedTrainingConfig {
        id: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        name: input.name.to_string(),
        description: input.description.to_string(),
        framework: input.framework.to_string(),
        worker_replicas: input.worker_replicas,
        gpu_per_worker: input.gpu_per_worker,
        cpu_per_worker: input.cpu_per_worker,
        memory_per_worker_gb: input.memory_per_worker_gb,
        entrypoint: input.entrypoint.to_string(),
        env_vars: Json(input.env_vars),
        tenant_id: input.tenant_id,
        created_by: input.created_by,
        status: input.status.to_string(),
    };
    cfg.before_insert();

    let res = sqlx::query(
        "INSERT INTO distributed_training_configs (created_at, updated_at, name, description, \
         framework, worker_replicas, gpu_per_worker, cpu_per_worker, memory_per_worker_gb, \
         entrypoint, env_vars, tenant_id, created_by, status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
    )
    .bind(cfg.created_at)
    .bind(cfg.updated_at)
    .bind(&cfg.name)
    .bind(&cfg.description)
    .bind(&cfg.framework)
    .bind(cfg.worker_replicas)
    .bind(cfg.gpu_per_worker)
    .bind(cfg.cpu_per_worker)
    .bind(cfg.memory_per_worker_gb)
    .bind(&cfg.entrypoint)
    .bind(&cfg.env_vars)
    .bind(cfg.tenant_id)
    .bind(cfg.created_by)
    .bind(&cfg.status)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted training config not found"))
}

/// 按 id 查询。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<DistributedTrainingConfig>> {
    let sql = if include_deleted {
        "SELECT * FROM distributed_training_configs WHERE id = ?1"
    } else {
        "SELECT * FROM distributed_training_configs WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 分页列表（可按 tenant_id 过滤）。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    tenant_id: Option<i64>,
) -> AppResult<PaginatedResult<DistributedTrainingConfig>> {
    let params = params.normalize();
    let sql_where = match tenant_id {
        Some(_) => "deleted_at IS NULL AND tenant_id = ?".to_string(),
        None => "deleted_at IS NULL".to_string(),
    };

    let count_sql = format!("SELECT COUNT(*) FROM distributed_training_configs WHERE {sql_where}");
    let list_sql = format!(
        "SELECT * FROM distributed_training_configs WHERE {sql_where} ORDER BY id ASC LIMIT ? OFFSET ?"
    );

    let mut query_count = sqlx::query_scalar::<_, i64>(&count_sql);
    let mut query_list = sqlx::query_as::<_, DistributedTrainingConfig>(&list_sql);

    if let Some(tid) = tenant_id {
        query_count = query_count.bind(tid);
        query_list = query_list.bind(tid);
    }
    query_list = query_list.bind(params.limit()).bind(params.offset());

    let total: i64 = query_count.fetch_one(pool).await?;
    let rows: Vec<DistributedTrainingConfig> = query_list.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("distributed_training_configs"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
