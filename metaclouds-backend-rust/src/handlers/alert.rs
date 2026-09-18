//! HTTP handlers for `/api/v1/alerts`（对齐 B6 任务规格）。
//!
//! 路由上：列表/详情/stats 需 JWT + `alert:read`，
//! 创建/更新/删除/acknowledge/resolve 需 JWT + `alert:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::auth::Claims;
use crate::error::AppResult;
use crate::models::alert::AlertResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::alert::{self, AlertStats, UpdateAlertInput};

/// 分页 + 过滤查询参数。
#[derive(utoipa::ToSchema, Debug, Deserialize)]
pub struct AlertListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub severity: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub status: Option<String>,
    pub cluster_id: Option<i64>,
    pub tenant_id: Option<i64>,
    pub search: Option<String>,
}

/// `POST /api/v1/alerts` 请求体。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate)]
pub struct CreateAlertRequest {
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub cluster_id: Option<i64>,
    #[serde(default)]
    pub job_id: Option<i64>,
    #[serde(default)]
    pub resource_id: Option<i64>,
    #[serde(default)]
    pub tenant_id: Option<i64>,
    #[serde(default)]
    pub metadata: Option<serde_json::Value>,
}

/// `PUT /api/v1/alerts/:id` 请求体（全部可选）。
#[derive(utoipa::ToSchema, Debug, Deserialize, Validate, Default)]
pub struct UpdateAlertRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub severity: Option<String>,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
    pub source: Option<String>,
    pub message: Option<String>,
    pub status: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// `GET /api/v1/alerts` — 分页 + 过滤列表。
#[utoipa::path(get,path="/api/v1/alerts",tag="alerts",responses((status=200,description="alerts",body=Vec<crate::models::alert::AlertResponse>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_alerts(
    State(state): State<AppState>,
    Query(q): Query<AlertListQuery>,
) -> AppResult<Json<ApiResponse<Vec<AlertResponse>>>> {
    let page = q.page.unwrap_or(1) as i64;
    let page_size = q.page_size.unwrap_or(10) as i64;
    let params = PaginationParams::new(page, page_size);

    let severity = q.severity.as_deref();
    let kind = q.kind.as_deref();
    let status = q.status.as_deref();
    let search = q.search.as_deref();

    let res = alert::list_alerts(
        &state.pool,
        params,
        severity,
        kind,
        status,
        q.cluster_id,
        q.tenant_id,
        search,
    )
    .await?;
    Ok(Json(ApiResponse::success(res.data)))
}

/// `GET /api/v1/alerts/:id` — 详情。
#[utoipa::path(get,path="/api/v1/alerts/{id}",tag="alerts",responses((status=200,description="alert",body=crate::models::alert::AlertResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_alert(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<AlertResponse>>> {
    let a = alert::get_alert(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(a)))
}

/// `POST /api/v1/alerts` — 创建（201）。
#[utoipa::path(post,path="/api/v1/alerts",request_body=CreateAlertRequest,tag="alerts",responses((status=201,description="created",body=crate::models::alert::AlertResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn create_alert(
    State(state): State<AppState>,
    claims: Claims,
    Json(body): Json<CreateAlertRequest>,
) -> AppResult<WithStatus<AlertResponse>> {
    body.validate()?;
    let input = alert::CreateAlertInput {
        name: body.name,
        description: body.description.unwrap_or_default(),
        severity: body.severity,
        kind: body.kind,
        source: body.source,
        message: body.message.unwrap_or_default(),
        cluster_id: body.cluster_id,
        job_id: body.job_id,
        resource_id: body.resource_id,
        tenant_id: body.tenant_id.unwrap_or(claims.tenant_id as i64),
        metadata: body.metadata,
    };
    let a = alert::create_alert(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(a),
    })
}

/// `PUT /api/v1/alerts/:id` — 更新。
#[utoipa::path(put,path="/api/v1/alerts/{id}",request_body=UpdateAlertRequest,tag="alerts",responses((status=200,description="updated",body=crate::models::alert::AlertResponse),(status=400,description="bad request",body=crate::openapi::ErrorResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse),(status=409,description="conflict",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn update_alert(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateAlertRequest>,
) -> AppResult<Json<ApiResponse<AlertResponse>>> {
    body.validate()?;
    let input = UpdateAlertInput {
        name: body.name,
        description: body.description,
        severity: body.severity,
        kind: body.kind,
        source: body.source,
        message: body.message,
        status: body.status,
        metadata: body.metadata,
    };
    let a = alert::update_alert(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(a)))
}

/// `DELETE /api/v1/alerts/:id` — 软删除，204。
#[utoipa::path(delete,path="/api/v1/alerts/{id}",tag="alerts",responses((status=204,description="deleted"),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn delete_alert(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    alert::delete_alert(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/alerts/:id/acknowledge` — 确认告警。
#[utoipa::path(post,path="/api/v1/alerts/{id}/acknowledge",tag="alerts",responses((status=200,description="acknowledged",body=crate::models::alert::AlertResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn acknowledge_alert(
    State(state): State<AppState>,
    claims: Claims,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<AlertResponse>>> {
    let a = alert::acknowledge_alert(&state.pool, id, claims.user_id as i64).await?;
    Ok(Json(ApiResponse::success(a)))
}

/// `POST /api/v1/alerts/:id/resolve` — 解决告警。
#[utoipa::path(post,path="/api/v1/alerts/{id}/resolve",tag="alerts",responses((status=200,description="resolved",body=crate::models::alert::AlertResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn resolve_alert(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<AlertResponse>>> {
    let a = alert::resolve_alert(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(a)))
}

/// `GET /api/v1/alerts/stats` — 告警统计。
#[utoipa::path(get,path="/api/v1/alerts/stats",tag="alerts",responses((status=200,description="alert stats",body=crate::services::alert::AlertStats),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_alert_stats(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<AlertStats>>> {
    let stats = alert::get_alert_stats(&state.pool).await?;
    Ok(Json(ApiResponse::success(stats)))
}
