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
use crate::services::quota::QuotaRequest;

/// 分页 + 过滤查询参数。
#[derive(Debug, Deserialize)]
pub struct QuotaListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub tenant_id: Option<i64>,
    pub partition_id: Option<i64>,
    pub status: Option<String>,
}

/// `POST /api/v1/quotas` 请求体。
#[derive(Debug, Deserialize, Validate)]
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
#[derive(Debug, Deserialize, Validate, Default)]
pub struct UpdateQuotaRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub gpu_limit: Option<i64>,
    pub cpu_limit: Option<f64>,
    pub memory_limit_gb: Option<f64>,
    pub storage_limit_gb: Option<f64>,
    pub status: Option<String>,
}

/// `POST /api/v1/quotas/:id/check` 请求体（资源请求量）。
#[derive(Debug, Deserialize, Default)]
pub struct CheckQuotaRequest {
    #[serde(default)]
    pub gpu: Option<i64>,
    #[serde(default)]
    pub cpu: Option<f64>,
    #[serde(default)]
    pub memory_gb: Option<f64>,
    #[serde(default)]
    pub storage_gb: Option<f64>,
}

/// 分页配额列表响应内层。
#[derive(Debug, Serialize)]
pub struct QuotaPage {
    pub data: Vec<ResourceQuotaResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

fn req_from_body(b: &CheckQuotaRequest) -> QuotaRequest {
    QuotaRequest {
        gpu: b.gpu.unwrap_or(0),
        cpu: b.cpu.unwrap_or(0.0),
        memory_gb: b.memory_gb.unwrap_or(0.0),
        storage_gb: b.storage_gb.unwrap_or(0.0),
    }
}

/// `GET /api/v1/quotas` — 分页列表（过滤）。
pub async fn list_quotas(
    State(state): State<AppState>,
    Query(q): Query<QuotaListQuery>,
) -> AppResult<Json<ApiResponse<QuotaPage>>> {
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
    Ok(Json(ApiResponse::success(QuotaPage {
        data: res.data,
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })))
}

/// `GET /api/v1/quotas/:id` — 详情。
pub async fn get_quota(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<ResourceQuotaResponse>>> {
    let q = quota_service::get_quota(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(q)))
}

/// `POST /api/v1/quotas` — 创建（201）。
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
pub async fn delete_quota(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    quota_service::delete_quota(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/quotas/:id/check` — 检查是否超限。
pub async fn check_quota(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<CheckQuotaRequest>,
) -> AppResult<Json<ApiResponse<quota_service::QuotaCheckResult>>> {
    let res = quota_service::check_quota(&state.pool, id, req_from_body(&body)).await?;
    Ok(Json(ApiResponse::success(res)))
}
