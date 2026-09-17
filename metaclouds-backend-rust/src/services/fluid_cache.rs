//! FluidCache 服务。
//!
//! CRUD + 按 dataset_id 过滤 + 分页。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::fluid_cache::{self, FluidCache, FluidCacheResponse};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建入参。
#[derive(Debug, Clone)]
pub struct CreateFluidCacheInput {
    pub name: String,
    pub dataset_id: i64,
    pub namespace: String,
    pub path: String,
    pub cache_class: String,
    pub replicas: i32,
    pub status: Option<String>,
}

/// 更新入参。
#[derive(Debug, Clone, Default)]
pub struct UpdateFluidCacheInput {
    pub name: Option<String>,
    pub namespace: Option<String>,
    pub path: Option<String>,
    pub cache_class: Option<String>,
    pub replicas: Option<i32>,
    pub status: Option<String>,
}

/// 创建 fluid cache。
pub async fn create_fluid_cache(
    pool: &SqlitePool,
    input: CreateFluidCacheInput,
) -> AppResult<FluidCacheResponse> {
    let status = input.status.unwrap_or_else(|| "inactive".to_string());
    let cache = fluid_cache::create(
        pool,
        fluid_cache::NewFluidCache {
            name: &input.name,
            dataset_id: input.dataset_id,
            namespace: &input.namespace,
            path: &input.path,
            cache_class: &input.cache_class,
            replicas: input.replicas,
            status: &status,
        },
    )
    .await?;
    Ok(cache.into())
}

/// 详情。
pub async fn get_fluid_cache(pool: &SqlitePool, id: i64) -> AppResult<FluidCacheResponse> {
    let cache = fluid_cache::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("fluid cache not found"))?;
    Ok(cache.into())
}

/// 分页列表（按 dataset_id 过滤）。
pub async fn list_fluid_caches(
    pool: &SqlitePool,
    params: PaginationParams,
    dataset_id: Option<i64>,
) -> AppResult<PaginatedResult<FluidCacheResponse>> {
    let rows: PaginatedResult<FluidCache> =
        fluid_cache::list_by_dataset(pool, params, dataset_id).await?;
    Ok(PaginatedResult {
        data: rows
            .data
            .into_iter()
            .map(FluidCacheResponse::from)
            .collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 更新。
pub async fn update_fluid_cache(
    pool: &SqlitePool,
    id: i64,
    input: UpdateFluidCacheInput,
) -> AppResult<FluidCacheResponse> {
    fluid_cache::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("fluid cache not found"))?;

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE fluid_caches SET \
            name = COALESCE(?1, name), \
            namespace = COALESCE(?2, namespace), \
            path = COALESCE(?3, path), \
            cache_class = COALESCE(?4, cache_class), \
            replicas = COALESCE(?5, replicas), \
            status = COALESCE(?6, status), \
            updated_at = ?7 \
         WHERE id = ?8 AND deleted_at IS NULL",
    )
    .bind(input.name)
    .bind(input.namespace)
    .bind(input.path)
    .bind(input.cache_class)
    .bind(input.replicas)
    .bind(input.status)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let cache = fluid_cache::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("fluid cache disappeared after update"))?;
    Ok(cache.into())
}

/// 软删除。
pub async fn delete_fluid_cache(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = fluid_cache::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("fluid cache not found"));
    }
    Ok(())
}
