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
use crate::response::ApiResponse;
use crate::services::monitoring::{self, AlertRuleDef};

/// `GET /api/v1/monitoring/metrics?name=...` 查询参数。
#[derive(Debug, Deserialize)]
pub struct MetricsQuery {
    pub name: Option<String>,
}

/// 规则列表响应包装。
#[derive(Debug, Serialize)]
pub struct AlertRulesResponse {
    pub rules: Vec<AlertRuleDef>,
    pub total: usize,
}

/// `GET /api/v1/monitoring/dashboard` — 返回 13 个业务指标。
pub async fn get_dashboard(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let stats = monitoring::get_dashboard_stats(&state.pool).await?;
    Ok(Json(ApiResponse::success(stats)))
}

/// `GET /api/v1/monitoring/metrics` — 全部指标或按 name 过滤。
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
pub async fn list_alert_rules() -> Json<ApiResponse<AlertRulesResponse>> {
    let rules = monitoring::list_alert_rules();
    let total = rules.len();
    Json(ApiResponse::success(AlertRulesResponse { rules, total }))
}

/// `POST /api/v1/monitoring/alert-rules/evaluate` — 评估规则。
pub async fn evaluate_alert_rules(
    State(state): State<AppState>,
) -> AppResult<Json<ApiResponse<serde_json::Value>>> {
    let triggered = monitoring::evaluate_alert_rules(&state.pool).await?;
    Ok(Json(ApiResponse::success(serde_json::json!({
        "triggered": triggered,
        "count": triggered.len(),
    }))))
}
