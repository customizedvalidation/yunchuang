//! DistributedTrainingConfig 服务。
//!
//! CRUD + 分页。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::distributed_training_config::{
    self, DistributedTrainingConfig, DistributedTrainingConfigResponse,
};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建入参。
#[derive(Debug, Clone)]
pub struct CreateTrainingConfigInput {
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
    pub status: Option<String>,
}

/// 更新入参。
#[derive(Debug, Clone, Default)]
pub struct UpdateTrainingConfigInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub framework: Option<String>,
    pub worker_replicas: Option<i32>,
    pub gpu_per_worker: Option<i32>,
    pub cpu_per_worker: Option<i32>,
    pub memory_per_worker_gb: Option<i32>,
    pub entrypoint: Option<String>,
    pub env_vars: Option<serde_json::Value>,
    pub status: Option<String>,
}

/// 创建。
pub async fn create_training_config(
    pool: &SqlitePool,
    input: CreateTrainingConfigInput,
) -> AppResult<DistributedTrainingConfigResponse> {
    let status = input.status.unwrap_or_else(|| "active".to_string());
    let cfg = distributed_training_config::create(
        pool,
        distributed_training_config::NewDistributedTrainingConfig {
            name: &input.name,
            description: &input.description,
            framework: &input.framework,
            worker_replicas: input.worker_replicas,
            gpu_per_worker: input.gpu_per_worker,
            cpu_per_worker: input.cpu_per_worker,
            memory_per_worker_gb: input.memory_per_worker_gb,
            entrypoint: &input.entrypoint,
            env_vars: input.env_vars,
            tenant_id: input.tenant_id,
            created_by: input.created_by,
            status: &status,
        },
    )
    .await?;
    Ok(cfg.into())
}

/// 详情。
pub async fn get_training_config(
    pool: &SqlitePool,
    id: i64,
) -> AppResult<DistributedTrainingConfigResponse> {
    let cfg = distributed_training_config::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("distributed training config not found"))?;
    Ok(cfg.into())
}

/// 分页列表。
pub async fn list_training_configs(
    pool: &SqlitePool,
    params: PaginationParams,
    tenant_id: Option<i64>,
) -> AppResult<PaginatedResult<DistributedTrainingConfigResponse>> {
    let rows: PaginatedResult<DistributedTrainingConfig> =
        distributed_training_config::list(pool, params, tenant_id).await?;
    Ok(PaginatedResult {
        data: rows
            .data
            .into_iter()
            .map(DistributedTrainingConfigResponse::from)
            .collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 更新。
pub async fn update_training_config(
    pool: &SqlitePool,
    id: i64,
    input: UpdateTrainingConfigInput,
) -> AppResult<DistributedTrainingConfigResponse> {
    distributed_training_config::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("distributed training config not found"))?;

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE distributed_training_configs SET \
            name = COALESCE(?1, name), \
            description = COALESCE(?2, description), \
            framework = COALESCE(?3, framework), \
            worker_replicas = COALESCE(?4, worker_replicas), \
            gpu_per_worker = COALESCE(?5, gpu_per_worker), \
            cpu_per_worker = COALESCE(?6, cpu_per_worker), \
            memory_per_worker_gb = COALESCE(?7, memory_per_worker_gb), \
            entrypoint = COALESCE(?8, entrypoint), \
            env_vars = COALESCE(?9, env_vars), \
            status = COALESCE(?10, status), \
            updated_at = ?11 \
         WHERE id = ?12 AND deleted_at IS NULL",
    )
    .bind(input.name)
    .bind(input.description)
    .bind(input.framework)
    .bind(input.worker_replicas)
    .bind(input.gpu_per_worker)
    .bind(input.cpu_per_worker)
    .bind(input.memory_per_worker_gb)
    .bind(input.entrypoint)
    .bind(
        input
            .env_vars
            .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "{}".into())),
    )
    .bind(input.status)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let cfg = distributed_training_config::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("training config disappeared after update"))?;
    Ok(cfg.into())
}

/// 软删除。
pub async fn delete_training_config(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = distributed_training_config::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("distributed training config not found"));
    }
    Ok(())
}
