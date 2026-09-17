//! Dataset 服务（对齐 Go `services/acceleration_extensions.go` 中的 Dataset CRUD）。
//!
//! CRUD + 分页 + 按 type/tenant_id 过滤 + 软删除。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::dataset::{self, Dataset, DatasetResponse};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建 Dataset 入参。
#[derive(Debug, Clone)]
pub struct CreateDatasetInput {
    pub name: String,
    pub description: String,
    pub dataset_type: String,
    pub source_path: String,
    pub format: String,
    pub size_bytes: i64,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: Option<String>,
    pub labels: serde_json::Value,
}

/// 更新 Dataset 入参：全部可选。
#[derive(Debug, Clone, Default)]
pub struct UpdateDatasetInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub dataset_type: Option<String>,
    pub source_path: Option<String>,
    pub format: Option<String>,
    pub size_bytes: Option<i64>,
    pub status: Option<String>,
    pub labels: Option<serde_json::Value>,
}

/// 创建 dataset。
pub async fn create_dataset(
    pool: &SqlitePool,
    input: CreateDatasetInput,
) -> AppResult<DatasetResponse> {
    if dataset::name_taken(pool, &input.name, 0).await? {
        return Err(AppError::conflict("dataset name already exists"));
    }
    let status = input.status.unwrap_or_else(|| "active".to_string());
    let ds = dataset::create(
        pool,
        dataset::NewDataset {
            name: &input.name,
            description: &input.description,
            dataset_type: &input.dataset_type,
            source_path: &input.source_path,
            format: &input.format,
            size_bytes: input.size_bytes,
            tenant_id: input.tenant_id,
            created_by: input.created_by,
            status: &status,
            labels: input.labels,
        },
    )
    .await?;
    Ok(ds.into())
}

/// Dataset 详情。
pub async fn get_dataset(pool: &SqlitePool, id: i64) -> AppResult<DatasetResponse> {
    let ds = dataset::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("dataset not found"))?;
    Ok(ds.into())
}

/// 分页列表（按 tenant_id / type 过滤）。
pub async fn list_datasets(
    pool: &SqlitePool,
    params: PaginationParams,
    tenant_id: Option<i64>,
    dataset_type: Option<&str>,
) -> AppResult<PaginatedResult<DatasetResponse>> {
    let rows: PaginatedResult<Dataset> =
        dataset::list(pool, params, tenant_id, dataset_type).await?;
    Ok(PaginatedResult {
        data: rows.data.into_iter().map(DatasetResponse::from).collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 更新 dataset。
pub async fn update_dataset(
    pool: &SqlitePool,
    id: i64,
    input: UpdateDatasetInput,
) -> AppResult<DatasetResponse> {
    dataset::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("dataset not found"))?;

    if let Some(name) = input.name.as_deref() {
        if dataset::name_taken(pool, name, id).await? {
            return Err(AppError::conflict("dataset name already exists"));
        }
    }

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE datasets SET \
            name = COALESCE(?1, name), \
            description = COALESCE(?2, description), \
            type = COALESCE(?3, type), \
            source_path = COALESCE(?4, source_path), \
            format = COALESCE(?5, format), \
            size_bytes = COALESCE(?6, size_bytes), \
            status = COALESCE(?7, status), \
            labels = COALESCE(?8, labels), \
            updated_at = ?9 \
         WHERE id = ?10 AND deleted_at IS NULL",
    )
    .bind(input.name)
    .bind(input.description)
    .bind(input.dataset_type)
    .bind(input.source_path)
    .bind(input.format)
    .bind(input.size_bytes)
    .bind(input.status)
    .bind(
        input
            .labels
            .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "{}".into())),
    )
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let ds = dataset::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("dataset disappeared after update"))?;
    Ok(ds.into())
}

/// 软删除 dataset。
pub async fn delete_dataset(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = dataset::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("dataset not found"));
    }
    Ok(())
}
