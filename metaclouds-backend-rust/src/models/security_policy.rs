//! SecurityPolicy 安全策略模型。
//!
//! 字段：id / name / description / policy_type(rbac/network/quota/audit) /
//! effect(allow/deny) / resources(JSON) / actions(JSON) / conditions(JSON) /
//! priority(i32) / enabled(bool) / tenant_id(FK) / created_by(FK→users) /
//! created_at / updated_at / deleted_at。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, Json, PaginatedResult, PaginationParams, SoftDelete,
};

/// policy_type 枚举常量。
pub mod policy_type {
    pub const RBAC: &str = "rbac";
    pub const NETWORK: &str = "network";
    pub const QUOTA: &str = "quota";
    pub const AUDIT: &str = "audit";
}

/// effect 枚举常量。
pub mod effect {
    pub const ALLOW: &str = "allow";
    pub const DENY: &str = "deny";
}

/// `security_policies` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub policy_type: String,
    pub effect: String,
    pub resources: Json<serde_json::Value>,
    pub actions: Json<serde_json::Value>,
    pub conditions: Json<serde_json::Value>,
    pub priority: i32,
    pub enabled: bool,
    pub tenant_id: i64,
    pub created_by: i64,
}

impl HasTimestamps for SecurityPolicy {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for SecurityPolicy {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外 SecurityPolicy 视图。
#[derive(utoipa::ToSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecurityPolicyResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub description: String,
    pub policy_type: String,
    pub effect: String,
    pub resources: serde_json::Value,
    pub actions: serde_json::Value,
    pub conditions: serde_json::Value,
    pub priority: i32,
    pub enabled: bool,
    pub tenant_id: i64,
    pub created_by: i64,
}

impl From<SecurityPolicy> for SecurityPolicyResponse {
    fn from(p: SecurityPolicy) -> Self {
        Self {
            id: p.id,
            created_at: p.created_at,
            updated_at: p.updated_at,
            name: p.name,
            description: p.description,
            policy_type: p.policy_type,
            effect: p.effect,
            resources: p.resources.0,
            actions: p.actions.0,
            conditions: p.conditions.0,
            priority: p.priority,
            enabled: p.enabled,
            tenant_id: p.tenant_id,
            created_by: p.created_by,
        }
    }
}

/// INSERT 入参。
pub struct NewSecurityPolicy<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub policy_type: &'a str,
    pub effect: &'a str,
    pub resources: serde_json::Value,
    pub actions: serde_json::Value,
    pub conditions: serde_json::Value,
    pub priority: i32,
    pub enabled: bool,
    pub tenant_id: i64,
    pub created_by: i64,
}

/// INSERT（自动时间戳）。
pub async fn create(pool: &SqlitePool, input: NewSecurityPolicy<'_>) -> AppResult<SecurityPolicy> {
    let now = Utc::now();
    let mut policy = SecurityPolicy {
        id: 0,
        created_at: now,
        updated_at: now,
        deleted_at: None,
        name: input.name.to_string(),
        description: input.description.to_string(),
        policy_type: input.policy_type.to_string(),
        effect: input.effect.to_string(),
        resources: Json(input.resources),
        actions: Json(input.actions),
        conditions: Json(input.conditions),
        priority: input.priority,
        enabled: input.enabled,
        tenant_id: input.tenant_id,
        created_by: input.created_by,
    };
    policy.before_insert();

    let res = sqlx::query(
        "INSERT INTO security_policies (created_at, updated_at, name, description, policy_type, \
         effect, resources, actions, conditions, priority, enabled, tenant_id, created_by) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(policy.created_at)
    .bind(policy.updated_at)
    .bind(&policy.name)
    .bind(&policy.description)
    .bind(&policy.policy_type)
    .bind(&policy.effect)
    .bind(&policy.resources)
    .bind(&policy.actions)
    .bind(&policy.conditions)
    .bind(policy.priority)
    .bind(policy.enabled)
    .bind(policy.tenant_id)
    .bind(policy.created_by)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted security policy not found"))
}

/// 按 id 查询（默认排除软删除）。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<SecurityPolicy>> {
    let sql = if include_deleted {
        "SELECT * FROM security_policies WHERE id = ?"
    } else {
        "SELECT * FROM security_policies WHERE id = ? AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 过滤条件。
#[derive(Debug, Clone, Default)]
pub struct PolicyListFilter<'a> {
    pub policy_type: Option<&'a str>,
    pub enabled: Option<bool>,
    pub search: Option<&'a str>,
}

/// 分页列表（可按 policy_type / enabled 过滤 + name 搜索）。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    filter: PolicyListFilter<'_>,
) -> AppResult<PaginatedResult<SecurityPolicy>> {
    let params = params.normalize();

    let mut where_parts: Vec<&str> = vec!["deleted_at IS NULL"];
    if filter.policy_type.is_some() {
        where_parts.push("policy_type = ?");
    }
    if filter.enabled.is_some() {
        where_parts.push("enabled = ?");
    }
    if filter.search.is_some() {
        where_parts.push("name LIKE ?");
    }
    let where_clause = where_parts.join(" AND ");

    let mut filter_binds: Vec<String> = Vec::new();
    if let Some(v) = filter.policy_type {
        filter_binds.push(v.to_string());
    }
    if let Some(v) = filter.enabled {
        filter_binds.push(if v { "1".into() } else { "0".into() });
    }
    if let Some(v) = filter.search {
        filter_binds.push(format!("%{v}%"));
    }

    let count_sql = format!("SELECT COUNT(*) FROM security_policies WHERE {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for b in &filter_binds {
        count_q = count_q.bind(b);
    }
    let total: i64 = count_q.fetch_one(pool).await?;

    let list_sql = format!(
        "SELECT * FROM security_policies WHERE {where_clause} ORDER BY priority ASC, id ASC LIMIT ? OFFSET ?"
    );
    let mut list_q = sqlx::query_as::<_, SecurityPolicy>(&list_sql);
    for b in &filter_binds {
        list_q = list_q.bind(b);
    }
    list_q = list_q.bind(params.limit()).bind(params.offset());
    let rows: Vec<SecurityPolicy> = list_q.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("security_policies"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
