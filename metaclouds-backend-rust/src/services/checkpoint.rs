//! Checkpoint 服务。
//!
//! CRUD + 按 job_id/dataset_id 过滤 + 分页。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::checkpoint::{self, Checkpoint, CheckpointResponse};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建入参。
#[derive(Debug, Clone)]
pub struct CreateCheckpointInput {
    pub name: String,
    pub description: String,
    pub job_id: Option<i64>,
    pub dataset_id: Option<i64>,
    pub path: String,
    pub format: String,
    pub size_bytes: i64,
    pub step: i64,
    pub epoch: i64,
    pub metrics: serde_json::Value,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: Option<String>,
}

/// 更新入参。
#[derive(Debug, Clone, Default)]
pub struct UpdateCheckpointInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub path: Option<String>,
    pub format: Option<String>,
    pub size_bytes: Option<i64>,
    pub step: Option<i64>,
    pub epoch: Option<i64>,
    pub metrics: Option<serde_json::Value>,
    pub status: Option<String>,
}

/// 创建。
pub async fn create_checkpoint(
    pool: &SqlitePool,
    input: CreateCheckpointInput,
) -> AppResult<CheckpointResponse> {
    let status = input.status.unwrap_or_else(|| "completed".to_string());
    let format = if input.format.is_empty() {
        "pytorch".to_string()
    } else {
        input.format
    };
    let ckpt = checkpoint::create(
        pool,
        checkpoint::NewCheckpoint {
            name: &input.name,
            description: &input.description,
            job_id: input.job_id,
            dataset_id: input.dataset_id,
            path: &input.path,
            format: &format,
            size_bytes: input.size_bytes,
            step: input.step,
            epoch: input.epoch,
            metrics: input.metrics,
            tenant_id: input.tenant_id,
            created_by: input.created_by,
            status: &status,
        },
    )
    .await?;
    Ok(ckpt.into())
}

/// 详情。
pub async fn get_checkpoint(pool: &SqlitePool, id: i64) -> AppResult<CheckpointResponse> {
    let ckpt = checkpoint::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("checkpoint not found"))?;
    Ok(ckpt.into())
}

/// 分页列表（按 job_id / dataset_id 过滤）。
pub async fn list_checkpoints(
    pool: &SqlitePool,
    params: PaginationParams,
    job_id: Option<i64>,
    dataset_id: Option<i64>,
) -> AppResult<PaginatedResult<CheckpointResponse>> {
    let rows: PaginatedResult<Checkpoint> =
        checkpoint::list(pool, params, job_id, dataset_id).await?;
    Ok(PaginatedResult {
        data: rows
            .data
            .into_iter()
            .map(CheckpointResponse::from)
            .collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 更新。
pub async fn update_checkpoint(
    pool: &SqlitePool,
    id: i64,
    input: UpdateCheckpointInput,
) -> AppResult<CheckpointResponse> {
    checkpoint::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("checkpoint not found"))?;

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE checkpoints SET \
            name = COALESCE(?1, name), \
            description = COALESCE(?2, description), \
            path = COALESCE(?3, path), \
            format = COALESCE(?4, format), \
            size_bytes = COALESCE(?5, size_bytes), \
            step = COALESCE(?6, step), \
            epoch = COALESCE(?7, epoch), \
            metrics = COALESCE(?8, metrics), \
            status = COALESCE(?9, status), \
            updated_at = ?10 \
         WHERE id = ?11 AND deleted_at IS NULL",
    )
    .bind(input.name)
    .bind(input.description)
    .bind(input.path)
    .bind(input.format)
    .bind(input.size_bytes)
    .bind(input.step)
    .bind(input.epoch)
    .bind(
        input
            .metrics
            .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "{}".into())),
    )
    .bind(input.status)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let ckpt = checkpoint::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("checkpoint disappeared after update"))?;
    Ok(ckpt.into())
}

/// 软删除。
pub async fn delete_checkpoint(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = checkpoint::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("checkpoint not found"));
    }
    Ok(())
}
