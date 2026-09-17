//! SchedulerIntegration 外部调度器集成模型（对齐 WP-P2-B4 调度域规格）。
//!
//! 落库表为 `migrations/007_b4_scheduler.sql` 建的 `scheduler_integrations`：
//! cluster_id 外键 / scheduler_type(kubernetes|yarn|slurm|custom) / endpoint /
//! auth_type(none|bearer|tls|mtls) / credentials·config 以 JSON TEXT 落库 /
//! status(connected|disconnected|error) / version / last_heartbeat 可空。
//!
//! `deleted_at` 在对外视图中剔除（对齐 GORM `gorm.DeletedAt` + `json:"-"`）。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, Json, PaginatedResult, PaginationParams, SoftDelete,
};
use serde_json::Value;

/// 调度器类型枚举常量（对齐 scheduler_type：kubernetes / yarn / slurm / custom）。
pub mod scheduler_type {
    pub const KUBERNETES: &str = "kubernetes";
    pub const YARN: &str = "yarn";
    pub const SLURM: &str = "slurm";
    pub const CUSTOM: &str = "custom";
}

/// 调度器连接状态枚举常量（对齐 status：connected / disconnected / error）。
pub mod scheduler_status {
    pub const CONNECTED: &str = "connected";
    pub const DISCONNECTED: &str = "disconnected";
    pub const ERROR: &str = "error";
}

/// 认证类型枚举常量（对齐 auth_type：none / bearer / tls / mtls）。
pub mod auth_type {
    pub const NONE: &str = "none";
    pub const BEARER: &str = "bearer";
    pub const TLS: &str = "tls";
    pub const MTLS: &str = "mtls";
}

/// `scheduler_integrations` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SchedulerIntegration {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub scheduler_type: String,
    pub endpoint: String,
    pub auth_type: String,
    /// 凭证，以 JSON TEXT 落库（脱敏后对外）。
    pub credentials: Json<Value>,
    pub status: String,
    pub cluster_id: i64,
    pub version: String,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub config: Json<Value>,
}

impl HasTimestamps for SchedulerIntegration {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for SchedulerIntegration {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外调度器集成视图（剔除软删除列 `deleted_at` 与敏感 `credentials`，对齐 Go 脱敏惯例）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchedulerIntegrationResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub description: String,
    pub scheduler_type: String,
    pub endpoint: String,
    pub auth_type: String,
    pub status: String,
    pub cluster_id: i64,
    pub version: String,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub config: Value,
}

impl From<SchedulerIntegration> for SchedulerIntegrationResponse {
    fn from(s: SchedulerIntegration) -> Self {
        Self {
            id: s.id,
            created_at: s.created_at,
            updated_at: s.updated_at,
            name: s.name,
            description: s.description,
            scheduler_type: s.scheduler_type,
            endpoint: s.endpoint,
            auth_type: s.auth_type,
            status: s.status,
            cluster_id: s.cluster_id,
            version: s.version,
            last_heartbeat: s.last_heartbeat,
            config: s.config.0,
        }
    }
}

/// INSERT 调度器集成入参。
pub struct NewScheduler<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub scheduler_type: &'a str,
    pub endpoint: &'a str,
    pub auth_type: &'a str,
    pub credentials: Value,
    pub status: &'a str,
    pub cluster_id: i64,
    pub version: &'a str,
    pub config: Value,
}

/// INSERT 调度器集成（自动时间戳）。
pub async fn create(pool: &SqlitePool, input: NewScheduler<'_>) -> AppResult<SchedulerIntegration> {
    let now = Utc::now();
    let res = sqlx::query(
        "INSERT INTO scheduler_integrations (created_at, updated_at, name, description, \
         scheduler_type, endpoint, auth_type, credentials, status, cluster_id, version, config) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )
    .bind(now)
    .bind(now)
    .bind(input.name)
    .bind(input.description)
    .bind(input.scheduler_type)
    .bind(input.endpoint)
    .bind(input.auth_type)
    .bind(Json(input.credentials))
    .bind(input.status)
    .bind(input.cluster_id)
    .bind(input.version)
    .bind(Json(input.config))
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted scheduler integration not found"))
}

/// 按 id 查询（默认排除软删除）。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<SchedulerIntegration>> {
    let sql = if include_deleted {
        "SELECT * FROM scheduler_integrations WHERE id = ?1"
    } else {
        "SELECT * FROM scheduler_integrations WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("scheduler_integrations"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

/// 更新最后心跳时间戳。
pub async fn touch_heartbeat(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let now = Utc::now();
    sqlx::query(
        "UPDATE scheduler_integrations SET last_heartbeat = ?1, updated_at = ?2 WHERE id = ?3",
    )
    .bind(now)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// 分页列表（动态 WHERE：软删除 + 可选 cluster_id / status / scheduler_type）。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    cluster_id: Option<i64>,
    status: Option<&str>,
    scheduler_type: Option<&str>,
) -> AppResult<PaginatedResult<SchedulerIntegration>> {
    let params = params.normalize();

    let mut where_parts: Vec<&str> = vec!["deleted_at IS NULL"];
    if cluster_id.is_some() {
        where_parts.push("cluster_id = ?");
    }
    if status.is_some() {
        where_parts.push("status = ?");
    }
    if scheduler_type.is_some() {
        where_parts.push("scheduler_type = ?");
    }
    let where_clause = where_parts.join(" AND ");

    let mut filter_binds: Vec<String> = Vec::new();
    if let Some(c) = cluster_id {
        filter_binds.push(c.to_string());
    }
    if let Some(s) = status {
        filter_binds.push(s.to_string());
    }
    if let Some(t) = scheduler_type {
        filter_binds.push(t.to_string());
    }

    let count_sql = format!("SELECT COUNT(*) FROM scheduler_integrations WHERE {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for b in &filter_binds {
        count_q = count_q.bind(b);
    }
    let total: i64 = count_q.fetch_one(pool).await?;

    let list_sql = format!(
        "SELECT * FROM scheduler_integrations WHERE {where_clause} ORDER BY id ASC LIMIT ? OFFSET ?"
    );
    let mut list_q = sqlx::query_as::<_, SchedulerIntegration>(&list_sql);
    for b in &filter_binds {
        list_q = list_q.bind(b);
    }
    list_q = list_q.bind(params.limit()).bind(params.offset());
    let rows: Vec<SchedulerIntegration> = list_q.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}
