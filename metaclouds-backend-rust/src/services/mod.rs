//! 领域服务层（WP-P2-B1 / B2）。
//!
//! 把业务规则（认证 / 租户 / 资源 / 集群 / 拓扑 / K8s mock）下沉到本层。
//! Handler 只做 HTTP 收发，不再直接写 SQL（K8s 只读面经 [`crate::services::k8s`]
//! 的 trait 抽象，本机用 mock 实现）。

pub mod acceleration;
pub mod alert;
pub mod auth;
pub mod checkpoint;
pub mod cluster;
pub mod dataset;
pub mod distributed_training;
pub mod fluid_cache;
pub mod gpu;
pub mod inference;
pub mod job;
pub mod k8s;
pub mod monitoring;
pub mod partition;
pub mod partition_permission;
pub mod quota;
pub mod resource;
pub mod scheduler;
pub mod security;
pub mod tenant;
pub mod topology;
