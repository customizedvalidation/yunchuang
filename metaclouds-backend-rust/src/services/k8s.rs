//! K8s 只读集成服务（对齐 Go `services/k8s_service.go` 的只读面）。
//!
//! 本机无 Kubernetes 集群，故用 trait 抽象出 [`K8sClient`]，提供 [`MockK8sClient`]
//! 返回确定性的模拟数据。真实 kube-rs 集成留待 Phase 3 / 有 K8s 环境时接入，
//! 本工作包**不引入 kube-rs 依赖**（也不新增任何 crate）。
//!
//! 返回结构字段名对齐 Go：
//! - [`Pod`]：name / status / node / gpus（Go `Pod`）；
//! - [`ClusterHealth`]：id / name / status / nodes / gpus_total / gpus_used /
//!   gpus_free / cpu_usage / memory_usage（Go `ClusterStatus`）。

use crate::error::{AppError, AppResult};

/// K8s Pod（对齐 Go `services.Pod`：name/status/node/gpus）。
#[derive(utoipa::ToSchema, Debug, Clone, serde::Serialize, PartialEq)]
pub struct Pod {
    pub name: String,
    pub status: String,
    pub node: String,
    pub gpus: i64,
}

/// K8s 节点（只读观测面）。
#[derive(utoipa::ToSchema, Debug, Clone, serde::Serialize, PartialEq)]
pub struct K8sNode {
    pub name: String,
    pub status: String,
    pub role: String,
    pub cpu_cores: i64,
    pub memory_gb: i64,
}

/// K8s Deployment（只读观测面）。
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct Deployment {
    pub name: String,
    pub status: String,
    pub replicas: i64,
    pub ready: i64,
}

/// 集群健康状态（对齐 Go `ClusterStatus` 字段名）。
#[derive(utoipa::ToSchema, Debug, Clone, serde::Serialize, PartialEq)]
pub struct ClusterHealth {
    pub id: i64,
    pub name: String,
    pub status: String,
    pub nodes: i64,
    pub gpus_total: i64,
    pub gpus_used: i64,
    pub gpus_free: i64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
}

/// K8s 只读客户端抽象。真实实现（kube-rs）后续接入；当前由 [`MockK8sClient`] 实现。
///
/// 使用原生 `async fn in trait`（Rust 1.75+ 即 dyn-compatible），无需 async-trait crate。
/// 本 trait 仅在本 crate 内部使用（非 dyn 对象边界），故允许 `async_fn_in_trait` lint。
#[allow(async_fn_in_trait)]
pub trait K8sClient: Send + Sync {
    /// 列出某集群的 Pod。
    async fn list_pods(&self, cluster_id: i64) -> AppResult<Vec<Pod>>;
    /// 列出某集群的节点。
    async fn list_nodes(&self, cluster_id: i64) -> AppResult<Vec<K8sNode>>;
    /// 集群健康状态（对齐 Go GetClusterStatus）。
    async fn get_cluster_health(&self, cluster_id: i64) -> AppResult<ClusterHealth>;
    /// 列出某集群的 Deployment。
    async fn list_deployments(&self, cluster_id: i64) -> AppResult<Vec<Deployment>>;
}

/// 确定性 mock K8s 客户端：本机无 K8s，返回固定结构的模拟数据。
///
/// 字段形状与命名严格对齐 Go 版，便于 Golden L2 对比；数据内容为占位。
#[derive(Debug, Clone, Default)]
pub struct MockK8sClient;

impl MockK8sClient {
    pub fn new() -> Self {
        Self
    }
}

impl K8sClient for MockK8sClient {
    async fn list_pods(&self, cluster_id: i64) -> AppResult<Vec<Pod>> {
        Ok(vec![
            Pod {
                name: format!("job-pod-{cluster_id}-0"),
                status: "Running".to_string(),
                node: "gpu-node-1".to_string(),
                gpus: 1,
            },
            Pod {
                name: format!("job-pod-{cluster_id}-1"),
                status: "Running".to_string(),
                node: "gpu-node-2".to_string(),
                gpus: 1,
            },
        ])
    }

    async fn list_nodes(&self, cluster_id: i64) -> AppResult<Vec<K8sNode>> {
        Ok(vec![
            K8sNode {
                name: "gpu-node-1".to_string(),
                status: "Ready".to_string(),
                role: "worker".to_string(),
                cpu_cores: 64,
                memory_gb: 512,
            },
            K8sNode {
                name: "gpu-node-2".to_string(),
                status: "Ready".to_string(),
                role: "worker".to_string(),
                cpu_cores: 64,
                memory_gb: 512,
            },
            K8sNode {
                name: format!("master-node-{cluster_id}"),
                status: "Ready".to_string(),
                role: "master".to_string(),
                cpu_cores: 32,
                memory_gb: 256,
            },
        ])
    }

    async fn get_cluster_health(&self, cluster_id: i64) -> AppResult<ClusterHealth> {
        if cluster_id <= 0 {
            return Err(AppError::not_found("cluster not found"));
        }
        // 对齐 Go GetClusterStatus 的聚合口径（mock 数据）。
        Ok(ClusterHealth {
            id: cluster_id,
            name: format!("cluster-{cluster_id}"),
            status: "healthy".to_string(),
            nodes: 3,
            gpus_total: 8,
            gpus_used: 2,
            gpus_free: 6,
            cpu_usage: 32.0,
            memory_usage: 48.0,
        })
    }

    async fn list_deployments(&self, cluster_id: i64) -> AppResult<Vec<Deployment>> {
        Ok(vec![
            Deployment {
                name: format!("training-stack-{cluster_id}"),
                status: "Available".to_string(),
                replicas: 2,
                ready: 2,
            },
            Deployment {
                name: "monitoring".to_string(),
                status: "Available".to_string(),
                replicas: 1,
                ready: 1,
            },
        ])
    }
}
