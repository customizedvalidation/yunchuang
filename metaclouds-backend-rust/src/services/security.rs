//! SecurityPolicy 服务。
//!
//! CRUD + enable/disable + 分页/搜索/过滤 + evaluate_policy 条件评估。

use chrono::Utc;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::security_policy::{
    self, NewSecurityPolicy, PolicyListFilter, SecurityPolicy, SecurityPolicyResponse,
};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建入参。
#[derive(Debug, Clone)]
pub struct CreatePolicyInput {
    pub name: String,
    pub description: String,
    pub policy_type: Option<String>,
    pub effect: Option<String>,
    pub resources: Option<serde_json::Value>,
    pub actions: Option<serde_json::Value>,
    pub conditions: Option<serde_json::Value>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
    pub tenant_id: i64,
    pub created_by: i64,
}

/// 更新入参。
#[derive(Debug, Clone, Default)]
pub struct UpdatePolicyInput {
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

/// 创建。
pub async fn create_policy(
    pool: &SqlitePool,
    input: CreatePolicyInput,
) -> AppResult<SecurityPolicyResponse> {
    let policy = security_policy::create(
        pool,
        NewSecurityPolicy {
            name: &input.name,
            description: &input.description,
            policy_type: input.policy_type.as_deref().unwrap_or("rbac"),
            effect: input.effect.as_deref().unwrap_or("allow"),
            resources: input.resources.unwrap_or(serde_json::json!([])),
            actions: input.actions.unwrap_or(serde_json::json!([])),
            conditions: input.conditions.unwrap_or(serde_json::json!({})),
            priority: input.priority.unwrap_or(100),
            enabled: input.enabled.unwrap_or(true),
            tenant_id: input.tenant_id,
            created_by: input.created_by,
        },
    )
    .await?;
    Ok(policy.into())
}

/// 详情。
pub async fn get_policy(pool: &SqlitePool, id: i64) -> AppResult<SecurityPolicyResponse> {
    let p = security_policy::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("security policy not found"))?;
    Ok(p.into())
}

/// 分页列表。
pub async fn list_policies(
    pool: &SqlitePool,
    params: PaginationParams,
    policy_type: Option<&str>,
    enabled: Option<bool>,
    search: Option<&str>,
) -> AppResult<PaginatedResult<SecurityPolicyResponse>> {
    let rows: PaginatedResult<SecurityPolicy> = security_policy::list(
        pool,
        params,
        PolicyListFilter {
            policy_type,
            enabled,
            search,
        },
    )
    .await?;
    Ok(PaginatedResult {
        data: rows
            .data
            .into_iter()
            .map(SecurityPolicyResponse::from)
            .collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 更新。
pub async fn update_policy(
    pool: &SqlitePool,
    id: i64,
    input: UpdatePolicyInput,
) -> AppResult<SecurityPolicyResponse> {
    security_policy::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("security policy not found"))?;

    let now = Utc::now();
    let resources_str = input
        .resources
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".into()));
    let actions_str = input
        .actions
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "[]".into()));
    let conditions_str = input
        .conditions
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "{}".into()));
    let enabled_i64 = input.enabled.map(|v| if v { 1i64 } else { 0i64 });

    sqlx::query(
        "UPDATE security_policies SET \
            name = COALESCE(?, name), \
            description = COALESCE(?, description), \
            policy_type = COALESCE(?, policy_type), \
            effect = COALESCE(?, effect), \
            resources = COALESCE(?, resources), \
            actions = COALESCE(?, actions), \
            conditions = COALESCE(?, conditions), \
            priority = COALESCE(?, priority), \
            enabled = COALESCE(?, enabled), \
            updated_at = ? \
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(input.name)
    .bind(input.description)
    .bind(input.policy_type)
    .bind(input.effect)
    .bind(resources_str)
    .bind(actions_str)
    .bind(conditions_str)
    .bind(input.priority)
    .bind(enabled_i64)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let p = security_policy::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("policy disappeared after update"))?;
    Ok(p.into())
}

/// 软删除。
pub async fn delete_policy(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = security_policy::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("security policy not found"));
    }
    Ok(())
}

/// 启用策略。
pub async fn enable_policy(pool: &SqlitePool, id: i64) -> AppResult<SecurityPolicyResponse> {
    set_enabled(pool, id, true).await
}

/// 禁用策略。
pub async fn disable_policy(pool: &SqlitePool, id: i64) -> AppResult<SecurityPolicyResponse> {
    set_enabled(pool, id, false).await
}

async fn set_enabled(
    pool: &SqlitePool,
    id: i64,
    enabled: bool,
) -> AppResult<SecurityPolicyResponse> {
    security_policy::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("security policy not found"))?;
    let now = Utc::now();
    sqlx::query(
        "UPDATE security_policies SET enabled = ?, updated_at = ? WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(if enabled { 1i64 } else { 0i64 })
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    let p = security_policy::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("policy disappeared after enable/disable"))?;
    Ok(p.into())
}

/// 根据 conditions 评估是否匹配上下文。
///
/// `conditions` 形如 `{"key": "value"}` 或 `{"key": {"$gt": 5}}`。
/// 这里实现简单等值匹配：上下文中每个 key 的值与 conditions 中对应值相等即命中。
pub fn evaluate_policy(policy: &SecurityPolicy, context: &serde_json::Value) -> bool {
    let conditions = &policy.conditions.0;
    let Some(cond_obj) = conditions.as_object() else {
        // 无条件 = 始终匹配
        return true;
    };
    if cond_obj.is_empty() {
        return true;
    }
    let Some(ctx_obj) = context.as_object() else {
        return false;
    };
    for (k, v) in cond_obj {
        match ctx_obj.get(k) {
            Some(ctx_val) if ctx_val == v => continue,
            _ => return false,
        }
    }
    true
}
