//! HTTP handlers for `/api/v1/resources`（对齐 Go `controllers/resource_controller.go`）。
//!
//! 仅做参数提取 / 校验 / 响应封装；CRUD 与软删除在 [`crate::services::resource`]。
//! 路由上：列表/详情需 JWT + `resource:read`，创建/更新/删除需 JWT + `resource:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::models::resource::ResourceResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::resource as resource_service;

/// 分页 + 搜索 + 过滤查询参数。
#[derive(utoipa::ToSchema, Debug, Deserialize)]
pub struct ResourceListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub search: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub cluster_id: Option<i64>,
}

/// `POST /api/v1/resources` 请求体。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
pub struct CreateResourceRequest {
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[validate(length(min = 1, message = "type is required"))]
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub cluster_id: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    #[validate(range(min = 0, message = "total must be >= 0"))]
    pub total: Option<i64>,
    #[serde(default)]
    pub used: Option<i64>,
    #[serde(default)]
    pub available: Option<i64>,
    #[serde(default)]
    pub utilization: Option<f64>,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub vendor: Option<String>,
    #[serde(default)]
    pub gpu_model: Option<String>,
    #[serde(default)]
    pub vram_total_mb: Option<i64>,
    #[serde(default)]
    pub vram_used_mb: Option<i64>,
    #[serde(default)]
    pub vram_oversubscription_ratio: Option<f64>,
    #[serde(default)]
    pub mig_enabled: Option<bool>,
}

/// `PUT /api/v1/resources/:id` 请求体（全部可选）。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate, Default)]
pub struct UpdateResourceRequest {
    pub status: Option<String>,
    #[validate(range(min = 0, message = "total must be >= 0"))]
    pub total: Option<i64>,
    #[validate(range(min = 0, message = "used must be >= 0"))]
    pub used: Option<i64>,
    #[validate(range(min = 0, message = "available must be >= 0"))]
    pub available: Option<i64>,
    #[validate(range(min = 0.0, message = "utilization must be >= 0"))]
    pub utilization: Option<f64>,
    pub details: Option<String>,
}

/// 分页资源列表响应内层。
#[derive(utoipa::ToSchema, Debug, Serialize)]
pub struct ResourcePage {
    pub data: Vec<ResourceResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

/// `GET /api/v1/resources` — 分页列表（搜索 + type/cluster_id 过滤）。
#[utoipa::path(get,path="/api/v1/resources",tag="resources",responses((status=200,description="paginated resources",body=ResourcePage),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_resources(
    State(state): State<AppState>,
    Query(q): Query<ResourceListQuery>,
) -> AppResult<Json<ApiResponse<ResourcePage>>> {
    let params =
        PaginationParams::new(q.page.unwrap_or(1) as i64, q.page_size.unwrap_or(10) as i64);
    let res = resource_service::list_resources(
        &state.pool,
        params,
        q.kind.as_deref(),
        q.cluster_id,
        q.search.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::success(ResourcePage {
        data: res.data,
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })))
}

/// `GET /api/v1/resources/:id` — 详情。
#[utoipa::path(get,path="/api/v1/resources/{id}",tag="resources",responses((status=200,description="resource",body=crate::models::resource::ResourceResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_resource(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<ResourceResponse>>> {
    let r = resource_service::get_resource(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(r)))
}

/// `POST /api/v1/resources` — 创建（201）。
#[utoipa::path(post,path="/api/v1/resources",request_body=CreateResourceRequest,tag="resources",responses((status=201,description="created",body=crate::models::resource::ResourceResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn create_resource(
    State(state): State<AppState>,
    Json(body): Json<CreateResourceRequest>,
) -> AppResult<WithStatus<ResourceResponse>> {
    body.validate()?;
    let input = resource_service::CreateResourceInput {
        cluster_id: body.cluster_id.unwrap_or(0),
        kind: body.kind,
        name: body.name,
        status: body.status,
        total: body.total.unwrap_or(0),
        used: body.used.unwrap_or(0),
        available: body.available.unwrap_or(0),
        utilization: body.utilization.unwrap_or(0.0),
        details: body.details.unwrap_or_default(),
        vendor: body.vendor.unwrap_or_default(),
        gpu_model: body.gpu_model.unwrap_or_default(),
        vram_total_mb: body.vram_total_mb.unwrap_or(0),
        vram_used_mb: body.vram_used_mb.unwrap_or(0),
        vram_oversubscription_ratio: body.vram_oversubscription_ratio.unwrap_or(1.0),
        mig_enabled: body.mig_enabled.unwrap_or(false),
    };
    let r = resource_service::create_resource(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(r),
    })
}

/// `PUT /api/v1/resources/:id` — 更新。
#[utoipa::path(put,path="/api/v1/resources/{id}",request_body=UpdateResourceRequest,tag="resources",responses((status=200,description="updated",body=crate::models::resource::ResourceResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn update_resource(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateResourceRequest>,
) -> AppResult<Json<ApiResponse<ResourceResponse>>> {
    body.validate()?;
    let input = resource_service::UpdateResourceInput {
        status: body.status,
        total: body.total,
        used: body.used,
        available: body.available,
        utilization: body.utilization,
        details: body.details,
    };
    let r = resource_service::update_resource(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(r)))
}

/// `DELETE /api/v1/resources/:id` — 软删除，204。
#[utoipa::path(delete,path="/api/v1/resources/{id}",tag="resources",responses((status=204,description="deleted"),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn delete_resource(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    resource_service::delete_resource(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
