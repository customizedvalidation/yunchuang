//! HTTP handlers for `/api/v1/gpus`（对齐 Go `controllers/gpu_controller.go`）。
//!
//! 仅做参数提取 / 校验 / 响应封装；CRUD / allocate·release / 利用率汇总在
//! [`crate::services::gpu`]。路由上：读需 JWT + `gpu:read`，写需 JWT + `gpu:write`。
//!
//! 路径对齐 Go `api/routes.go`：
//!   GET/POST `/gpus`，GET/PUT/DELETE `/gpus/:id`，
//!   GET `/gpus/allocations`，POST `/gpus/allocations`，DELETE `/gpus/allocations/:id`，
//!   GET `/gpus/utilization`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::models::gpu_allocation::GpuAllocationResponse;
use crate::models::gpu_device::GpuDeviceResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::gpu::{self, AllocateGpuInput, CreateGpuDeviceInput, UpdateGpuDeviceInput};

/// 分页 + 过滤查询参数（设备列表）。
#[derive(Debug, Deserialize)]
pub struct GpuListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub cluster_id: Option<i64>,
    pub vendor: Option<String>,
    pub status: Option<String>,
}

/// 分页 + 过滤查询参数（分配记录列表）。
#[derive(Debug, Deserialize)]
pub struct AllocationListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub job_id: Option<i64>,
    pub user_id: Option<i64>,
    pub status: Option<String>,
}

/// `POST /api/v1/gpus` 请求体。
#[derive(Debug, Deserialize, Validate)]
pub struct CreateGpuDeviceRequest {
    #[serde(default)]
    pub cluster_id: Option<i64>,
    #[serde(default)]
    pub node_name: Option<String>,
    #[validate(length(min = 1, message = "vendor is required"))]
    pub vendor: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub index: Option<i64>,
    #[serde(default)]
    pub total_memory_gb: Option<i64>,
    #[serde(default)]
    pub allocatable_memory_gb: Option<i64>,
    #[serde(default)]
    pub used_memory_gb: Option<i64>,
    #[serde(default)]
    pub mig_enabled: Option<bool>,
    #[serde(default)]
    pub mig_profiles: Option<String>,
    #[serde(default)]
    pub driver_version: Option<String>,
    #[serde(default)]
    pub cuda_version: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub utilization: Option<f64>,
    #[serde(default)]
    pub temperature: Option<i64>,
    #[serde(default)]
    pub power_draw: Option<i64>,
    #[serde(default)]
    pub details: Option<String>,
}

/// `PUT /api/v1/gpus/:id` 请求体（全部可选）。
#[derive(Debug, Deserialize, Validate, Default)]
pub struct UpdateGpuDeviceRequest {
    pub node_name: Option<String>,
    pub vendor: Option<String>,
    pub model: Option<String>,
    pub total_memory_gb: Option<i64>,
    pub allocatable_memory_gb: Option<i64>,
    pub used_memory_gb: Option<i64>,
    pub mig_enabled: Option<bool>,
    pub mig_profiles: Option<String>,
    pub driver_version: Option<String>,
    pub cuda_version: Option<String>,
    pub status: Option<String>,
    pub utilization: Option<f64>,
    pub temperature: Option<i64>,
    pub power_draw: Option<i64>,
    pub details: Option<String>,
}

/// `POST /api/v1/gpus/allocations` 请求体（对应 Go `AllocateGPURequest`）。
#[derive(Debug, Deserialize, Validate)]
pub struct AllocateGpuRequest {
    #[serde(default)]
    pub job_id: Option<i64>,
    #[serde(default)]
    pub tenant_id: Option<i64>,
    #[serde(default)]
    pub user_id: Option<i64>,
    #[serde(default)]
    pub fraction: Option<f64>,
    #[serde(default)]
    pub memory_gb: Option<i64>,
    #[serde(default)]
    pub vendor: Option<String>,
}

/// 分页设备列表响应内层。
#[derive(Debug, Serialize)]
pub struct GpuDevicePage {
    pub data: Vec<GpuDeviceResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

/// 分页分配记录响应内层。
#[derive(Debug, Serialize)]
pub struct GpuAllocationPage {
    pub data: Vec<GpuAllocationResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

/// `GET /api/v1/gpus` — 分页设备列表（cluster_id/vendor/status 过滤）。
pub async fn list_gpu_devices(
    State(state): State<AppState>,
    Query(q): Query<GpuListQuery>,
) -> AppResult<Json<ApiResponse<GpuDevicePage>>> {
    let params =
        PaginationParams::new(q.page.unwrap_or(1) as i64, q.page_size.unwrap_or(10) as i64);
    let res = gpu::list_gpu_devices(
        &state.pool,
        params,
        q.cluster_id,
        q.vendor.as_deref(),
        q.status.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::success(GpuDevicePage {
        data: res.data,
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })))
}

/// `GET /api/v1/gpus/:id` — 设备详情。
pub async fn get_gpu_device(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<GpuDeviceResponse>>> {
    let d = gpu::get_gpu_device(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(d)))
}

/// `POST /api/v1/gpus` — 创建设备（201）。
pub async fn create_gpu_device(
    State(state): State<AppState>,
    Json(body): Json<CreateGpuDeviceRequest>,
) -> AppResult<WithStatus<GpuDeviceResponse>> {
    body.validate()?;
    let input = CreateGpuDeviceInput {
        cluster_id: body.cluster_id.unwrap_or(0),
        node_name: body.node_name.unwrap_or_default(),
        vendor: body.vendor,
        model: body.model.unwrap_or_default(),
        index: body.index.unwrap_or(0),
        total_memory_gb: body.total_memory_gb.unwrap_or(0),
        allocatable_memory_gb: body.allocatable_memory_gb.unwrap_or(0),
        used_memory_gb: body.used_memory_gb.unwrap_or(0),
        mig_enabled: body.mig_enabled.unwrap_or(false),
        mig_profiles: body.mig_profiles.unwrap_or_else(|| "[]".to_string()),
        driver_version: body.driver_version.unwrap_or_default(),
        cuda_version: body.cuda_version.unwrap_or_default(),
        status: body.status,
        utilization: body.utilization.unwrap_or(0.0),
        temperature: body.temperature.unwrap_or(0),
        power_draw: body.power_draw.unwrap_or(0),
        details: body.details.unwrap_or_default(),
    };
    let d = gpu::create_gpu_device(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(d),
    })
}

/// `PUT /api/v1/gpus/:id` — 更新设备。
pub async fn update_gpu_device(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateGpuDeviceRequest>,
) -> AppResult<Json<ApiResponse<GpuDeviceResponse>>> {
    body.validate()?;
    let input = UpdateGpuDeviceInput {
        node_name: body.node_name,
        vendor: body.vendor,
        model: body.model,
        total_memory_gb: body.total_memory_gb,
        allocatable_memory_gb: body.allocatable_memory_gb,
        used_memory_gb: body.used_memory_gb,
        mig_enabled: body.mig_enabled,
        mig_profiles: body.mig_profiles,
        driver_version: body.driver_version,
        cuda_version: body.cuda_version,
        status: body.status,
        utilization: body.utilization,
        temperature: body.temperature,
        power_draw: body.power_draw,
        details: body.details,
    };
    let d = gpu::update_gpu_device(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(d)))
}

/// `DELETE /api/v1/gpus/:id` — 软删除，204。
pub async fn delete_gpu_device(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    gpu::delete_gpu_device(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/gpus/allocations` — 分页分配记录列表（job_id/user_id/status 过滤）。
pub async fn list_allocations(
    State(state): State<AppState>,
    Query(q): Query<AllocationListQuery>,
) -> AppResult<Json<ApiResponse<GpuAllocationPage>>> {
    let params =
        PaginationParams::new(q.page.unwrap_or(1) as i64, q.page_size.unwrap_or(10) as i64);
    let res = gpu::list_allocations(
        &state.pool,
        params,
        q.job_id,
        q.user_id,
        q.status.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::success(GpuAllocationPage {
        data: res.data,
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })))
}

/// `POST /api/v1/gpus/allocations` — 分配 GPU（201）。
pub async fn allocate_gpu(
    State(state): State<AppState>,
    Json(body): Json<AllocateGpuRequest>,
) -> AppResult<WithStatus<GpuAllocationResponse>> {
    body.validate()?;
    let input = AllocateGpuInput {
        job_id: body.job_id.unwrap_or(0),
        tenant_id: body.tenant_id.unwrap_or(0),
        user_id: body.user_id.unwrap_or(0),
        fraction: body.fraction.unwrap_or(1.0),
        memory_gb: body.memory_gb.unwrap_or(0),
        vendor: body.vendor.unwrap_or_default(),
    };
    let a = gpu::allocate_gpu(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(a),
    })
}

/// `DELETE /api/v1/gpus/allocations/:id` — 释放分配，204。
pub async fn release_gpu(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    gpu::release_gpu(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/gpus/utilization` — GPU 利用率汇总。
pub async fn get_gpu_utilization(
    State(state): State<AppState>,
    Query(q): Query<GpuListQuery>,
) -> AppResult<Json<ApiResponse<std::collections::HashMap<String, serde_json::Value>>>> {
    let summary = gpu::get_gpu_utilization_summary(&state.pool, q.cluster_id).await?;
    Ok(Json(ApiResponse::success(summary)))
}
