//! HTTP handlers for `/api/v1/acceleration/suites`（对齐 Go
//! `controllers/acceleration_controller.go`）。
//!
//! 路由上：列表/详情需 JWT + `acceleration:read`，创建/更新/删除/start/stop
//! 需 JWT + `acceleration:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::auth::Claims;
use crate::error::AppResult;
use crate::models::acceleration_suite::AccelerationSuiteResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::acceleration as acceleration_service;

/// 分页 + 过滤查询参数。
#[derive(Debug, Deserialize)]
pub struct SuiteListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub tenant_id: Option<i64>,
    pub status: Option<String>,
}

/// `POST /api/v1/acceleration/suites` 请求体。
#[derive(Debug, Deserialize, Validate)]
pub struct CreateSuiteRequest {
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_suite_type")]
    pub suite_type: String,
    #[serde(default)]
    pub dataset_id: Option<i64>,
    #[serde(default)]
    pub training_config_id: Option<i64>,
    #[serde(default)]
    pub inference_config_id: Option<i64>,
    #[serde(default)]
    pub fluid_cache_id: Option<i64>,
    #[serde(default)]
    pub acceleration_config: Option<serde_json::Value>,
    #[serde(default)]
    pub tenant_id: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
}

fn default_suite_type() -> String {
    "training".to_string()
}

/// `PUT /api/v1/acceleration/suites/:id` 请求体（全部可选）。
#[derive(Debug, Deserialize, Validate, Default)]
pub struct UpdateSuiteRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub suite_type: Option<String>,
    pub dataset_id: Option<Option<i64>>,
    pub training_config_id: Option<Option<i64>>,
    pub inference_config_id: Option<Option<i64>>,
    pub fluid_cache_id: Option<Option<i64>>,
    pub acceleration_config: Option<serde_json::Value>,
    pub status: Option<String>,
}

/// 分页列表响应内层。
#[derive(Debug, Serialize)]
pub struct SuitePage {
    pub data: Vec<AccelerationSuiteResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

/// `GET /api/v1/acceleration/suites` — 分页列表。
pub async fn list_suites(
    State(state): State<AppState>,
    Query(q): Query<SuiteListQuery>,
) -> AppResult<Json<ApiResponse<SuitePage>>> {
    let page = q.page.unwrap_or(1) as i64;
    let page_size = q.page_size.unwrap_or(10) as i64;
    let params = PaginationParams::new(page, page_size);

    let res =
        acceleration_service::list_suites(&state.pool, params, q.tenant_id, q.status.as_deref())
            .await?;
    Ok(Json(ApiResponse::success(SuitePage {
        data: res.data,
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })))
}

/// `GET /api/v1/acceleration/suites/:id` — 详情（含关联对象）。
pub async fn get_suite(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<AccelerationSuiteResponse>>> {
    let suite = acceleration_service::get_suite(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(suite)))
}

/// `POST /api/v1/acceleration/suites` — 创建（201）。
pub async fn create_suite(
    State(state): State<AppState>,
    claims: Claims,
    Json(body): Json<CreateSuiteRequest>,
) -> AppResult<WithStatus<AccelerationSuiteResponse>> {
    body.validate()?;
    let input = acceleration_service::CreateSuiteInput {
        name: body.name,
        description: body.description.unwrap_or_default(),
        suite_type: body.suite_type,
        dataset_id: body.dataset_id,
        training_config_id: body.training_config_id,
        inference_config_id: body.inference_config_id,
        fluid_cache_id: body.fluid_cache_id,
        acceleration_config: body.acceleration_config.unwrap_or(serde_json::json!({})),
        tenant_id: body.tenant_id.unwrap_or(claims.tenant_id as i64),
        created_by: claims.user_id as i64,
        status: body.status,
    };
    let suite = acceleration_service::create_suite(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(suite),
    })
}

/// `PUT /api/v1/acceleration/suites/:id` — 更新。
pub async fn update_suite(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateSuiteRequest>,
) -> AppResult<Json<ApiResponse<AccelerationSuiteResponse>>> {
    body.validate()?;
    let input = acceleration_service::UpdateSuiteInput {
        name: body.name,
        description: body.description,
        suite_type: body.suite_type,
        dataset_id: body.dataset_id,
        training_config_id: body.training_config_id,
        inference_config_id: body.inference_config_id,
        fluid_cache_id: body.fluid_cache_id,
        acceleration_config: body.acceleration_config,
        status: body.status,
    };
    let suite = acceleration_service::update_suite(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(suite)))
}

/// `DELETE /api/v1/acceleration/suites/:id` — 软删除，204。
pub async fn delete_suite(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    acceleration_service::delete_suite(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/acceleration/suites/:id/start` — 启动 suite。
pub async fn start_suite(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<AccelerationSuiteResponse>>> {
    let suite = acceleration_service::start_suite(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(suite)))
}

/// `POST /api/v1/acceleration/suites/:id/stop` — 停止 suite。
pub async fn stop_suite(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<AccelerationSuiteResponse>>> {
    let suite = acceleration_service::stop_suite(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(suite)))
}
