//! HTTP handlers for `/api/v1/tenants`（对齐 Go `controllers/tenant_controller.go`）。
//!
//! 仅做参数提取 / 校验 / 响应封装；CRUD 与软删除在 [`crate::services::tenant`]。
//! 路由上：列表/详情需 JWT + `tenant:read`，创建/更新/删除需 JWT + `tenant:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::models::tenant::TenantResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::tenant as tenant_service;

/// 分页查询参数。
#[derive(utoipa::ToSchema, Debug, Deserialize)]
pub struct TenantListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

/// `POST /api/v1/tenants` 请求体（对齐 Go `CreateTenantRequest`）。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
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
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate, Default)]
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

/// `GET /api/v1/tenants` — 分页列表。
#[utoipa::path(get,path="/api/v1/tenants",tag="tenants",responses((status=200,description="tenants",body=Vec<crate::models::tenant::TenantResponse>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_tenants(
    State(state): State<AppState>,
    Query(q): Query<TenantListQuery>,
) -> AppResult<Json<ApiResponse<Vec<TenantResponse>>>> {
    let page = q.page.unwrap_or(1) as i64;
    let page_size = q.page_size.unwrap_or(10) as i64;
    let params = PaginationParams::new(page, page_size);

    let res = tenant_service::list_tenants(&state.pool, params).await?;
    Ok(Json(ApiResponse::success(res.data)))
}

/// `GET /api/v1/tenants/:id` — 详情。
#[utoipa::path(get,path="/api/v1/tenants/{id}",tag="tenants",responses((status=200,description="tenant",body=crate::models::tenant::TenantResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_tenant(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<TenantResponse>>> {
    let tenant = tenant_service::get_tenant(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(tenant)))
}

/// `POST /api/v1/tenants` — 创建（201）。
#[utoipa::path(post,path="/api/v1/tenants",request_body=CreateTenantRequest,tag="tenants",responses((status=201,description="created",body=crate::models::tenant::TenantResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
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
#[utoipa::path(put,path="/api/v1/tenants/{id}",request_body=UpdateTenantRequest,tag="tenants",responses((status=200,description="updated",body=crate::models::tenant::TenantResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
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
#[utoipa::path(delete,path="/api/v1/tenants/{id}",tag="tenants",responses((status=204,description="deleted"),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn delete_tenant(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    tenant_service::delete_tenant(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
