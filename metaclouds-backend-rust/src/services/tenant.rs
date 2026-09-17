//! 租户服务（对齐 Go `services/tenant_service.go`）。
//!
//! CRUD + 分页 + 软删除。唯一性、字段合并、404 判定都在本层，
//! handler 只负责参数提取与响应封装。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::tenant::{self, Tenant, TenantResponse};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建租户入参（对齐 Go `CreateTenantRequest`）。
#[derive(Debug, Clone)]
pub struct CreateTenantInput {
    pub name: String,
    pub description: String,
    pub gpu_quota: i64,
    pub cpu_quota: i64,
    pub memory_quota: i64,
    pub storage_quota: i64,
}

/// 更新租户入参：全部可选，仅更新传入字段（对齐 Go `UpdateTenantRequest`）。
#[derive(Debug, Clone, Default)]
pub struct UpdateTenantInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub gpu_quota: Option<i64>,
    pub cpu_quota: Option<i64>,
    pub memory_quota: Option<i64>,
    pub storage_quota: Option<i64>,
}

/// 按名称检查是否已存在未删除的租户。
async fn name_taken(pool: &SqlitePool, name: &str, except_id: i64) -> AppResult<bool> {
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM tenants WHERE name = ?1 AND deleted_at IS NULL AND id != ?2 LIMIT 1",
    )
    .bind(name)
    .bind(except_id)
    .fetch_optional(pool)
    .await?;
    Ok(exists.is_some())
}

/// 创建租户（status 固定为 active，对齐 Go `CreateTenant`）。
pub async fn create_tenant(
    pool: &SqlitePool,
    input: CreateTenantInput,
) -> AppResult<TenantResponse> {
    if name_taken(pool, &input.name, 0).await? {
        return Err(AppError::conflict("tenant name already exists"));
    }
    let tenant = tenant::create(
        pool,
        tenant::NewTenant {
            name: &input.name,
            description: &input.description,
            status: "active",
            gpu_quota: input.gpu_quota,
            cpu_quota: input.cpu_quota,
            memory_quota: input.memory_quota,
            storage_quota: input.storage_quota,
        },
    )
    .await?;
    Ok(tenant.into())
}

/// 租户详情（404 若不存在或已软删除）。
pub async fn get_tenant(pool: &SqlitePool, id: i64) -> AppResult<TenantResponse> {
    let tenant = tenant::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("tenant not found"))?;
    Ok(tenant.into())
}

/// 分页租户列表（过滤软删除）。
pub async fn list_tenants(
    pool: &SqlitePool,
    params: PaginationParams,
) -> AppResult<PaginatedResult<TenantResponse>> {
    let rows: PaginatedResult<Tenant> = tenant::list(pool, params, false).await?;
    Ok(PaginatedResult {
        data: rows.data.into_iter().map(TenantResponse::from).collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 更新租户：仅覆盖传入字段；重名校验；自动刷 updated_at。
pub async fn update_tenant(
    pool: &SqlitePool,
    id: i64,
    input: UpdateTenantInput,
) -> AppResult<TenantResponse> {
    // 404 早判：必须存在且未软删除。
    tenant::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("tenant not found"))?;

    if let Some(name) = input.name.as_deref() {
        if name_taken(pool, name, id).await? {
            return Err(AppError::conflict("tenant name already exists"));
        }
    }

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE tenants SET \
            name = COALESCE(?1, name), \
            description = COALESCE(?2, description), \
            status = COALESCE(?3, status), \
            gpu_quota = COALESCE(?4, gpu_quota), \
            cpu_quota = COALESCE(?5, cpu_quota), \
            memory_quota = COALESCE(?6, memory_quota), \
            storage_quota = COALESCE(?7, storage_quota), \
            updated_at = ?8 \
         WHERE id = ?9 AND deleted_at IS NULL",
    )
    .bind(input.name)
    .bind(input.description)
    .bind(input.status)
    .bind(input.gpu_quota)
    .bind(input.cpu_quota)
    .bind(input.memory_quota)
    .bind(input.storage_quota)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let tenant = tenant::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("tenant disappeared after update"))?;
    Ok(tenant.into())
}

/// 软删除租户（404 若不存在或已软删除）。
pub async fn delete_tenant(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = tenant::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("tenant not found"));
    }
    Ok(())
}
