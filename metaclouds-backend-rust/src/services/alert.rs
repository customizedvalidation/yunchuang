//! Alert 服务。
//!
//! CRUD + acknowledge/resolve 状态机 + 分页/搜索/过滤 + stats 聚合。

use chrono::Utc;
use serde::Serialize;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::alert::{self, Alert, AlertListFilter, AlertResponse, NewAlert};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建入参。
#[derive(Debug, Clone)]
pub struct CreateAlertInput {
    pub name: String,
    pub description: String,
    pub severity: Option<String>,
    pub kind: Option<String>,
    pub source: Option<String>,
    pub message: String,
    pub cluster_id: Option<i64>,
    pub job_id: Option<i64>,
    pub resource_id: Option<i64>,
    pub tenant_id: i64,
    pub metadata: Option<serde_json::Value>,
}

/// 更新入参。
#[derive(Debug, Clone, Default)]
pub struct UpdateAlertInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub severity: Option<String>,
    pub kind: Option<String>,
    pub source: Option<String>,
    pub message: Option<String>,
    pub status: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// 创建。
pub async fn create_alert(pool: &SqlitePool, input: CreateAlertInput) -> AppResult<AlertResponse> {
    let alert = alert::create(
        pool,
        NewAlert {
            name: &input.name,
            description: &input.description,
            severity: input.severity.as_deref().unwrap_or("warning"),
            kind: input.kind.as_deref().unwrap_or("system"),
            source: input.source.as_deref().unwrap_or(""),
            message: &input.message,
            cluster_id: input.cluster_id,
            job_id: input.job_id,
            resource_id: input.resource_id,
            tenant_id: input.tenant_id,
            metadata: input.metadata.unwrap_or(serde_json::json!({})),
        },
    )
    .await?;
    Ok(alert.into())
}

/// 详情。
pub async fn get_alert(pool: &SqlitePool, id: i64) -> AppResult<AlertResponse> {
    let a = alert::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("alert not found"))?;
    Ok(a.into())
}

/// 分页列表。
#[allow(clippy::too_many_arguments)]
pub async fn list_alerts(
    pool: &SqlitePool,
    params: PaginationParams,
    severity: Option<&str>,
    kind: Option<&str>,
    status: Option<&str>,
    cluster_id: Option<i64>,
    tenant_id: Option<i64>,
    search: Option<&str>,
) -> AppResult<PaginatedResult<AlertResponse>> {
    let rows: PaginatedResult<Alert> = alert::list(
        pool,
        params,
        AlertListFilter {
            severity,
            kind,
            alert_status: status,
            cluster_id,
            tenant_id,
            search,
        },
    )
    .await?;
    Ok(PaginatedResult {
        data: rows.data.into_iter().map(AlertResponse::from).collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 更新。
pub async fn update_alert(
    pool: &SqlitePool,
    id: i64,
    input: UpdateAlertInput,
) -> AppResult<AlertResponse> {
    alert::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("alert not found"))?;

    let now = Utc::now();
    let metadata_str = input
        .metadata
        .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "{}".into()));
    sqlx::query(
        "UPDATE alerts SET \
            name = COALESCE(?, name), \
            description = COALESCE(?, description), \
            severity = COALESCE(?, severity), \
            type = COALESCE(?, type), \
            source = COALESCE(?, source), \
            message = COALESCE(?, message), \
            status = COALESCE(?, status), \
            metadata = COALESCE(?, metadata), \
            updated_at = ? \
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(input.name)
    .bind(input.description)
    .bind(input.severity)
    .bind(input.kind)
    .bind(input.source)
    .bind(input.message)
    .bind(input.status)
    .bind(metadata_str)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let a = alert::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("alert disappeared after update"))?;
    Ok(a.into())
}

/// 软删除。
pub async fn delete_alert(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = alert::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("alert not found"));
    }
    Ok(())
}

/// 确认告警：status → acknowledged，记录 acknowledged_at / acknowledged_by。
pub async fn acknowledge_alert(
    pool: &SqlitePool,
    id: i64,
    user_id: i64,
) -> AppResult<AlertResponse> {
    let a = alert::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("alert not found"))?;
    if a.status == "resolved" {
        return Err(AppError::bad_request("cannot acknowledge a resolved alert"));
    }
    let now = Utc::now();
    sqlx::query(
        "UPDATE alerts SET status = ?, acknowledged_at = ?, acknowledged_by = ?, updated_at = ? \
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind("acknowledged")
    .bind(now)
    .bind(user_id)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let updated = alert::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("alert disappeared after acknowledge"))?;
    Ok(updated.into())
}

/// 解决告警：status → resolved，记录 resolved_at。
pub async fn resolve_alert(pool: &SqlitePool, id: i64) -> AppResult<AlertResponse> {
    alert::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("alert not found"))?;
    let now = Utc::now();
    sqlx::query(
        "UPDATE alerts SET status = ?, resolved_at = ?, updated_at = ? \
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind("resolved")
    .bind(now)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let updated = alert::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("alert disappeared after resolve"))?;
    Ok(updated.into())
}

/// 告警统计（按 severity / status 聚合）。
#[derive(utoipa::ToSchema, Debug, Serialize)]
pub struct AlertStats {
    pub by_severity: serde_json::Value,
    pub by_status: serde_json::Value,
    pub total: i64,
    pub active: i64,
}

pub async fn get_alert_stats(pool: &SqlitePool) -> AppResult<AlertStats> {
    let sev_rows = alert::count_by_severity(pool).await?;
    let status_rows = alert::count_by_status(pool).await?;

    let mut by_severity = serde_json::Map::new();
    for (k, v) in sev_rows {
        by_severity.insert(k, serde_json::json!(v));
    }
    let mut by_status = serde_json::Map::new();
    let mut active = 0i64;
    for (k, v) in status_rows {
        if k == "active" {
            active = v;
        }
        by_status.insert(k, serde_json::json!(v));
    }
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM alerts WHERE deleted_at IS NULL")
        .fetch_one(pool)
        .await?;

    Ok(AlertStats {
        by_severity: serde_json::Value::Object(by_severity),
        by_status: serde_json::Value::Object(by_status),
        total,
        active,
    })
}
