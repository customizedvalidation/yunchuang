//! HTTP handlers for `/api/v1/topology/nodes`（对齐 Go `controllers/topology_controller.go`）。
//!
//! 仅做参数提取 / 校验 / 响应封装；CRUD 在 [`crate::services::topology`]。
//! 路由上：列表/详情需 JWT + `topology:read`，创建/更新/删除需 JWT + `topology:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::models::topology::TopologyResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::topology as topology_service;

/// 分页 + 过滤查询参数。
#[derive(utoipa::ToSchema, Debug, Deserialize)]
pub struct NodeListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub cluster_id: Option<i64>,
    pub role: Option<String>,
}

/// `POST /api/v1/topology/nodes` 请求体。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
pub struct CreateNodeRequest {
    #[validate(length(min = 1, message = "hostname is required"))]
    pub hostname: String,
    #[serde(default)]
    pub cluster_id: Option<i64>,
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub cpu_cores: Option<i64>,
    #[serde(default)]
    pub memory_gb: Option<i64>,
    #[serde(default)]
    pub gpu_count: Option<i64>,
    #[serde(default)]
    pub gpu_model: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub labels: Option<serde_json::Value>,
}

/// `PUT /api/v1/topology/nodes/:id` 请求体（全部可选）。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate, Default)]
pub struct UpdateNodeRequest {
    pub hostname: Option<String>,
    pub ip: Option<String>,
    pub role: Option<String>,
    pub cpu_cores: Option<i64>,
    pub memory_gb: Option<i64>,
    pub gpu_count: Option<i64>,
    pub gpu_model: Option<String>,
    pub status: Option<String>,
    pub labels: Option<serde_json::Value>,
}

/// `GET /api/v1/topology/nodes` — 分页列表（cluster_id / role 过滤）。
#[utoipa::path(get,path="/api/v1/topology",tag="topology",responses((status=200,description="nodes",body=Vec<crate::models::topology::TopologyResponse>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_nodes(
    State(state): State<AppState>,
    Query(q): Query<NodeListQuery>,
) -> AppResult<Json<ApiResponse<Vec<TopologyResponse>>>> {
    let params =
        PaginationParams::new(q.page.unwrap_or(1) as i64, q.page_size.unwrap_or(10) as i64);
    let res =
        topology_service::list_nodes(&state.pool, params, q.cluster_id, q.role.as_deref()).await?;
    Ok(Json(ApiResponse::success(res.data)))
}

/// `GET /api/v1/topology/nodes/:id` — 详情。
#[utoipa::path(get,path="/api/v1/topology/{id}",tag="topology",responses((status=200,description="node",body=crate::models::topology::TopologyResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_node(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<TopologyResponse>>> {
    let n = topology_service::get_node(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(n)))
}

/// `POST /api/v1/topology/nodes` — 创建（201）。
#[utoipa::path(post,path="/api/v1/topology",request_body=CreateNodeRequest,tag="topology",responses((status=201,description="created",body=crate::models::topology::TopologyResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn create_node(
    State(state): State<AppState>,
    Json(body): Json<CreateNodeRequest>,
) -> AppResult<WithStatus<TopologyResponse>> {
    body.validate()?;
    let input = topology_service::CreateNodeInput {
        cluster_id: body.cluster_id.unwrap_or(0),
        hostname: body.hostname,
        ip: body.ip.unwrap_or_default(),
        role: body.role.unwrap_or_else(|| "worker".to_string()),
        cpu_cores: body.cpu_cores.unwrap_or(0),
        memory_gb: body.memory_gb.unwrap_or(0),
        gpu_count: body.gpu_count.unwrap_or(0),
        gpu_model: body.gpu_model.unwrap_or_default(),
        status: body.status.unwrap_or_else(|| "unknown".to_string()),
        labels: body.labels.unwrap_or(serde_json::json!({})),
    };
    let n = topology_service::create_node(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(n),
    })
}

/// `PUT /api/v1/topology/nodes/:id` — 更新。
#[utoipa::path(put,path="/api/v1/topology/{id}",request_body=UpdateNodeRequest,tag="topology",responses((status=200,description="updated",body=crate::models::topology::TopologyResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn update_node(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateNodeRequest>,
) -> AppResult<Json<ApiResponse<TopologyResponse>>> {
    body.validate()?;
    let input = topology_service::UpdateNodeInput {
        hostname: body.hostname,
        ip: body.ip,
        role: body.role,
        cpu_cores: body.cpu_cores,
        memory_gb: body.memory_gb,
        gpu_count: body.gpu_count,
        gpu_model: body.gpu_model,
        status: body.status,
        labels: body.labels,
    };
    let n = topology_service::update_node(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(n)))
}

/// `DELETE /api/v1/topology/nodes/:id` — 软删除，204。
#[utoipa::path(delete,path="/api/v1/topology/{id}",tag="topology",responses((status=204,description="deleted"),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn delete_node(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    topology_service::delete_node(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}
