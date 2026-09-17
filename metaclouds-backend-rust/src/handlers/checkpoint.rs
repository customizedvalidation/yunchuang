//! HTTP handlers for `/api/v1/checkpoints`（对齐 Go `controllers/checkpoint_controller.go`）。
//!
//! 路由上：列表/详情需 JWT + `checkpoint:read`，创建/更新/删除需 JWT + `checkpoint:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::auth::Claims;
use crate::error::AppResult;
use crate::models::checkpoint::CheckpointResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::checkpoint as checkpoint_service;

/// 分页 + 过滤查询参数。
#[derive(Debug, Deserialize)]
pub struct CheckpointListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub job_id: Option<i64>,
    pub dataset_id: Option<i64>,
}

/// `POST /api/v1/checkpoints` 请求体。
#[derive(Debug, Deserialize, Validate)]
pub struct CreateCheckpointRequest {
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub job_id: Option<i64>,
    #[serde(default)]
    pub dataset_id: Option<i64>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<i64>,
    #[serde(default)]
    pub step: Option<i64>,
    #[serde(default)]
    pub epoch: Option<i64>,
    #[serde(default)]
    pub metrics: Option<serde_json::Value>,
    #[serde(default)]
    pub tenant_id: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
}

/// `PUT /api/v1/checkpoints/:id` 请求体（全部可选）。
#[derive(Debug, Deserialize, Validate, Default)]
pub struct UpdateCheckpointRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub path: Option<String>,
    pub format: Option<String>,
    pub size_bytes: Option<i64>,
    pub step: Option<i64>,
    pub epoch: Option<i64>,
    pub metrics: Option<serde_json::Value>,
    pub status: Option<String>,
}

/// 分页列表响应内层。
#[derive(Debug, Serialize)]
pub struct CheckpointPage {
    pub data: Vec<CheckpointResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

/// `GET /api/v1/checkpoints` — 分页 + 过滤列表。
pub async fn list_checkpoints(
    State(state): State<AppState>,
    Query(q): Query<CheckpointListQuery>,
) -> AppResult<Json<ApiResponse<CheckpointPage>>> {
    let page = q.page.unwrap_or(1) as i64;
    let page_size = q.page_size.unwrap_or(10) as i64;
    let params = PaginationParams::new(page, page_size);

    let res =
        checkpoint_service::list_checkpoints(&state.pool, params, q.job_id, q.dataset_id).await?;
    Ok(Json(ApiResponse::success(CheckpointPage {
        data: res.data,
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })))
}

/// `GET /api/v1/checkpoints/:id` — 详情。
pub async fn get_checkpoint(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<CheckpointResponse>>> {
    let ckpt = checkpoint_service::get_checkpoint(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(ckpt)))
}

/// `POST /api/v1/checkpoints` — 创建（201）。
pub async fn create_checkpoint(
    State(state): State<AppState>,
    claims: Claims,
    Json(body): Json<CreateCheckpointRequest>,
) -> AppResult<WithStatus<CheckpointResponse>> {
    body.validate()?;
    let input = checkpoint_service::CreateCheckpointInput {
        name: body.name,
        description: body.description.unwrap_or_default(),
        job_id: body.job_id,
        dataset_id: body.dataset_id,
        path: body.path.unwrap_or_default(),
        format: body.format.unwrap_or_default(),
        size_bytes: body.size_bytes.unwrap_or(0),
        step: body.step.unwrap_or(0),
        epoch: body.epoch.unwrap_or(0),
        metrics: body.metrics.unwrap_or(serde_json::json!({})),
        tenant_id: body.tenant_id.unwrap_or(claims.tenant_id as i64),
        created_by: claims.user_id as i64,
        status: body.status,
    };
    let ckpt = checkpoint_service::create_checkpoint(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(ckpt),
    })
}

/// `PUT /api/v1/checkpoints/:id` — 更新。
pub async fn update_checkpoint(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateCheckpointRequest>,
) -> AppResult<Json<ApiResponse<CheckpointResponse>>> {
    body.validate()?;
    let input = checkpoint_service::UpdateCheckpointInput {
        name: body.name,
        description: body.description,
        path: body.path,
        format: body.format,
        size_bytes: body.size_bytes,
        step: body.step,
        epoch: body.epoch,
        metrics: body.metrics,
        status: body.status,
    };
    let ckpt = checkpoint_service::update_checkpoint(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(ckpt)))
}

/// `DELETE /api/v1/checkpoints/:id` — 软删除，204。
pub async fn delete_checkpoint(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    checkpoint_service::delete_checkpoint(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
