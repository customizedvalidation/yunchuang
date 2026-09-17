//! HTTP handlers for `/api/v1/tenants`（对齐 Go `controllers/tenant_controller.go`）。
//!
//! 仅做参数提取 / 校验 / 响应封装；CRUD 与软删除在 [`crate::services::tenant`]。
//! 路由上：列表/详情需 JWT + `tenant:read`，创建/更新/删除需 JWT + `tenant:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::models::tenant::TenantResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::tenant as tenant_service;

/// 分页查询参数。
#[derive(Debug, Deserialize)]
pub struct TenantListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// `POST /api/v1/tenants` 请求体（对齐 Go `CreateTenantRequest`）。
#[derive(Debug, Deserialize, Validate)]
pub struct CreateTenantRequest {
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    #[validate(range(min = 0, message = "gpu_quota must be >= 0"))]
    pub gpu_quota: Option<i64>,
    #[serde(default)]
    #[validate(range(min = 0, message = "cpu_quota must be >= 0"))]
    pub cpu_quota: Option<i64>,
    #[serde(default)]
    #[validate(range(min = 0, message = "memory_quota must be >= 0"))]
    pub memory_quota: Option<i64>,
    #[serde(default)]
    #[validate(range(min = 0, message = "storage_quota must be >= 0"))]
    pub storage_quota: Option<i64>,
}

/// `PUT /api/v1/tenants/:id` 请求体（全部可选）。
#[derive(Debug, Deserialize, Validate, Default)]
pub struct UpdateTenantRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    #[validate(range(min = 0, message = "gpu_quota must be >= 0"))]
    pub gpu_quota: Option<i64>,
    #[validate(range(min = 0, message = "cpu_quota must be >= 0"))]
    pub cpu_quota: Option<i64>,
    #[validate(range(min = 0, message = "memory_quota must be >= 0"))]
    pub memory_quota: Option<i64>,
    #[validate(range(min = 0, message = "storage_quota must be >= 0"))]
    pub storage_quota: Option<i64>,
}

/// 分页租户列表响应内层。
#[derive(Debug, Serialize)]
pub struct TenantPage {
    pub data: Vec<TenantResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

/// `GET /api/v1/tenants` — 分页列表。
pub async fn list_tenants(
    State(state): State<AppState>,
    Query(q): Query<TenantListQuery>,
) -> AppResult<Json<ApiResponse<TenantPage>>> {
    let page = q.page.unwrap_or(1) as i64;
    let page_size = q.page_size.unwrap_or(10) as i64;
    let params = PaginationParams::new(page, page_size);

    let res = tenant_service::list_tenants(&state.pool, params).await?;
    Ok(Json(ApiResponse::success(TenantPage {
        data: res.data,
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })))
}

/// `GET /api/v1/tenants/:id` — 详情。
pub async fn get_tenant(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<TenantResponse>>> {
    let tenant = tenant_service::get_tenant(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(tenant)))
}

/// `POST /api/v1/tenants` — 创建（201）。
pub async fn create_tenant(
    State(state): State<AppState>,
    Json(body): Json<CreateTenantRequest>,
) -> AppResult<WithStatus<TenantResponse>> {
    body.validate()?;
    let input = tenant_service::CreateTenantInput {
        name: body.name,
        description: body.description.unwrap_or_default(),
        gpu_quota: body.gpu_quota.unwrap_or(0),
        cpu_quota: body.cpu_quota.unwrap_or(0),
        memory_quota: body.memory_quota.unwrap_or(0),
        storage_quota: body.storage_quota.unwrap_or(0),
    };
    let tenant = tenant_service::create_tenant(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(tenant),
    })
}

/// `PUT /api/v1/tenants/:id` — 更新。
pub async fn update_tenant(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateTenantRequest>,
) -> AppResult<Json<ApiResponse<TenantResponse>>> {
    body.validate()?;
    let input = tenant_service::UpdateTenantInput {
        name: body.name,
        description: body.description,
        status: body.status,
        gpu_quota: body.gpu_quota,
        cpu_quota: body.cpu_quota,
        memory_quota: body.memory_quota,
        storage_quota: body.storage_quota,
    };
    let tenant = tenant_service::update_tenant(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(tenant)))
}

/// `DELETE /api/v1/tenants/:id` — 软删除，成功 204（对齐 Go `response.NoContent`）。
pub async fn delete_tenant(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    tenant_service::delete_tenant(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
