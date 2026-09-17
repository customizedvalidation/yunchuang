//! 配额服务（对齐 WP-P2-B4 调度域规格）。
//!
//! CRUD + 分页 + 按 tenant_id/partition_id/status 过滤 + 软删除 +
//! check_quota（验证资源使用是否超限，返回 allowed + 超限项）+
//! allocate/release（更新 used 字段）。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::resource_quota::{self, quota_status, NewQuota, ResourceQuotaResponse};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建配额入参。
#[derive(Debug, Clone)]
pub struct CreateQuotaInput {
    pub name: String,
    pub description: String,
    pub tenant_id: i64,
    pub partition_id: Option<i64>,
    pub gpu_limit: i64,
    pub cpu_limit: f64,
    pub memory_limit_gb: f64,
    pub storage_limit_gb: f64,
    pub status: Option<String>,
}

/// 更新配额入参：全部可选（limit 字段）。
#[derive(Debug, Clone, Default)]
pub struct UpdateQuotaInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub gpu_limit: Option<i64>,
    pub cpu_limit: Option<f64>,
    pub memory_limit_gb: Option<f64>,
    pub storage_limit_gb: Option<f64>,
    pub status: Option<String>,
}

/// check_quota 请求入参（资源请求量）。
#[derive(Debug, Clone, Copy)]
pub struct QuotaRequest {
    pub gpu: i64,
    pub cpu: f64,
    pub memory_gb: f64,
    pub storage_gb: f64,
}

/// check_quota 结果：是否允许 + 超限项列表。
#[derive(Debug, Clone, serde::Serialize)]
pub struct QuotaCheckResult {
    pub allowed: bool,
    pub exceeded: Vec<String>,
}

/// 创建配额（status 默认 active，同名同 scope 冲突 409）。
pub async fn create_quota(
    pool: &SqlitePool,
    input: CreateQuotaInput,
) -> AppResult<ResourceQuotaResponse> {
    // 同名租户内唯一。
    let dup: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM resource_quotas WHERE tenant_id = ?1 AND name = ?2 AND deleted_at IS NULL LIMIT 1",
    )
    .bind(input.tenant_id)
    .bind(&input.name)
    .fetch_optional(pool)
    .await?;
    if dup.is_some() {
        return Err(AppError::conflict(
            "quota name already exists for this tenant",
        ));
    }

    let status = input
        .status
        .unwrap_or_else(|| quota_status::ACTIVE.to_string());
    let q = resource_quota::create(
        pool,
        NewQuota {
            name: input.name,
            description: input.description,
            tenant_id: input.tenant_id,
            partition_id: input.partition_id,
            gpu_limit: input.gpu_limit,
            cpu_limit: input.cpu_limit,
            memory_limit_gb: input.memory_limit_gb,
            storage_limit_gb: input.storage_limit_gb,
            status,
        },
    )
    .await?;
    Ok(q.into())
}

/// 配额详情（404 若不存在或已软删除）。
pub async fn get_quota(pool: &SqlitePool, id: i64) -> AppResult<ResourceQuotaResponse> {
    let q = resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("resource quota not found"))?;
    Ok(q.into())
}

/// 分页配额列表（按 tenant_id/partition_id/status 过滤；排除软删除）。
pub async fn list_quotas(
    pool: &SqlitePool,
    params: PaginationParams,
    tenant_id: Option<i64>,
    partition_id: Option<i64>,
    status: Option<&str>,
) -> AppResult<PaginatedResult<ResourceQuotaResponse>> {
    let res = resource_quota::list(pool, params, tenant_id, partition_id, status).await?;
    Ok(PaginatedResult {
        data: res
            .data
            .into_iter()
            .map(ResourceQuotaResponse::from)
            .collect(),
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })
}

/// 更新配额：仅覆盖传入字段；自动刷 updated_at。
pub async fn update_quota(
    pool: &SqlitePool,
    id: i64,
    input: UpdateQuotaInput,
) -> AppResult<ResourceQuotaResponse> {
    resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("resource quota not found"))?;

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE resource_quotas SET \
            name = COALESCE(?1, name), \
            description = COALESCE(?2, description), \
            gpu_limit = COALESCE(?3, gpu_limit), \
            cpu_limit = COALESCE(?4, cpu_limit), \
            memory_limit_gb = COALESCE(?5, memory_limit_gb), \
            storage_limit_gb = COALESCE(?6, storage_limit_gb), \
            status = COALESCE(?7, status), \
            updated_at = ?8 \
         WHERE id = ?9 AND deleted_at IS NULL",
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(input.gpu_limit)
    .bind(input.cpu_limit)
    .bind(input.memory_limit_gb)
    .bind(input.storage_limit_gb)
    .bind(&input.status)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let q = resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("quota disappeared after update"))?;
    Ok(q.into())
}

/// 软删除配额（404 若不存在或已软删除）。
pub async fn delete_quota(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = resource_quota::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("resource quota not found"));
    }
    Ok(())
}

/// 校验资源请求是否在配额内（不修改 used）。
///
/// 对每类资源：若 limit > 0，则 used + requested > limit 记为超限。
/// 返回 allowed（全部未超限）+ 超限项名称列表。
pub async fn check_quota(
    pool: &SqlitePool,
    id: i64,
    request: QuotaRequest,
) -> AppResult<QuotaCheckResult> {
    let q = resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("resource quota not found"))?;

    let mut exceeded = Vec::new();
    if q.gpu_limit > 0 && q.gpu_used + request.gpu > q.gpu_limit {
        exceeded.push("gpu".to_string());
    }
    if q.cpu_limit > 0.0 && q.cpu_used + request.cpu > q.cpu_limit {
        exceeded.push("cpu".to_string());
    }
    if q.memory_limit_gb > 0.0 && q.memory_used_gb + request.memory_gb > q.memory_limit_gb {
        exceeded.push("memory".to_string());
    }
    if q.storage_limit_gb > 0.0 && q.storage_used_gb + request.storage_gb > q.storage_limit_gb {
        exceeded.push("storage".to_string());
    }

    Ok(QuotaCheckResult {
        allowed: exceeded.is_empty(),
        exceeded,
    })
}

/// 分配资源：先校验，未超限则累加 used；超限返回 403。
pub async fn allocate_resources(
    pool: &SqlitePool,
    id: i64,
    request: QuotaRequest,
) -> AppResult<ResourceQuotaResponse> {
    let check = check_quota(pool, id, request).await?;
    if !check.allowed {
        return Err(AppError::forbidden(format!(
            "quota exceeded for: {}",
            check.exceeded.join(", ")
        )));
    }

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE resource_quotas SET \
            gpu_used = gpu_used + ?1, \
            cpu_used = cpu_used + ?2, \
            memory_used_gb = memory_used_gb + ?3, \
            storage_used_gb = storage_used_gb + ?4, \
            updated_at = ?5 \
         WHERE id = ?6 AND deleted_at IS NULL",
    )
    .bind(request.gpu)
    .bind(request.cpu)
    .bind(request.memory_gb)
    .bind(request.storage_gb)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let q = resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("quota disappeared after allocate"))?;
    Ok(q.into())
}

/// 释放资源：扣减 used（不为负）。
pub async fn release_resources(
    pool: &SqlitePool,
    id: i64,
    request: QuotaRequest,
) -> AppResult<ResourceQuotaResponse> {
    resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("resource quota not found"))?;

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE resource_quotas SET \
            gpu_used = MAX(gpu_used - ?1, 0), \
            cpu_used = MAX(cpu_used - ?2, 0.0), \
            memory_used_gb = MAX(memory_used_gb - ?3, 0.0), \
            storage_used_gb = MAX(storage_used_gb - ?4, 0.0), \
            updated_at = ?5 \
         WHERE id = ?6 AND deleted_at IS NULL",
    )
    .bind(request.gpu)
    .bind(request.cpu)
    .bind(request.memory_gb)
    .bind(request.storage_gb)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let q = resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("quota disappeared after release"))?;
    Ok(q.into())
}
