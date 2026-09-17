//! HTTP handlers for `/api/v1/clusters`（对齐 Go `controllers/cluster_controller.go`）。
//!
//! 仅做参数提取 / 校验 / 响应封装；CRUD 与软删除在 [`crate::services::cluster`]。
//! 路由上：列表/详情需 JWT + `cluster:read`，创建/更新/删除需 JWT + `cluster:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::models::cluster::ClusterResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::cluster as cluster_service;

/// 分页 + 搜索查询参数。
#[derive(Debug, Deserialize)]
pub struct ClusterListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub search: Option<String>,
}

/// `POST /api/v1/clusters` 请求体（对齐 Go `CreateClusterRequest`）。
#[derive(Debug, Deserialize, Validate)]
pub struct CreateClusterRequest {
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    #[validate(range(min = 0, message = "nodes must be >= 0"))]
    pub nodes: Option<i64>,
    #[serde(default)]
    pub gpus: Option<i64>,
    #[serde(default)]
    pub cpus: Option<i64>,
    #[serde(default)]
    pub memory: Option<i64>,
    #[serde(default)]
    pub storage: Option<i64>,
    #[serde(default)]
    pub network_type: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
}

/// `PUT /api/v1/clusters/:id` 请求体（全部可选）。
#[derive(Debug, Deserialize, Validate, Default)]
pub struct UpdateClusterRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    #[validate(range(min = 0, message = "nodes must be >= 0"))]
    pub nodes: Option<i64>,
    pub gpus: Option<i64>,
    pub cpus: Option<i64>,
    pub memory: Option<i64>,
    pub storage: Option<i64>,
    pub network_type: Option<String>,
    pub location: Option<String>,
}

/// 分页集群列表响应内层。
#[derive(Debug, Serialize)]
pub struct ClusterPage {
    pub data: Vec<ClusterResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

/// `GET /api/v1/clusters` — 分页列表（name 搜索）。
pub async fn list_clusters(
    State(state): State<AppState>,
    Query(q): Query<ClusterListQuery>,
) -> AppResult<Json<ApiResponse<ClusterPage>>> {
    let params =
        PaginationParams::new(q.page.unwrap_or(1) as i64, q.page_size.unwrap_or(10) as i64);
    let res = cluster_service::list_clusters(&state.pool, params, q.search.as_deref()).await?;
    Ok(Json(ApiResponse::success(ClusterPage {
        data: res.data,
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })))
}

/// `GET /api/v1/clusters/:id` — 详情。
pub async fn get_cluster(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<ClusterResponse>>> {
    let c = cluster_service::get_cluster(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(c)))
}

/// `POST /api/v1/clusters` — 创建（201）。
pub async fn create_cluster(
    State(state): State<AppState>,
    Json(body): Json<CreateClusterRequest>,
) -> AppResult<WithStatus<ClusterResponse>> {
    body.validate()?;
    let input = cluster_service::CreateClusterInput {
        name: body.name,
        description: body.description.unwrap_or_default(),
        nodes: body.nodes.unwrap_or(0),
        gpus: body.gpus.unwrap_or(0),
        cpus: body.cpus.unwrap_or(0),
        memory: body.memory.unwrap_or(0),
        storage: body.storage.unwrap_or(0),
        network_type: body.network_type.unwrap_or_default(),
        location: body.location.unwrap_or_default(),
    };
    let c = cluster_service::create_cluster(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(c),
    })
}

/// `PUT /api/v1/clusters/:id` — 更新。
pub async fn update_cluster(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateClusterRequest>,
) -> AppResult<Json<ApiResponse<ClusterResponse>>> {
    body.validate()?;
    let input = cluster_service::UpdateClusterInput {
        name: body.name,
        description: body.description,
        status: body.status,
        nodes: body.nodes,
        gpus: body.gpus,
        cpus: body.cpus,
        memory: body.memory,
        storage: body.storage,
        network_type: body.network_type,
        location: body.location,
    };
    let c = cluster_service::update_cluster(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(c)))
}

/// `DELETE /api/v1/clusters/:id` — 软删除，204。
pub async fn delete_cluster(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    cluster_service::delete_cluster(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
