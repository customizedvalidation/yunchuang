//! InferenceConfig 服务。
//!
//! CRUD + 分页。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::inference_config::{self, InferenceConfig, InferenceConfigResponse};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建入参。
#[derive(Debug, Clone)]
pub struct CreateInferenceConfigInput {
    pub name: String,
    pub description: String,
    pub model_path: String,
    pub runtime: String,
    pub replicas: i32,
    pub gpu_per_replica: i32,
    pub cpu_per_replica: i32,
    pub memory_per_replica_gb: i32,
    pub port: i32,
    pub health_check_path: String,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: Option<String>,
}

/// 更新入参。
#[derive(Debug, Clone, Default)]
pub struct UpdateInferenceConfigInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub model_path: Option<String>,
    pub runtime: Option<String>,
    pub replicas: Option<i32>,
    pub gpu_per_replica: Option<i32>,
    pub cpu_per_replica: Option<i32>,
    pub memory_per_replica_gb: Option<i32>,
    pub port: Option<i32>,
    pub health_check_path: Option<String>,
    pub status: Option<String>,
}

/// 创建。
pub async fn create_inference_config(
    pool: &SqlitePool,
    input: CreateInferenceConfigInput,
) -> AppResult<InferenceConfigResponse> {
    let status = input.status.unwrap_or_else(|| "active".to_string());
    let cfg = inference_config::create(
        pool,
        inference_config::NewInferenceConfig {
            name: &input.name,
            description: &input.description,
            model_path: &input.model_path,
            runtime: &input.runtime,
            replicas: input.replicas,
            gpu_per_replica: input.gpu_per_replica,
            cpu_per_replica: input.cpu_per_replica,
            memory_per_replica_gb: input.memory_per_replica_gb,
            port: input.port,
            health_check_path: &input.health_check_path,
            tenant_id: input.tenant_id,
            created_by: input.created_by,
            status: &status,
        },
    )
    .await?;
    Ok(cfg.into())
}

/// 详情。
pub async fn get_inference_config(
    pool: &SqlitePool,
    id: i64,
) -> AppResult<InferenceConfigResponse> {
    let cfg = inference_config::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("inference config not found"))?;
    Ok(cfg.into())
}

/// 分页列表。
pub async fn list_inference_configs(
    pool: &SqlitePool,
    params: PaginationParams,
    tenant_id: Option<i64>,
) -> AppResult<PaginatedResult<InferenceConfigResponse>> {
    let rows: PaginatedResult<InferenceConfig> =
        inference_config::list(pool, params, tenant_id).await?;
    Ok(PaginatedResult {
        data: rows
            .data
            .into_iter()
            .map(InferenceConfigResponse::from)
            .collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 更新。
pub async fn update_inference_config(
    pool: &SqlitePool,
    id: i64,
    input: UpdateInferenceConfigInput,
) -> AppResult<InferenceConfigResponse> {
    inference_config::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("inference config not found"))?;

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE inference_configs SET \
            name = COALESCE(?1, name), \
            description = COALESCE(?2, description), \
            model_path = COALESCE(?3, model_path), \
            runtime = COALESCE(?4, runtime), \
            replicas = COALESCE(?5, replicas), \
            gpu_per_replica = COALESCE(?6, gpu_per_replica), \
            cpu_per_replica = COALESCE(?7, cpu_per_replica), \
            memory_per_replica_gb = COALESCE(?8, memory_per_replica_gb), \
            port = COALESCE(?9, port), \
            health_check_path = COALESCE(?10, health_check_path), \
            status = COALESCE(?11, status), \
            updated_at = ?12 \
         WHERE id = ?13 AND deleted_at IS NULL",
    )
    .bind(input.name)
    .bind(input.description)
    .bind(input.model_path)
    .bind(input.runtime)
    .bind(input.replicas)
    .bind(input.gpu_per_replica)
    .bind(input.cpu_per_replica)
    .bind(input.memory_per_replica_gb)
    .bind(input.port)
    .bind(input.health_check_path)
    .bind(input.status)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let cfg = inference_config::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inference config disappeared after update"))?;
    Ok(cfg.into())
}

/// 软删除。
pub async fn delete_inference_config(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = inference_config::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("inference config not found"));
    }
    Ok(())
}
