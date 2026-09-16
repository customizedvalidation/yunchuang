//! 基于角色的访问控制（RBAC）。
//!
//! 逐字对齐 Go 侧 `pkg/authz/authz.go`：权限常量字符串、角色枚举
//! （admin/manager/user）与角色-权限矩阵。判定保持 fail-closed：
//! 未知角色一律拒绝；`admin` 短路放行全部权限。
//!
//! 提供 `RequirePermission` / `RequireRole` 两个 axum 中间件，从请求扩展中
//! 取出 `jwt_auth` 注入的 `Claims.role` 做判定，不足时返回 403
//! "Insufficient permissions"。

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;

use crate::auth::jwt::Claims;
use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// 权限常量：字符串逐字对齐 Go `pkg/authz/authz.go`。
// ---------------------------------------------------------------------------

pub mod permissions {
    pub const CLUSTER_READ: &str = "cluster:read";
    pub const CLUSTER_WRITE: &str = "cluster:write";
    pub const RESOURCE_READ: &str = "resource:read";
    pub const RESOURCE_WRITE: &str = "resource:write";
    pub const JOB_READ: &str = "job:read";
    pub const JOB_WRITE: &str = "job:write";
    pub const JOB_SUBMIT: &str = "job:submit";
    pub const TENANT_READ: &str = "tenant:read";
    pub const TENANT_WRITE: &str = "tenant:write";
    pub const MONITORING_READ: &str = "monitoring:read";
    pub const MONITORING_WRITE: &str = "monitoring:write";
    pub const ACCELERATION_READ: &str = "acceleration:read";
    pub const ACCELERATION_WRITE: &str = "acceleration:write";
    pub const SECURITY_READ: &str = "security:read";
    pub const SECURITY_WRITE: &str = "security:write";
    pub const ADMIN: &str = "admin";

    pub const GPU_READ: &str = "gpu:read";
    pub const GPU_WRITE: &str = "gpu:write";
    pub const PARTITION_READ: &str = "partition:read";
    pub const PARTITION_WRITE: &str = "partition:write";
    pub const QUOTA_READ: &str = "quota:read";
    pub const QUOTA_WRITE: &str = "quota:write";
    pub const SCHEDULER_READ: &str = "scheduler:read";
    pub const SCHEDULER_WRITE: &str = "scheduler:write";
    pub const TOPOLOGY_READ: &str = "topology:read";
    pub const TOPOLOGY_WRITE: &str = "topology:write";
    pub const DATASET_READ: &str = "dataset:read";
    pub const DATASET_WRITE: &str = "dataset:write";
    pub const CHECKPOINT_READ: &str = "checkpoint:read";
    pub const CHECKPOINT_WRITE: &str = "checkpoint:write";
}

/// 角色枚举，对齐 Go `RoleAdmin/RoleManager/RoleUser`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Admin,
    Manager,
    User,
}

impl Role {
    /// 角色字符串（与 JWT claims.role 一致，全小写）。
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Manager => "manager",
            Role::User => "user",
        }
    }
}

/// Manager 角色被授予的权限清单，对齐 Go `rolePermissions[RoleManager]`。
const MANAGER_PERMISSIONS: &[&str] = &[
    permissions::CLUSTER_READ,
    permissions::CLUSTER_WRITE,
    permissions::RESOURCE_READ,
    permissions::RESOURCE_WRITE,
    permissions::JOB_READ,
    permissions::JOB_WRITE,
    permissions::JOB_SUBMIT,
    permissions::TENANT_READ,
    permissions::MONITORING_READ,
    permissions::MONITORING_WRITE,
    permissions::ACCELERATION_READ,
    permissions::ACCELERATION_WRITE,
    permissions::SECURITY_READ,
    permissions::GPU_READ,
    permissions::GPU_WRITE,
    permissions::PARTITION_READ,
    permissions::PARTITION_WRITE,
    permissions::QUOTA_READ,
    permissions::QUOTA_WRITE,
    permissions::SCHEDULER_READ,
    permissions::SCHEDULER_WRITE,
    permissions::TOPOLOGY_READ,
    permissions::TOPOLOGY_WRITE,
    permissions::DATASET_READ,
    permissions::DATASET_WRITE,
    permissions::CHECKPOINT_READ,
    permissions::CHECKPOINT_WRITE,
];

/// User 角色被授予的权限清单，对齐 Go `rolePermissions[RoleUser]`。
const USER_PERMISSIONS: &[&str] = &[
    permissions::CLUSTER_READ,
    permissions::RESOURCE_READ,
    permissions::JOB_READ,
    permissions::MONITORING_READ,
    permissions::ACCELERATION_READ,
    permissions::SECURITY_READ,
    permissions::GPU_READ,
    permissions::PARTITION_READ,
    permissions::QUOTA_READ,
    permissions::SCHEDULER_READ,
    permissions::TOPOLOGY_READ,
    permissions::DATASET_READ,
    permissions::CHECKPOINT_READ,
];

/// 判断角色是否具备某项权限。
///
/// - `admin` 短路放行；
/// - 其余角色查表；
/// - 未知角色（不在矩阵内）一律返回 false —— fail-closed。
pub fn has_permission(role: &str, permission: &str) -> bool {
    if role == Role::Admin.as_str() {
        return true;
    }
    let list = if role == Role::Manager.as_str() {
        MANAGER_PERMISSIONS
    } else if role == Role::User.as_str() {
        USER_PERMISSIONS
    } else {
        return false;
    };
    list.contains(&permission)
}

/// 从请求扩展中取出已认证角色；缺失上下文返回 401。
fn role_from_request(request: &Request) -> AppResult<String> {
    let claims = request
        .extensions()
        .get::<Claims>()
        .cloned()
        .ok_or_else(|| AppError::unauthorized("Authorization header is required"))?;
    Ok(claims.role)
}

/// 中间件工厂：要求调用方具备指定权限（须在 `jwt_auth` 之后层叠）。
///
/// 用法：`from_fn_with_state(permission.to_string(), require_permission)`。
pub async fn require_permission(
    State(permission): State<String>,
    request: Request,
    next: Next,
) -> AppResult<Response> {
    let role = role_from_request(&request)?;
    if !has_permission(&role, &permission) {
        tracing::warn!(
            role = %role,
            required_permission = %permission,
            "authorization denied - insufficient permission"
        );
        return Err(AppError::forbidden("Insufficient permissions"));
    }
    Ok(next.run(request).await)
}

/// 中间件工厂：要求调用方属于指定角色之一（admin 始终放行）。
///
/// 用法：`from_fn_with_state(vec!["manager".into(), ...], require_role)`。
pub async fn require_role(
    State(roles): State<Vec<String>>,
    request: Request,
    next: Next,
) -> AppResult<Response> {
    let role = role_from_request(&request)?;
    let allowed = role == Role::Admin.as_str() || roles.iter().any(|r| r == &role);
    if !allowed {
        tracing::warn!(role = %role, "authorization denied - role not allowed");
        return Err(AppError::forbidden("Insufficient permissions"));
    }
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use permissions::*;

    #[test]
    fn admin_short_circuits_everything() {
        for p in [
            CLUSTER_READ,
            CLUSTER_WRITE,
            SECURITY_WRITE,
            ADMIN,
            CHECKPOINT_WRITE,
        ] {
            assert!(has_permission("admin", p), "admin 应具备 {p}");
        }
    }

    #[test]
    fn unknown_role_is_fail_closed() {
        for p in [CLUSTER_READ, JOB_SUBMIT, ADMIN] {
            assert!(!has_permission("superguest", p));
            assert!(!has_permission("", p));
        }
    }

    #[test]
    fn user_read_only_allow_and_deny() {
        // 允许：各只读权限。
        for p in [
            CLUSTER_READ,
            RESOURCE_READ,
            JOB_READ,
            MONITORING_READ,
            ACCELERATION_READ,
            SECURITY_READ,
            GPU_READ,
            PARTITION_READ,
            QUOTA_READ,
            SCHEDULER_READ,
            TOPOLOGY_READ,
            DATASET_READ,
            CHECKPOINT_READ,
        ] {
            assert!(has_permission("user", p), "user 应具备 {p}");
        }
        // 拒绝：任何写 / 提交 / 管理员权限。
        for p in [
            CLUSTER_WRITE,
            RESOURCE_WRITE,
            JOB_WRITE,
            JOB_SUBMIT,
            TENANT_WRITE,
            MONITORING_WRITE,
            SECURITY_WRITE,
            GPU_WRITE,
            QUOTA_WRITE,
            ADMIN,
        ] {
            assert!(!has_permission("user", p), "user 不应具备 {p}");
        }
    }

    #[test]
    fn manager_write_and_submit_allow_admin_only_deny() {
        // 允许：manager 可写大部分资源并可提交作业。
        for p in [
            CLUSTER_WRITE,
            RESOURCE_WRITE,
            JOB_WRITE,
            JOB_SUBMIT,
            MONITORING_WRITE,
            GPU_WRITE,
            CHECKPOINT_WRITE,
        ] {
            assert!(has_permission("manager", p), "manager 应具备 {p}");
        }
        // 拒绝：manager 不持有租户写与管理员权限。
        for p in [TENANT_WRITE, SECURITY_WRITE, ADMIN] {
            assert!(!has_permission("manager", p), "manager 不应具备 {p}");
        }
    }

    #[test]
    fn permission_strings_match_go_contract() {
        // 逐字核对关键权限名，防止漂移。
        assert_eq!(CLUSTER_READ, "cluster:read");
        assert_eq!(JOB_SUBMIT, "job:submit");
        assert_eq!(ACCELERATION_WRITE, "acceleration:write");
        assert_eq!(TOPOLOGY_READ, "topology:read");
        assert_eq!(CHECKPOINT_WRITE, "checkpoint:write");
    }
}
