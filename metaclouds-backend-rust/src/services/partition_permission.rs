//! 分区授权服务（对齐 WP-P2-B4 调度域规格）。
//!
//! grant / revoke / list / check。授权表无软删除，revoke 为物理删除。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::partition::partition_status;
use crate::models::partition_permission::{self, NewPermission, PartitionPermissionResponse};
use chrono::DateTime;
use chrono::Utc;

/// 授予授权入参。
#[derive(Debug, Clone)]
pub struct GrantPermissionInput {
    pub partition_id: i64,
    pub user_id: i64,
    pub tenant_id: i64,
    pub permission_type: String,
    pub granted_by: i64,
    pub expires_at: Option<DateTime<Utc>>,
}

/// check_permission 结果：是否允许 + 命中的权限级别。
#[derive(Debug, Clone, serde::Serialize)]
pub struct PermissionCheck {
    pub allowed: bool,
    pub permission_type: Option<String>,
}

/// 权限等级排序：read < write < admin。
fn rank(level: &str) -> i32 {
    match level {
        "admin" => 3,
        "write" => 2,
        "read" => 1,
        _ => 0,
    }
}

/// 授予（或更新）某 principal 对某分区的授权。
pub async fn grant_permission(
    pool: &SqlitePool,
    input: GrantPermissionInput,
) -> AppResult<PartitionPermissionResponse> {
    // 分区必须存在（对齐 Go SetPartitionPermission 先校验分区）。
    let _part = crate::models::partition::get_by_id(pool, input.partition_id, false)
        .await?
        .ok_or_else(|| AppError::not_found("partition not found"))?;

    // 已存在同 (partition_id, user_id) 授权则更新级别，否则新建。
    let existing: Option<(i64,)> = sqlx::query_as(
        "SELECT id FROM partition_permissions WHERE partition_id = ?1 AND user_id = ?2 LIMIT 1",
    )
    .bind(input.partition_id)
    .bind(input.user_id)
    .fetch_optional(pool)
    .await?;

    let perm = if let Some((id,)) = existing {
        let now = Utc::now();
        sqlx::query(
            "UPDATE partition_permissions SET permission_type = ?1, granted_by = ?2, \
             granted_at = ?3, expires_at = ?4, updated_at = ?5 WHERE id = ?6",
        )
        .bind(&input.permission_type)
        .bind(input.granted_by)
        .bind(now)
        .bind(input.expires_at)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
        partition_permission::get_by_id(pool, id)
            .await?
            .ok_or_else(|| AppError::internal("partition permission disappeared after update"))?
    } else {
        partition_permission::create(
            pool,
            NewPermission {
                partition_id: input.partition_id,
                user_id: input.user_id,
                tenant_id: input.tenant_id,
                permission_type: input.permission_type,
                granted_by: input.granted_by,
                expires_at: input.expires_at,
            },
        )
        .await?
    };
    Ok(perm.into())
}

/// 撤销授权（物理删除，404 若不存在）。
pub async fn revoke_permission(pool: &SqlitePool, perm_id: i64) -> AppResult<()> {
    let hit = partition_permission::delete(pool, perm_id).await?;
    if !hit {
        return Err(AppError::not_found("partition permission not found"));
    }
    Ok(())
}

/// 列出某分区的全部授权。
pub async fn list_permissions_by_partition(
    pool: &SqlitePool,
    partition_id: i64,
) -> AppResult<Vec<PartitionPermissionResponse>> {
    let rows = partition_permission::list_by_partition(pool, partition_id).await?;
    Ok(rows
        .into_iter()
        .map(PartitionPermissionResponse::from)
        .collect())
}

/// 列出某用户的全部授权。
pub async fn list_permissions_by_user(
    pool: &SqlitePool,
    user_id: i64,
) -> AppResult<Vec<PartitionPermissionResponse>> {
    let rows = partition_permission::list_by_user(pool, user_id).await?;
    Ok(rows
        .into_iter()
        .map(PartitionPermissionResponse::from)
        .collect())
}

/// 校验用户对分区是否具备指定权限级别（read/write/admin）。
///
/// 未找到授权记录默认无访问（对齐 Go CheckPartitionAccess fail-closed）；
/// 已过期（expires_at < now）的授权视为无效。
pub async fn check_permission(
    pool: &SqlitePool,
    user_id: i64,
    partition_id: i64,
    required: &str,
) -> AppResult<PermissionCheck> {
    let now = Utc::now();
    let rows: Vec<(String, Option<DateTime<Utc>>)> = sqlx::query_as(
        "SELECT permission_type, expires_at FROM partition_permissions \
         WHERE partition_id = ?1 AND user_id = ?2",
    )
    .bind(partition_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let required_rank = rank(required);
    for (ptype, expires_at) in rows {
        if let Some(exp) = expires_at {
            if exp < now {
                continue;
            }
        }
        if rank(&ptype) >= required_rank {
            return Ok(PermissionCheck {
                allowed: true,
                permission_type: Some(ptype),
            });
        }
    }
    Ok(PermissionCheck {
        allowed: false,
        permission_type: None,
    })
}

/// 校验分区当前状态是否可用（active），用于业务前置。
#[allow(dead_code)]
pub(crate) fn partition_usable(status: &str) -> bool {
    status == partition_status::ACTIVE
}
