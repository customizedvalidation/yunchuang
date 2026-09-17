//! HTTP handlers for `/api/v1/datasets`（对齐 Go `controllers/dataset_controller.go`）。
//!
//! 路由上：列表/详情需 JWT + `dataset:read`，创建/更新/删除需 JWT + `dataset:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::auth::Claims;
use crate::error::AppResult;
use crate::models::dataset::DatasetResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::dataset as dataset_service;

/// 分页 + 过滤查询参数。
#[derive(Debug, Deserialize)]
pub struct DatasetListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub tenant_id: Option<i64>,
    #[serde(rename = "type")]
    pub dataset_type: Option<String>,
}

/// `POST /api/v1/datasets` 请求体。
#[derive(Debug, Deserialize, Validate)]
pub struct CreateDatasetRequest {
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_dataset_type")]
    #[validate(length(min = 1, message = "type is required"))]
    pub r#type: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub size_bytes: Option<i64>,
    #[serde(default)]
    pub tenant_id: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub labels: Option<serde_json::Value>,
}

fn default_dataset_type() -> String {
    "private".to_string()
}

/// `PUT /api/v1/datasets/:id` 请求体（全部可选）。
#[derive(Debug, Deserialize, Validate, Default)]
pub struct UpdateDatasetRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub dataset_type: Option<String>,
    pub source_path: Option<String>,
    pub format: Option<String>,
    pub size_bytes: Option<i64>,
    pub status: Option<String>,
    pub labels: Option<serde_json::Value>,
}

/// 分页列表响应内层。
#[derive(Debug, Serialize)]
pub struct DatasetPage {
    pub data: Vec<DatasetResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

/// `GET /api/v1/datasets` — 分页 + 过滤列表。
pub async fn list_datasets(
    State(state): State<AppState>,
    Query(q): Query<DatasetListQuery>,
) -> AppResult<Json<ApiResponse<DatasetPage>>> {
    let page = q.page.unwrap_or(1) as i64;
    let page_size = q.page_size.unwrap_or(10) as i64;
    let params = PaginationParams::new(page, page_size);

    let res =
        dataset_service::list_datasets(&state.pool, params, q.tenant_id, q.dataset_type.as_deref())
            .await?;
    Ok(Json(ApiResponse::success(DatasetPage {
        data: res.data,
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })))
}

/// `GET /api/v1/datasets/:id` — 详情。
pub async fn get_dataset(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<DatasetResponse>>> {
    let ds = dataset_service::get_dataset(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(ds)))
}

/// `POST /api/v1/datasets` — 创建（201）。
pub async fn create_dataset(
    State(state): State<AppState>,
    claims: Claims,
    Json(body): Json<CreateDatasetRequest>,
) -> AppResult<WithStatus<DatasetResponse>> {
    body.validate()?;
    let input = dataset_service::CreateDatasetInput {
        name: body.name,
        description: body.description.unwrap_or_default(),
        dataset_type: body.r#type,
        source_path: body.source_path.unwrap_or_default(),
        format: body.format.unwrap_or_default(),
        size_bytes: body.size_bytes.unwrap_or(0),
        tenant_id: body.tenant_id.unwrap_or(claims.tenant_id as i64),
        created_by: claims.user_id as i64,
        status: body.status,
        labels: body.labels.unwrap_or(serde_json::json!({})),
    };
    let ds = dataset_service::create_dataset(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(ds),
    })
}

/// `PUT /api/v1/datasets/:id` — 更新。
pub async fn update_dataset(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateDatasetRequest>,
) -> AppResult<Json<ApiResponse<DatasetResponse>>> {
    body.validate()?;
    let input = dataset_service::UpdateDatasetInput {
        name: body.name,
        description: body.description,
        dataset_type: body.dataset_type,
        source_path: body.source_path,
        format: body.format,
        size_bytes: body.size_bytes,
        status: body.status,
        labels: body.labels,
    };
    let ds = dataset_service::update_dataset(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(ds)))
}

/// `DELETE /api/v1/datasets/:id` — 软删除，204。
pub async fn delete_dataset(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    dataset_service::delete_dataset(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
