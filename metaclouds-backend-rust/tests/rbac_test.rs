//! RBAC 权限矩阵测试：逐权限核对允许/拒绝，权限名与 Go 版逐字一致。

use metaclouds_backend_rust::authz::has_permission;
use metaclouds_backend_rust::authz::permissions as p;

/// 全部权限清单（与 Go `pkg/authz/authz.go` 常量一一对应）。
fn all_permissions() -> Vec<&'static str> {
    vec![
        p::CLUSTER_READ,
        p::CLUSTER_WRITE,
        p::RESOURCE_READ,
        p::RESOURCE_WRITE,
        p::JOB_READ,
        p::JOB_WRITE,
        p::JOB_SUBMIT,
        p::TENANT_READ,
        p::TENANT_WRITE,
        p::MONITORING_READ,
        p::MONITORING_WRITE,
        p::ACCELERATION_READ,
        p::ACCELERATION_WRITE,
        p::SECURITY_READ,
        p::SECURITY_WRITE,
        p::ADMIN,
        p::GPU_READ,
        p::GPU_WRITE,
        p::PARTITION_READ,
        p::PARTITION_WRITE,
        p::QUOTA_READ,
        p::QUOTA_WRITE,
        p::SCHEDULER_READ,
        p::SCHEDULER_WRITE,
        p::TOPOLOGY_READ,
        p::TOPOLOGY_WRITE,
        p::DATASET_READ,
        p::DATASET_WRITE,
        p::CHECKPOINT_READ,
        p::CHECKPOINT_WRITE,
    ]
}

#[test]
fn admin_holds_every_permission() {
    for perm in all_permissions() {
        assert!(has_permission("admin", perm), "admin 应具备 {perm}");
    }
}

#[test]
fn user_only_read_permissions() {
    let allowed = [
        p::CLUSTER_READ,
        p::RESOURCE_READ,
        p::JOB_READ,
        p::MONITORING_READ,
        p::ACCELERATION_READ,
        p::SECURITY_READ,
        p::GPU_READ,
        p::PARTITION_READ,
        p::QUOTA_READ,
        p::SCHEDULER_READ,
        p::TOPOLOGY_READ,
        p::DATASET_READ,
        p::CHECKPOINT_READ,
    ];
    for perm in all_permissions() {
        if allowed.contains(&perm) {
            assert!(has_permission("user", perm), "user 应具备 {perm}");
        } else {
            assert!(!has_permission("user", perm), "user 不应具备 {perm}");
        }
    }
}

#[test]
fn manager_writes_but_not_admin_only() {
    // manager 允许的写/提交权限。
    let allowed_writes = [
        p::CLUSTER_WRITE,
        p::RESOURCE_WRITE,
        p::JOB_WRITE,
        p::JOB_SUBMIT,
        p::MONITORING_WRITE,
        p::ACCELERATION_WRITE,
        p::GPU_WRITE,
        p::PARTITION_WRITE,
        p::QUOTA_WRITE,
        p::SCHEDULER_WRITE,
        p::TOPOLOGY_WRITE,
        p::DATASET_WRITE,
        p::CHECKPOINT_WRITE,
    ];
    for perm in allowed_writes {
        assert!(has_permission("manager", perm), "manager 应具备 {perm}");
    }
    // manager 不持有管理员专属权限。
    for perm in [p::ADMIN, p::TENANT_WRITE, p::SECURITY_WRITE] {
        assert!(!has_permission("manager", perm), "manager 不应具备 {perm}");
    }
}

#[test]
fn unknown_role_is_fail_closed() {
    for perm in all_permissions() {
        assert!(!has_permission("superuser", perm), "未知角色应拒绝 {perm}");
        assert!(!has_permission("", perm), "空角色应拒绝 {perm}");
    }
}

#[test]
fn permission_strings_are_verbatim() {
    // 双端 diff 基线：任一漂移都会让这里失败。
    assert_eq!(p::CLUSTER_READ, "cluster:read");
    assert_eq!(p::CLUSTER_WRITE, "cluster:write");
    assert_eq!(p::RESOURCE_READ, "resource:read");
    assert_eq!(p::RESOURCE_WRITE, "resource:write");
    assert_eq!(p::JOB_READ, "job:read");
    assert_eq!(p::JOB_WRITE, "job:write");
    assert_eq!(p::JOB_SUBMIT, "job:submit");
    assert_eq!(p::TENANT_READ, "tenant:read");
    assert_eq!(p::TENANT_WRITE, "tenant:write");
    assert_eq!(p::MONITORING_READ, "monitoring:read");
    assert_eq!(p::MONITORING_WRITE, "monitoring:write");
    assert_eq!(p::ACCELERATION_READ, "acceleration:read");
    assert_eq!(p::ACCELERATION_WRITE, "acceleration:write");
    assert_eq!(p::SECURITY_READ, "security:read");
    assert_eq!(p::SECURITY_WRITE, "security:write");
    assert_eq!(p::ADMIN, "admin");
    assert_eq!(p::GPU_READ, "gpu:read");
    assert_eq!(p::GPU_WRITE, "gpu:write");
    assert_eq!(p::PARTITION_READ, "partition:read");
    assert_eq!(p::PARTITION_WRITE, "partition:write");
    assert_eq!(p::QUOTA_READ, "quota:read");
    assert_eq!(p::QUOTA_WRITE, "quota:write");
    assert_eq!(p::SCHEDULER_READ, "scheduler:read");
    assert_eq!(p::SCHEDULER_WRITE, "scheduler:write");
    assert_eq!(p::TOPOLOGY_READ, "topology:read");
    assert_eq!(p::TOPOLOGY_WRITE, "topology:write");
    assert_eq!(p::DATASET_READ, "dataset:read");
    assert_eq!(p::DATASET_WRITE, "dataset:write");
    assert_eq!(p::CHECKPOINT_READ, "checkpoint:read");
    assert_eq!(p::CHECKPOINT_WRITE, "checkpoint:write");
}
