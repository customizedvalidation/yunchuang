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
