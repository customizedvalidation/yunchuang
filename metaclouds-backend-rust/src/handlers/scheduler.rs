//! HTTP handlers for `/api/v1/schedulers`（对齐 WP-P2-B4 调度域规格）。
//!
//! 仅做参数提取 / 校验 / 响应封装；CRUD / test-connection / sync 在
//! [`crate::services::scheduler`]。路由上：读需 JWT + `scheduler:read`，
//! 写（含 test-connection / sync）需 JWT + `scheduler:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::models::scheduler_integration::SchedulerIntegrationResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::scheduler as scheduler_service;
use serde_json::Value;

/// 分页 + 过滤查询参数。
#[derive(utoipa::ToSchema, Debug, Deserialize)]
pub struct SchedulerListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub cluster_id: Option<i64>,
    pub status: Option<String>,
    pub scheduler_type: Option<String>,
}

/// `POST /api/v1/schedulers` 请求体。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
pub struct CreateSchedulerRequest {
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub scheduler_type: Option<String>,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub auth_type: Option<String>,
    #[serde(default)]
    pub credentials: Option<Value>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub cluster_id: Option<i64>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub config: Option<Value>,
}

/// `PUT /api/v1/schedulers/:id` 请求体（全部可选）。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate, Default)]
pub struct UpdateSchedulerRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub scheduler_type: Option<String>,
    pub endpoint: Option<String>,
    pub auth_type: Option<String>,
    pub credentials: Option<Value>,
    pub status: Option<String>,
    pub version: Option<String>,
    pub config: Option<Value>,
}

/// `GET /api/v1/schedulers` — 分页列表（过滤）。
#[utoipa::path(get,path="/api/v1/schedulers",tag="schedulers",responses((status=200,description="schedulers",body=Vec<crate::models::scheduler_integration::SchedulerIntegrationResponse>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_schedulers(
    State(state): State<AppState>,
    Query(q): Query<SchedulerListQuery>,
) -> AppResult<Json<ApiResponse<Vec<SchedulerIntegrationResponse>>>> {
    let params =
        PaginationParams::new(q.page.unwrap_or(1) as i64, q.page_size.unwrap_or(10) as i64);
    let res = scheduler_service::list_schedulers(
        &state.pool,
        params,
        q.cluster_id,
        q.status.as_deref(),
        q.scheduler_type.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::success(res.data)))
}

/// `GET /api/v1/schedulers/:id` — 详情。
#[utoipa::path(get,path="/api/v1/schedulers/{id}",tag="schedulers",responses((status=200,description="scheduler",body=crate::models::scheduler_integration::SchedulerIntegrationResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_scheduler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<SchedulerIntegrationResponse>>> {
    let s = scheduler_service::get_scheduler(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(s)))
}

/// `POST /api/v1/schedulers` — 创建（201）。
#[utoipa::path(post,path="/api/v1/schedulers",request_body=CreateSchedulerRequest,tag="schedulers",responses((status=201,description="created",body=crate::models::scheduler_integration::SchedulerIntegrationResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn create_scheduler(
    State(state): State<AppState>,
    Json(body): Json<CreateSchedulerRequest>,
) -> AppResult<WithStatus<SchedulerIntegrationResponse>> {
    body.validate()?;
    let input = scheduler_service::CreateSchedulerInput {
        name: body.name,
        description: body.description.unwrap_or_default(),
        scheduler_type: body
            .scheduler_type
            .unwrap_or_else(|| "kubernetes".to_string()),
        endpoint: body.endpoint.unwrap_or_default(),
        auth_type: body.auth_type.unwrap_or_default(),
        credentials: body
            .credentials
            .unwrap_or(Value::Object(serde_json::Map::new())),
        status: body.status,
        cluster_id: body.cluster_id.unwrap_or(0),
        version: body.version.unwrap_or_default(),
        config: body.config.unwrap_or(Value::Object(serde_json::Map::new())),
    };
    let s = scheduler_service::create_scheduler(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(s),
    })
}

/// `PUT /api/v1/schedulers/:id` — 更新。
#[utoipa::path(put,path="/api/v1/schedulers/{id}",request_body=UpdateSchedulerRequest,tag="schedulers",responses((status=200,description="updated",body=crate::models::scheduler_integration::SchedulerIntegrationResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn update_scheduler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateSchedulerRequest>,
) -> AppResult<Json<ApiResponse<SchedulerIntegrationResponse>>> {
    body.validate()?;
    let input = scheduler_service::UpdateSchedulerInput {
        name: body.name,
        description: body.description,
        scheduler_type: body.scheduler_type,
        endpoint: body.endpoint,
        auth_type: body.auth_type,
        credentials: body.credentials,
        status: body.status,
        version: body.version,
        config: body.config,
    };
    let s = scheduler_service::update_scheduler(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(s)))
}

/// `DELETE /api/v1/schedulers/:id` — 软删除，204。
#[utoipa::path(delete,path="/api/v1/schedulers/{id}",tag="schedulers",responses((status=204,description="deleted"),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn delete_scheduler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    scheduler_service::delete_scheduler(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/schedulers/:id/test-connection` — mock 测试连接。
#[utoipa::path(post,path="/api/v1/schedulers/{id}/test-connection",tag="schedulers",responses((status=200,description="connection test",body=crate::services::scheduler::ConnectionTest),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn test_connection(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<scheduler_service::ConnectionTest>>> {
    let r = scheduler_service::test_connection(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(r)))
}

/// `POST /api/v1/schedulers/:id/sync` — mock 同步资源。
#[utoipa::path(post,path="/api/v1/schedulers/{id}/sync",tag="schedulers",responses((status=200,description="sync result",body=crate::services::scheduler::ResourceSync),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn sync_resources(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<scheduler_service::ResourceSync>>> {
    let r = scheduler_service::sync_resources(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(r)))
}
