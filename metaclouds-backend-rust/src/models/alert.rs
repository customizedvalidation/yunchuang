//! Alert 告警模型。
//!
//! 字段：id / name / description / severity(info/warning/critical) /
//! kind(=type 列: resource/job/gpu/system/cluster) / status(active/acknowledged/resolved) /
//! source / message / cluster_id(FK, nullable) / job_id(FK, nullable) /
//! resource_id(FK, nullable) / tenant_id(FK) / triggered_at / acknowledged_at(nullable) /
//! resolved_at(nullable) / acknowledged_by(FK→users, nullable) / metadata(JSON) /
//! created_at / updated_at / deleted_at。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, Json, PaginatedResult, PaginationParams, SoftDelete,
};

/// severity 枚举常量。
pub mod severity {
    pub const INFO: &str = "info";
    pub const WARNING: &str = "warning";
    pub const CRITICAL: &str = "critical";
}

/// type 枚举常量（Rust 侧仍叫 kind，DB 列名 type）。
pub mod alert_type {
    pub const RESOURCE: &str = "resource";
    pub const JOB: &str = "job";
    pub const GPU: &str = "gpu";
    pub const SYSTEM: &str = "system";
    pub const CLUSTER: &str = "cluster";
}

/// status 枚举常量。
pub mod status {
    pub const ACTIVE: &str = "active";
    pub const ACKNOWLEDGED: &str = "acknowledged";
    pub const RESOLVED: &str = "resolved";
}

/// `alerts` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Alert {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub severity: String,
    /// DB 列名为 `type`（SQL 关键字），Rust 字段名用 `kind` 规避。
    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub kind: String,
    pub status: String,
    pub source: String,
    pub message: String,
    pub cluster_id: Option<i64>,
    pub job_id: Option<i64>,
    pub resource_id: Option<i64>,
    pub tenant_id: i64,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<i64>,
    pub metadata: Json<serde_json::Value>,
}

impl HasTimestamps for Alert {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for Alert {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外 Alert 视图。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AlertResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub description: String,
    pub severity: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub status: String,
    pub source: String,
    pub message: String,
    pub cluster_id: Option<i64>,
    pub job_id: Option<i64>,
    pub resource_id: Option<i64>,
    pub tenant_id: i64,
    pub triggered_at: DateTime<Utc>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub acknowledged_by: Option<i64>,
    pub metadata: serde_json::Value,
}

impl From<Alert> for AlertResponse {
    fn from(a: Alert) -> Self {
        Self {
            id: a.id,
            created_at: a.created_at,
            updated_at: a.updated_at,
            name: a.name,
            description: a.description,
            severity: a.severity,
            kind: a.kind,
            status: a.status,
            source: a.source,
            message: a.message,
            cluster_id: a.cluster_id,
            job_id: a.job_id,
            resource_id: a.resource_id,
            tenant_id: a.tenant_id,
            triggered_at: a.triggered_at,
            acknowledged_at: a.acknowledged_at,
            resolved_at: a.resolved_at,
            acknowledged_by: a.acknowledged_by,
            metadata: a.metadata.0,
        }
    }
}

/// INSERT Alert 入参。
pub struct NewAlert<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub severity: &'a str,
    pub kind: &'a str,
    pub source: &'a str,
    pub message: &'a str,
    pub cluster_id: Option<i64>,
    pub job_id: Option<i64>,
    pub resource_id: Option<i64>,
    pub tenant_id: i64,
    pub metadata: serde_json::Value,
}

/// INSERT（自动时间戳 + triggered_at 默认 now + status 默认 active）。
pub async fn create(pool: &SqlitePool, input: NewAlert<'_>) -> AppResult<Alert> {
    let now = Utc::now();
    let mut alert = Alert {
        id: 0,
        created_at: now,
        updated_at: now,
        deleted_at: None,
        name: input.name.to_string(),
        description: input.description.to_string(),
        severity: input.severity.to_string(),
        kind: input.kind.to_string(),
        status: status::ACTIVE.to_string(),
        source: input.source.to_string(),
        message: input.message.to_string(),
        cluster_id: input.cluster_id,
        job_id: input.job_id,
        resource_id: input.resource_id,
        tenant_id: input.tenant_id,
        triggered_at: now,
        acknowledged_at: None,
        resolved_at: None,
        acknowledged_by: None,
        metadata: Json(input.metadata),
    };
    alert.before_insert();

    let res = sqlx::query(
        "INSERT INTO alerts (created_at, updated_at, name, description, severity, type, status, \
         source, message, cluster_id, job_id, resource_id, tenant_id, triggered_at, metadata) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(alert.created_at)
    .bind(alert.updated_at)
    .bind(&alert.name)
    .bind(&alert.description)
    .bind(&alert.severity)
    .bind(&alert.kind)
    .bind(&alert.status)
    .bind(&alert.source)
    .bind(&alert.message)
    .bind(alert.cluster_id)
    .bind(alert.job_id)
    .bind(alert.resource_id)
    .bind(alert.tenant_id)
    .bind(alert.triggered_at)
    .bind(&alert.metadata)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted alert not found"))
}

/// 按 id 查询（默认排除软删除）。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<Alert>> {
    let sql = if include_deleted {
        "SELECT * FROM alerts WHERE id = ?"
    } else {
        "SELECT * FROM alerts WHERE id = ? AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 过滤条件（list 用）。
#[derive(Debug, Clone, Default)]
pub struct AlertListFilter<'a> {
    pub severity: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub alert_status: Option<&'a str>,
    pub cluster_id: Option<i64>,
    pub tenant_id: Option<i64>,
    pub search: Option<&'a str>,
}

/// 分页列表（可按 severity / type / status / cluster_id / tenant_id 过滤 + name 搜索）。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    filter: AlertListFilter<'_>,
) -> AppResult<PaginatedResult<Alert>> {
    let params = params.normalize();

    let mut where_parts: Vec<&str> = vec!["deleted_at IS NULL"];
    if filter.severity.is_some() {
        where_parts.push("severity = ?");
    }
    if filter.kind.is_some() {
        where_parts.push("type = ?");
    }
    if filter.alert_status.is_some() {
        where_parts.push("status = ?");
    }
    if filter.cluster_id.is_some() {
        where_parts.push("cluster_id = ?");
    }
    if filter.tenant_id.is_some() {
        where_parts.push("tenant_id = ?");
    }
    if filter.search.is_some() {
        where_parts.push("name LIKE ?");
    }
    let where_clause = where_parts.join(" AND ");

    // 收集过滤绑定值，保持与 where_parts 中可选项顺序一致。
    let mut filter_binds: Vec<String> = Vec::new();
    if let Some(v) = filter.severity {
        filter_binds.push(v.to_string());
    }
    if let Some(v) = filter.kind {
        filter_binds.push(v.to_string());
    }
    if let Some(v) = filter.alert_status {
        filter_binds.push(v.to_string());
    }
    if let Some(v) = filter.cluster_id {
        filter_binds.push(v.to_string());
    }
    if let Some(v) = filter.tenant_id {
        filter_binds.push(v.to_string());
    }
    if let Some(v) = filter.search {
        filter_binds.push(format!("%{v}%"));
    }

    let count_sql = format!("SELECT COUNT(*) FROM alerts WHERE {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for b in &filter_binds {
        count_q = count_q.bind(b);
    }
    let total: i64 = count_q.fetch_one(pool).await?;

    let list_sql =
        format!("SELECT * FROM alerts WHERE {where_clause} ORDER BY id DESC LIMIT ? OFFSET ?");
    let mut list_q = sqlx::query_as::<_, Alert>(&list_sql);
    for b in &filter_binds {
        list_q = list_q.bind(b);
    }
    list_q = list_q.bind(params.limit()).bind(params.offset());
    let rows: Vec<Alert> = list_q.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("alerts"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

/// 按 severity/status 统计（dashboard stats 用）。
pub async fn count_by_severity(pool: &SqlitePool) -> AppResult<Vec<(String, i64)>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT severity, COUNT(*) FROM alerts WHERE deleted_at IS NULL GROUP BY severity",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn count_by_status(pool: &SqlitePool) -> AppResult<Vec<(String, i64)>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT status, COUNT(*) FROM alerts WHERE deleted_at IS NULL GROUP BY status",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
