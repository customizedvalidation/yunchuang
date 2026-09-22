//! HTTP handlers for `/api/v1/checkpoints`（对齐 Go `controllers/checkpoint_controller.go`）。
//!
//! 路由上：列表/详情需 JWT + `checkpoint:read`，创建/更新/删除需 JWT + `checkpoint:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::auth::Claims;
use crate::error::{AppError, AppResult};
use crate::models::checkpoint::CheckpointResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::checkpoint as checkpoint_service;

/// 分页 + 过滤查询参数。
#[derive(utoipa::ToSchema, Debug, Deserialize)]
pub struct CheckpointListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub job_id: Option<i64>,
    pub dataset_id: Option<i64>,
}

/// `POST /api/v1/checkpoints` 请求体。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
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
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate, Default)]
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

/// `GET /api/v1/checkpoints` — 分页 + 过滤列表。
#[utoipa::path(get,path="/api/v1/checkpoints",tag="checkpoints",responses((status=200,description="checkpoints",body=Vec<crate::models::checkpoint::CheckpointResponse>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_checkpoints(
    State(state): State<AppState>,
    Query(q): Query<CheckpointListQuery>,
) -> AppResult<Json<ApiResponse<Vec<CheckpointResponse>>>> {
    let page = q.page.unwrap_or(1) as i64;
    let page_size = q.page_size.unwrap_or(10) as i64;
    let params = PaginationParams::new(page, page_size);

    let res =
        checkpoint_service::list_checkpoints(&state.pool, params, q.job_id, q.dataset_id).await?;
    Ok(Json(ApiResponse::success(res.data)))
}

/// `GET /api/v1/checkpoints/:id` — 详情。
#[utoipa::path(get,path="/api/v1/checkpoints/{id}",tag="checkpoints",responses((status=200,description="checkpoint",body=crate::models::checkpoint::CheckpointResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_checkpoint(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<CheckpointResponse>>> {
    let ckpt = checkpoint_service::get_checkpoint(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(ckpt)))
}

/// `GET /api/v1/checkpoints/latest/:jobId` — 该作业最新检查点。
///
/// 复用列表服务按 `job_id` 过滤、取第一条；无检查点返回 404。
/// 前端作业详情对话框据此高亮"最新检查点"。
#[utoipa::path(get,path="/api/v1/checkpoints/latest/{job_id}",tag="checkpoints",responses((status=200,description="latest checkpoint",body=crate::models::checkpoint::CheckpointResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_latest_checkpoint(
    State(state): State<AppState>,
    Path(job_id): Path<i64>,
) -> AppResult<Json<ApiResponse<CheckpointResponse>>> {
    let params = PaginationParams::new(1, 1);
    let res = checkpoint_service::list_checkpoints(&state.pool, params, Some(job_id), None).await?;
    let latest = res
        .data
        .into_iter()
        .next()
        .ok_or_else(|| AppError::not_found("no checkpoint for this job"))?;
    Ok(Json(ApiResponse::success(latest)))
}

/// `POST /api/v1/checkpoints` — 创建（201）。
#[utoipa::path(post,path="/api/v1/checkpoints",request_body=CreateCheckpointRequest,tag="checkpoints",responses((status=201,description="created",body=crate::models::checkpoint::CheckpointResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
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
#[utoipa::path(put,path="/api/v1/checkpoints/{id}",request_body=UpdateCheckpointRequest,tag="checkpoints",responses((status=200,description="updated",body=crate::models::checkpoint::CheckpointResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
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
#[utoipa::path(delete,path="/api/v1/checkpoints/{id}",tag="checkpoints",responses((status=204,description="deleted"),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn delete_checkpoint(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    checkpoint_service::delete_checkpoint(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
