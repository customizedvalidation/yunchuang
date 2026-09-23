//! HTTP handlers for `/api/v1/quotas`（对齐 WP-P2-B4 调度域规格）。
//!
//! 仅做参数提取 / 校验 / 响应封装；CRUD 与 check/allocate/release 在
//! [`crate::services::quota`]。路由上：读需 JWT + `quota:read`，写需 JWT + `quota:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::models::resource_quota::ResourceQuotaResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::quota as quota_service;

/// 分页 + 过滤查询参数。
#[derive(utoipa::ToSchema, Debug, Deserialize)]
pub struct QuotaListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub tenant_id: Option<i64>,
    pub partition_id: Option<i64>,
    pub status: Option<String>,
}

/// `POST /api/v1/quotas` 请求体。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
pub struct CreateQuotaRequest {
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub tenant_id: i64,
    #[serde(default)]
    pub partition_id: Option<i64>,
    #[serde(default)]
    pub gpu_limit: Option<i64>,
    #[serde(default)]
    pub cpu_limit: Option<f64>,
    #[serde(default)]
    pub memory_limit_gb: Option<f64>,
    #[serde(default)]
    pub storage_limit_gb: Option<f64>,
    #[serde(default)]
    pub status: Option<String>,
}

/// `PUT /api/v1/quotas/:id` 请求体（全部可选）。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate, Default)]
pub struct UpdateQuotaRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub gpu_limit: Option<i64>,
    pub cpu_limit: Option<f64>,
    pub memory_limit_gb: Option<f64>,
    pub storage_limit_gb: Option<f64>,
    pub status: Option<String>,
}

/// `GET /api/v1/quotas/usage` 查询参数：按维度返回实时用量。
#[derive(utoipa::ToSchema, Debug, Deserialize)]
pub struct QuotaUsageQuery {
    pub scope_type: Option<String>,
    pub scope_id: Option<i64>,
}

/// `GET /api/v1/quotas/usage` — 按维度聚合 GPU 实时用量。
///
/// 数据口径（全部来自真实 DB，无写死 0）：
/// - `scope_type=cluster`：按 `gpu_devices.cluster_id = scope_id` 统计设备总数/已分配/可用。
/// - `scope_type=tenant`：经 `gpu_allocations.tenant_id = scope_id` 反查当前活跃占用的设备，
///   并从 `resource_quotas.tenant_id` 汇总 GPU 上限。
/// - `scope_type=user`：经 `gpu_allocations.user_id = scope_id` 反查该用户活跃占用的设备。
/// - 缺省/未知 scope：全集群聚合。
///
/// 前端 `MultiTenantManagement.vue` 消费 `gpu_used` / `gpu_limit`，故同时保留这两个别名。
#[utoipa::path(get,path="/api/v1/quotas/usage",tag="quotas",responses((status=200,description="quota usage",body=serde_json::Value),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_quota_usage(
    State(state): State<AppState>,
    Query(q): Query<QuotaUsageQuery>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let scope_type = q.scope_type.unwrap_or_default();
    let scope_id = q.scope_id.unwrap_or(0);
    let pool = &state.pool;

    // 按 scope 构造设备过滤子句；每个非缺省分支恰好含一个 `?` 占位符。
    let (filter_sql, bind_scope) = match scope_type.as_str() {
        "cluster" => ("gpu_devices.cluster_id = ?", true),
        "tenant" => (
            "gpu_devices.id IN (SELECT device_id FROM gpu_allocations \
             WHERE tenant_id = ? AND status = 'active' AND deleted_at IS NULL)",
            true,
        ),
        "user" => (
            "gpu_devices.id IN (SELECT device_id FROM gpu_allocations \
             WHERE user_id = ? AND status = 'active' AND deleted_at IS NULL)",
            true,
        ),
        _ => ("1=1", false),
    };

    let total_sql =
        format!("SELECT COUNT(*) FROM gpu_devices WHERE deleted_at IS NULL AND {filter_sql}");
    let allocated_sql = format!(
        "SELECT COUNT(*) FROM gpu_devices WHERE deleted_at IS NULL AND {filter_sql} AND status = 'allocated'"
    );
    let available_sql = format!(
        "SELECT COUNT(*) FROM gpu_devices WHERE deleted_at IS NULL AND {filter_sql} AND status = 'available'"
    );
    let by_status_sql = format!(
        "SELECT status, COUNT(*) FROM gpu_devices WHERE deleted_at IS NULL AND {filter_sql} GROUP BY status"
    );
    let by_vendor_sql = format!(
        "SELECT vendor, COUNT(*), \
         COALESCE(SUM(CASE WHEN status = 'allocated' THEN 1 ELSE 0 END), 0) \
         FROM gpu_devices WHERE deleted_at IS NULL AND {filter_sql} GROUP BY vendor"
    );

    let total: i64 = {
        let mut q = sqlx::query_scalar::<_, i64>(&total_sql);
        if bind_scope {
            q = q.bind(scope_id);
        }
        q.fetch_one(pool).await?
    };
    let gpu_allocated: i64 = {
        let mut q = sqlx::query_scalar::<_, i64>(&allocated_sql);
        if bind_scope {
            q = q.bind(scope_id);
        }
        q.fetch_one(pool).await?
    };
    let gpu_available: i64 = {
        let mut q = sqlx::query_scalar::<_, i64>(&available_sql);
        if bind_scope {
            q = q.bind(scope_id);
        }
        q.fetch_one(pool).await?
    };

    let by_status_rows: Vec<(String, i64)> = {
        let mut q = sqlx::query_as::<_, (String, i64)>(&by_status_sql);
        if bind_scope {
            q = q.bind(scope_id);
        }
        q.fetch_all(pool).await?
    };
    let by_vendor_rows: Vec<(String, i64, i64)> = {
        let mut q = sqlx::query_as::<_, (String, i64, i64)>(&by_vendor_sql);
        if bind_scope {
            q = q.bind(scope_id);
        }
        q.fetch_all(pool).await?
    };

    // GPU 上限：仅 tenant 维度有配额表记录；cluster/user 维度无配额行，返回 0（不限制）。
    let gpu_limit: i64 = if scope_type == "tenant" {
        sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(SUM(gpu_limit), 0) FROM resource_quotas \
             WHERE tenant_id = ? AND deleted_at IS NULL",
        )
        .bind(scope_id)
        .fetch_one(pool)
        .await?
    } else {
        0
    };

    let gpu_used_percent = if gpu_limit > 0 {
        (gpu_allocated as f64) * 100.0 / (gpu_limit as f64)
    } else {
        0.0
    };

    let by_status: Vec<serde_json::Value> = by_status_rows
        .into_iter()
        .map(|(status, count)| serde_json::json!({ "status": status, "count": count }))
        .collect();
    let by_vendor: Vec<serde_json::Value> = by_vendor_rows
        .into_iter()
        .map(|(vendor, count, allocated)| {
            serde_json::json!({ "vendor": vendor, "total": count, "allocated": allocated })
        })
        .collect();

    Ok(Json(ApiResponse::success(serde_json::json!({
        "scope_type": scope_type,
        "scope_id": scope_id,
        "gpu_total": total,
        "gpu_allocated": gpu_allocated,
        "gpu_available": gpu_available,
        "gpu_used": gpu_allocated,
        "gpu_limit": gpu_limit,
        "gpu_used_percent": (gpu_used_percent * 10.0).round() / 10.0,
        "by_vendor": by_vendor,
        "by_status": by_status,
    }))))
}

/// `POST /api/v1/quotas/check` 请求体（对齐 Go `CheckQuotaRequest`）。
#[derive(utoipa::ToSchema, Debug, Deserialize, Default)]
pub struct CheckQuotaRequest {
    pub scope_type: String,
    pub scope_id: i64,
    pub resource_type: String,
    #[serde(default)]
    pub requested: i64,
    #[serde(default)]
    pub gpu_fraction: f64,
}

/// 配额校验响应（对齐 Go `gin.H{"allowed": allowed}`）。
#[derive(utoipa::ToSchema, Debug, Serialize)]
pub struct CheckQuotaResponse {
    pub allowed: bool,
}

/// `GET /api/v1/quotas` — 分页列表（过滤）。
#[utoipa::path(get,path="/api/v1/quotas",tag="quotas",responses((status=200,description="quotas",body=Vec<crate::models::resource_quota::ResourceQuotaResponse>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_quotas(
    State(state): State<AppState>,
    Query(q): Query<QuotaListQuery>,
) -> AppResult<Json<ApiResponse<Vec<ResourceQuotaResponse>>>> {
    let params =
        PaginationParams::new(q.page.unwrap_or(1) as i64, q.page_size.unwrap_or(10) as i64);
    let res = quota_service::list_quotas(
        &state.pool,
        params,
        q.tenant_id,
        q.partition_id,
        q.status.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::success(res.data)))
}

/// `GET /api/v1/quotas/:id` — 详情。
#[utoipa::path(get,path="/api/v1/quotas/{id}",tag="quotas",responses((status=200,description="quota",body=crate::models::resource_quota::ResourceQuotaResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_quota(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<ResourceQuotaResponse>>> {
    let q = quota_service::get_quota(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(q)))
}

/// `POST /api/v1/quotas` — 创建（201）。
#[utoipa::path(post,path="/api/v1/quotas",request_body=CreateQuotaRequest,tag="quotas",responses((status=201,description="created",body=crate::models::resource_quota::ResourceQuotaResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn create_quota(
    State(state): State<AppState>,
    Json(body): Json<CreateQuotaRequest>,
) -> AppResult<WithStatus<ResourceQuotaResponse>> {
    body.validate()?;
    let input = quota_service::CreateQuotaInput {
        name: body.name,
        description: body.description.unwrap_or_default(),
        tenant_id: body.tenant_id,
        partition_id: body.partition_id,
        gpu_limit: body.gpu_limit.unwrap_or(0),
        cpu_limit: body.cpu_limit.unwrap_or(0.0),
        memory_limit_gb: body.memory_limit_gb.unwrap_or(0.0),
        storage_limit_gb: body.storage_limit_gb.unwrap_or(0.0),
        status: body.status,
    };
    let q = quota_service::create_quota(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(q),
    })
}

/// `PUT /api/v1/quotas/:id` — 更新。
#[utoipa::path(put,path="/api/v1/quotas/{id}",request_body=UpdateQuotaRequest,tag="quotas",responses((status=200,description="updated",body=crate::models::resource_quota::ResourceQuotaResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn update_quota(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateQuotaRequest>,
) -> AppResult<Json<ApiResponse<ResourceQuotaResponse>>> {
    body.validate()?;
    let input = quota_service::UpdateQuotaInput {
        name: body.name,
        description: body.description,
        gpu_limit: body.gpu_limit,
        cpu_limit: body.cpu_limit,
        memory_limit_gb: body.memory_limit_gb,
        storage_limit_gb: body.storage_limit_gb,
        status: body.status,
    };
    let q = quota_service::update_quota(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(q)))
}

/// `DELETE /api/v1/quotas/:id` — 软删除，204。
#[utoipa::path(delete,path="/api/v1/quotas/{id}",tag="quotas",responses((status=204,description="deleted"),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn delete_quota(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    quota_service::delete_quota(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/quotas/check` — 按 scope 校验是否超限（对齐 Go）。
#[utoipa::path(post,path="/api/v1/quotas/check",request_body=CheckQuotaRequest,tag="quotas",responses((status=200,description="check result",body=CheckQuotaResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn check_quota(
    State(state): State<AppState>,
    Json(body): Json<CheckQuotaRequest>,
) -> AppResult<Json<ApiResponse<CheckQuotaResponse>>> {
    let allowed = quota_service::check_quota_by_scope(
        &state.pool,
        &body.scope_type,
        body.scope_id,
        &body.resource_type,
        body.requested,
        body.gpu_fraction,
    )
    .await?;
    Ok(Json(ApiResponse::success(CheckQuotaResponse { allowed })))
}
