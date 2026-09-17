//! 资源服务（对齐 Go `services/resource_service.go`）。
//!
//! CRUD + 分页 + 按 type/cluster_id 过滤 + name 搜索 + 软删除。
//! handler 只做参数提取与响应封装，所有 DB 操作在本层。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::resource::{self, NewResource, Resource, ResourceResponse};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建资源入参。
#[derive(Debug, Clone)]
pub struct CreateResourceInput {
    pub cluster_id: i64,
    pub kind: String,
    pub name: String,
    pub status: Option<String>,
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

/// 更新资源入参：全部可选，仅更新传入字段。
#[derive(Debug, Clone, Default)]
pub struct UpdateResourceInput {
    pub status: Option<String>,
    pub total: Option<i64>,
    pub used: Option<i64>,
    pub available: Option<i64>,
    pub utilization: Option<f64>,
    pub details: Option<String>,
}

/// 创建资源。
pub async fn create_resource(
    pool: &SqlitePool,
    input: CreateResourceInput,
) -> AppResult<ResourceResponse> {
    let resource = resource::create(
        pool,
        NewResource {
            cluster_id: input.cluster_id,
            kind: &input.kind,
            name: &input.name,
            status: input.status.as_deref().unwrap_or("available"),
            total: input.total,
            used: input.used,
            available: input.available,
            utilization: input.utilization,
            details: &input.details,
            vendor: &input.vendor,
            gpu_model: &input.gpu_model,
            vram_total_mb: input.vram_total_mb,
            vram_used_mb: input.vram_used_mb,
            vram_oversubscription_ratio: input.vram_oversubscription_ratio,
            mig_enabled: input.mig_enabled,
        },
    )
    .await?;
    Ok(resource.into())
}

/// 资源详情（404 若不存在或已软删除）。
pub async fn get_resource(pool: &SqlitePool, id: i64) -> AppResult<ResourceResponse> {
    let resource = resource::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("resource not found"))?;
    Ok(resource.into())
}

/// 分页资源列表（可按 type / cluster_id 过滤，按 name 搜索）。
pub async fn list_resources(
    pool: &SqlitePool,
    params: PaginationParams,
    kind: Option<&str>,
    cluster_id: Option<i64>,
    search: Option<&str>,
) -> AppResult<PaginatedResult<ResourceResponse>> {
    let rows: PaginatedResult<Resource> =
        resource::list(pool, params, kind, cluster_id, search).await?;
    Ok(PaginatedResult {
        data: rows.data.into_iter().map(ResourceResponse::from).collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 更新资源：仅覆盖传入字段，自动刷 updated_at。
pub async fn update_resource(
    pool: &SqlitePool,
    id: i64,
    input: UpdateResourceInput,
) -> AppResult<ResourceResponse> {
    resource::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("resource not found"))?;

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE resources SET \
            status = COALESCE(?1, status), \
            total = COALESCE(?2, total), \
            used = COALESCE(?3, used), \
            available = COALESCE(?4, available), \
            utilization = COALESCE(?5, utilization), \
            details = COALESCE(?6, details), \
            updated_at = ?7 \
         WHERE id = ?8 AND deleted_at IS NULL",
    )
    .bind(&input.status)
    .bind(input.total)
    .bind(input.used)
    .bind(input.available)
    .bind(input.utilization)
    .bind(&input.details)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let resource = resource::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("resource disappeared after update"))?;
    Ok(resource.into())
}

/// 软删除资源（404 若不存在或已软删除）。
pub async fn delete_resource(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = resource::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("resource not found"));
    }
    Ok(())
}
