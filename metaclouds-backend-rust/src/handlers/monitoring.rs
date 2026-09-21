//! HTTP handlers for `/api/v1/monitoring`（对齐 B6 任务规格）。
//!
//! - `GET  /api/v1/monitoring/dashboard`       — JWT + monitoring:read，返回 13 指标
//! - `GET  /api/v1/monitoring/metrics`          — JWT + monitoring:read，可按 name 过滤
//! - `GET  /api/v1/monitoring/alert-rules`     — JWT + monitoring:read，返回 16 规则
//! - `POST /api/v1/monitoring/alert-rules/evaluate` — JWT + monitoring:write，评估规则

use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use serde::Serialize;

use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::models::alert::AlertResponse;
use crate::orm::PaginationParams;
use crate::response::ApiResponse;
use crate::services::alert as alert_service;
use crate::services::monitoring::{self, AlertRuleDef};

/// `GET /api/v1/monitoring/metrics?name=...` 查询参数。
#[derive(utoipa::ToSchema, Debug, Deserialize)]
pub struct MetricsQuery {
    pub name: Option<String>,
}

/// 规则列表响应包装。
#[derive(utoipa::ToSchema, Debug, Serialize)]
pub struct AlertRulesResponse {
    pub rules: Vec<AlertRuleDef>,
    pub total: usize,
}

/// `GET /api/v1/monitoring/dashboard` — 返回 13 个业务指标。
#[utoipa::path(get,path="/api/v1/monitoring/dashboard",tag="monitoring",responses((status=200,description="dashboard metrics",body=serde_json::Value),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_dashboard(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let stats = monitoring::get_dashboard_stats(&state.pool).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// `GET /api/v1/monitoring/metrics` — 全部指标或按 name 过滤。
#[utoipa::path(get,path="/api/v1/monitoring/metrics",tag="monitoring",responses((status=200,description="metrics",body=serde_json::Value),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn get_metrics(
    State(state): State<AppState>,
    Query(q): Query<MetricsQuery>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    match q.name {
        Some(name) if !name.is_empty() => {
            let m = monitoring::get_metric(&state.pool, &name).await?;
            Ok(Json(ApiResponse::success(m)))
        }
        _ => {
            let all = monitoring::get_dashboard_stats(&state.pool).await?;
            Ok(Json(ApiResponse::success(all)))
        }
    }
}

/// `GET /api/v1/monitoring/alert-rules` — 返回 16 条告警规则定义。
#[utoipa::path(get,path="/api/v1/monitoring/alert-rules",tag="monitoring",responses((status=200,description="alert rules",body=AlertRulesResponse),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_alert_rules() -> Json<ApiResponse<AlertRulesResponse>> {
    let rules = monitoring::list_alert_rules();
    let total = rules.len();
    Json(ApiResponse::success(AlertRulesResponse { rules, total }))
}

/// `POST /api/v1/monitoring/alert-rules/evaluate` — 评估规则。
#[utoipa::path(post,path="/api/v1/monitoring/alert-rules/evaluate",tag="monitoring",responses((status=200,description="evaluation result",body=serde_json::Value),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn evaluate_alert_rules(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let triggered = monitoring::evaluate_alert_rules(&state.pool).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "triggered": triggered,
        "count": triggered.len(),
    }))))
}

/// `GET /api/v1/monitoring/alerts` — Vue3 别名，对齐前端 `monitoringApi.alerts()`。
///
/// 后端正式路由为 `GET /alerts`（分页）。前端监控页 / 仪表盘期望扁平告警数组
/// （不分页），故此处复用 alert service 并以大 page_size 一次性取回全部告警。
#[utoipa::path(get,path="/api/v1/monitoring/alerts",tag="monitoring",responses((status=200,description="alerts",body=Vec<AlertResponse>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_monitoring_alerts(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<Vec<AlertResponse>>>> {
    let params = PaginationParams::new(1, 1000);
    let res =
        alert_service::list_alerts(&state.pool, params, None, None, None, None, None, None).await?;
    Ok(Json(ApiResponse::success(res.data)))
}
