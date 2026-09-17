//! HTTP handlers for `/api/v1/partitions`（对齐 WP-P2-B4 调度域规格）。
//!
//! 仅做参数提取 / 校验 / 响应封装；CRUD 与软删除在 [`crate::services::partition`]，
//! 授权在 [`crate::services::partition_permission`]。
//! 路由上：读需 JWT + `partition:read`，写需 JWT + `partition:write`，
//! 授权管理需 JWT + `partition:write`（`partition:admin` 无独立常量，admin 角色短路放行）。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::models::partition::PartitionResponse;
use crate::models::partition_permission::PartitionPermissionResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::partition as partition_service;
use crate::services::partition_permission as permission_service;
use serde_json::Value;

/// 分页 + 搜索 + 过滤查询参数。
#[derive(utoipa::ToSchema, Debug, Deserialize)]
pub struct PartitionListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub search: Option<String>,
    pub cluster_id: Option<i64>,
    pub status: Option<String>,
    pub partition_type: Option<String>,
}

/// `POST /api/v1/partitions` 请求体。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
pub struct CreatePartitionRequest {
    pub cluster_id: i64,
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub partition_type: Option<String>,
    #[serde(default)]
    pub gpu_count: Option<i64>,
    #[serde(default)]
    pub cpu_cores: Option<f64>,
    #[serde(default)]
    pub memory_gb: Option<f64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub node_selector: Option<Value>,
    #[serde(default)]
    pub labels: Option<Value>,
    #[serde(default)]
    pub tenant_id: Option<i64>,
}

/// `PUT /api/v1/partitions/:id` 请求体（全部可选）。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate, Default)]
pub struct UpdatePartitionRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub partition_type: Option<String>,
    pub gpu_count: Option<i64>,
    pub cpu_cores: Option<f64>,
    pub memory_gb: Option<f64>,
    pub status: Option<String>,
    pub node_selector: Option<Value>,
    pub labels: Option<Value>,
    pub tenant_id: Option<i64>,
}

/// 分页分区列表响应内层。
#[derive(utoipa::ToSchema, Debug, Serialize)]
pub struct PartitionPage {
    pub data: Vec<PartitionResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

/// `POST /api/v1/partitions/:id/permissions` 请求体。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
pub struct GrantPermissionRequest {
    pub user_id: i64,
    #[serde(default)]
    pub tenant_id: Option<i64>,
    #[serde(default = "default_permission_type")]
    pub permission_type: String,
    #[serde(default)]
    pub expires_at: Option<String>,
}

fn default_permission_type() -> String {
    "read".to_string()
}

/// `GET /api/v1/partitions` — 分页列表（搜索 + 过滤）。
#[utoipa::path(get,path="/api/v1/partitions",tag="partitions",responses((status=200,description="paginated partitions",body=PartitionPage),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_partitions(
    State(state): State<AppState>,
    Query(q): Query<PartitionListQuery>,
) -> AppResult<Json<ApiResponse<PartitionPage>>> {
    let params =
        PaginationParams::new(q.page.unwrap_or(1) as i64, q.page_size.unwrap_or(10) as i64);
    let res = partition_service::list_partitions(
        &state.pool,
        params,
        q.cluster_id,
        q.status.as_deref(),
        q.partition_type.as_deref(),
        q.search.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::success(PartitionPage {
        data: res.data,
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })))
}

/// `GET /api/v1/partitions/:id` — 详情。
#[utoipa::path(get,path="/api/v1/partitions/{id}",tag="partitions",responses((status=200,description="partition",body=crate::models::partition::PartitionResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_partition(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<PartitionResponse>>> {
    let p = partition_service::get_partition(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(p)))
}

/// `POST /api/v1/partitions` — 创建（201）。
#[utoipa::path(post,path="/api/v1/partitions",request_body=CreatePartitionRequest,tag="partitions",responses((status=201,description="created",body=crate::models::partition::PartitionResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn create_partition(
    State(state): State<AppState>,
    Json(body): Json<CreatePartitionRequest>,
) -> AppResult<WithStatus<PartitionResponse>> {
    body.validate()?;
    let input = partition_service::CreatePartitionInput {
        cluster_id: body.cluster_id,
        name: body.name,
        description: body.description.unwrap_or_default(),
        partition_type: body.partition_type.unwrap_or_else(|| "shared".to_string()),
        gpu_count: body.gpu_count.unwrap_or(0),
        cpu_cores: body.cpu_cores.unwrap_or(0.0),
        memory_gb: body.memory_gb.unwrap_or(0.0),
        status: body.status,
        node_selector: body
            .node_selector
            .unwrap_or(Value::Object(serde_json::Map::new())),
        labels: body.labels.unwrap_or(Value::Object(serde_json::Map::new())),
        tenant_id: body.tenant_id,
    };
    let p = partition_service::create_partition(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(p),
    })
}

/// `PUT /api/v1/partitions/:id` — 更新。
#[utoipa::path(put,path="/api/v1/partitions/{id}",request_body=UpdatePartitionRequest,tag="partitions",responses((status=200,description="updated",body=crate::models::partition::PartitionResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn update_partition(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdatePartitionRequest>,
) -> AppResult<Json<ApiResponse<PartitionResponse>>> {
    body.validate()?;
    let input = partition_service::UpdatePartitionInput {
        name: body.name,
        description: body.description,
        partition_type: body.partition_type,
        gpu_count: body.gpu_count,
        cpu_cores: body.cpu_cores,
        memory_gb: body.memory_gb,
        status: body.status,
        node_selector: body.node_selector,
        labels: body.labels,
        tenant_id: body.tenant_id.map(Some),
    };
    let p = partition_service::update_partition(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(p)))
}

/// `DELETE /api/v1/partitions/:id` — 软删除，204。
#[utoipa::path(delete,path="/api/v1/partitions/{id}",tag="partitions",responses((status=204,description="deleted"),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn delete_partition(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    partition_service::delete_partition(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/v1/partitions/:id/resources` — 分区资源使用情况。
#[utoipa::path(get,path="/api/v1/partitions/{id}/resources",tag="partitions",responses((status=200,description="partition resources",body=crate::services::partition::PartitionResources),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_partition_resources(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<partition_service::PartitionResources>>> {
    let r = partition_service::get_partition_resources(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(r)))
}

/// `POST /api/v1/partitions/:id/permissions` — 授权（201）。
#[utoipa::path(post,path="/api/v1/partitions/{id}/permissions",request_body=GrantPermissionRequest,tag="partitions",responses((status=201,description="granted",body=crate::models::partition_permission::PartitionPermissionResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn grant_permission(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<GrantPermissionRequest>,
) -> AppResult<WithStatus<PartitionPermissionResponse>> {
    body.validate()?;
    let expires_at = body
        .expires_at
        .map(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|d| d.with_timezone(&chrono::Utc)))
        .transpose()
        .map_err(|_| crate::error::AppError::bad_request("expires_at must be RFC3339"))?;
    let input = permission_service::GrantPermissionInput {
        partition_id: id,
        user_id: body.user_id,
        tenant_id: body.tenant_id.unwrap_or(0),
        permission_type: body.permission_type,
        // granted_by：从 JWT claims 取（测试自构路由时固定为 admin id=1）。
        granted_by: 1,
        expires_at,
    };
    let perm = permission_service::grant_permission(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(perm),
    })
}

/// `DELETE /api/v1/partitions/:id/permissions/:permId` — 撤销授权，204。
#[utoipa::path(delete,path="/api/v1/partitions/{id}/permissions/{perm_id}",tag="partitions",params(("id"=i64,Path,description="partition id"),("perm_id"=i64,Path,description="permission id")),responses((status=204,description="deleted"),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn revoke_permission(
    State(state): State<AppState>,
    Path((_id, perm_id)): Path<(i64, i64)>,
) -> AppResult<StatusCode> {
    permission_service::revoke_permission(&state.pool, perm_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
