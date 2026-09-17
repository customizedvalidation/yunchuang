//! HTTP handlers for `/api/v1/security/policies`（对齐 B6 任务规格）。
//!
//! 路由上：列表/详情需 JWT + `security:read`，
//! 创建/更新/删除/enable/disable 需 JWT + `security:write`。

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde::Serialize;
use validator::Validate;

use crate::auth::middleware::AppState;
use crate::auth::Claims;
use crate::error::AppResult;
use crate::models::security_policy::SecurityPolicyResponse;
use crate::orm::PaginationParams;
use crate::response::{ApiResponse, WithStatus};
use crate::services::security::{self, CreatePolicyInput, UpdatePolicyInput};

/// 分页 + 过滤查询参数。
#[derive(Debug, Deserialize)]
pub struct PolicyListQuery {
    pub page: Option<u32>,
    pub page_size: Option<u32>,
    pub policy_type: Option<String>,
    pub enabled: Option<bool>,
    pub search: Option<String>,
}

/// `POST /api/v1/security/policies` 请求体。
#[derive(Debug, Deserialize, Validate)]
pub struct CreatePolicyRequest {
    #[validate(length(min = 1, message = "name is required"))]
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub policy_type: Option<String>,
    #[serde(default)]
    pub effect: Option<String>,
    #[serde(default)]
    pub resources: Option<serde_json::Value>,
    #[serde(default)]
    pub actions: Option<serde_json::Value>,
    #[serde(default)]
    pub conditions: Option<serde_json::Value>,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub tenant_id: Option<i64>,
}

/// `PUT /api/v1/security/policies/:id` 请求体（全部可选）。
#[derive(Debug, Deserialize, Validate, Default)]
pub struct UpdatePolicyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub policy_type: Option<String>,
    pub effect: Option<String>,
    pub resources: Option<serde_json::Value>,
    pub actions: Option<serde_json::Value>,
    pub conditions: Option<serde_json::Value>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}

/// 分页列表响应内层。
#[derive(Debug, Serialize)]
pub struct PolicyPage {
    pub data: Vec<SecurityPolicyResponse>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

/// `GET /api/v1/security/policies` — 分页 + 过滤列表。
pub async fn list_policies(
    State(state): State<AppState>,
    Query(q): Query<PolicyListQuery>,
) -> AppResult<Json<ApiResponse<PolicyPage>>> {
    let page = q.page.unwrap_or(1) as i64;
    let page_size = q.page_size.unwrap_or(10) as i64;
    let params = PaginationParams::new(page, page_size);

    let res = security::list_policies(
        &state.pool,
        params,
        q.policy_type.as_deref(),
        q.enabled,
        q.search.as_deref(),
    )
    .await?;
    Ok(Json(ApiResponse::success(PolicyPage {
        data: res.data,
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })))
}

/// `GET /api/v1/security/policies/:id` — 详情。
pub async fn get_policy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<SecurityPolicyResponse>>> {
    let p = security::get_policy(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(p)))
}

/// `POST /api/v1/security/policies` — 创建（201）。
pub async fn create_policy(
    State(state): State<AppState>,
    claims: Claims,
    Json(body): Json<CreatePolicyRequest>,
) -> AppResult<WithStatus<SecurityPolicyResponse>> {
    body.validate()?;
    let input = CreatePolicyInput {
        name: body.name,
        description: body.description.unwrap_or_default(),
        policy_type: body.policy_type,
        effect: body.effect,
        resources: body.resources,
        actions: body.actions,
        conditions: body.conditions,
        priority: body.priority,
        enabled: body.enabled,
        tenant_id: body.tenant_id.unwrap_or(claims.tenant_id as i64),
        created_by: claims.user_id as i64,
    };
    let p = security::create_policy(&state.pool, input).await?;
    Ok(WithStatus {
        status: StatusCode::CREATED,
        inner: ApiResponse::success(p),
    })
}

/// `PUT /api/v1/security/policies/:id` — 更新。
pub async fn update_policy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdatePolicyRequest>,
) -> AppResult<Json<ApiResponse<SecurityPolicyResponse>>> {
    body.validate()?;
    let input = UpdatePolicyInput {
        name: body.name,
        description: body.description,
        policy_type: body.policy_type,
        effect: body.effect,
        resources: body.resources,
        actions: body.actions,
        conditions: body.conditions,
        priority: body.priority,
        enabled: body.enabled,
    };
    let p = security::update_policy(&state.pool, id, input).await?;
    Ok(Json(ApiResponse::success(p)))
}

/// `DELETE /api/v1/security/policies/:id` — 软删除，204。
pub async fn delete_policy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    security::delete_policy(&state.pool, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/security/policies/:id/enable` — 启用。
pub async fn enable_policy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<SecurityPolicyResponse>>> {
    let p = security::enable_policy(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(p)))
}

/// `POST /api/v1/security/policies/:id/disable` — 禁用。
pub async fn disable_policy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<SecurityPolicyResponse>>> {
    let p = security::disable_policy(&state.pool, id).await?;
    Ok(Json(ApiResponse::success(p)))
}
