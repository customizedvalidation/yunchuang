//! HTTP handlers for `/api/v1/k8s/clusters/{id}/*`（对齐 Go `controllers/k8s_controller.go` 的只读面）。
//!
//! 本机无 Kubernetes，数据源为 [`crate::services::k8s::MockK8sClient`]。
//! 路由上均需 JWT + `cluster:read`。返回结构字段名对齐 Go 版。
//!
//! **P0-1 / P1-3（2026-11-09）**：`K8S_SIMULATION_MODE` 此前被读取但从未被业务消费，
//! 生产 ConfigMap 设成 `false` 却依旧返回 mock 数据，造成「已接入真实集群」的错觉。
//! 现在该开关具有真实语义：
//! - `true`：明确返回 mock 数据（前端 `/system/mode` 会同步展示演示模式横幅）；
//! - `false`：在 kube-rs 真实客户端接入前 **fail-closed**，返回错误而不是伪造数据。

use axum::extract::{Path, State};
use axum::Json;

use crate::auth::middleware::AppState;
use crate::error::{AppError, AppResult};
use crate::response::ApiResponse;
use crate::services::k8s::{K8sClient, MockK8sClient};

/// 真实集群客户端尚未接线时，禁止用 mock 数据冒充真实集群。
fn ensure_mock_allowed(state: &AppState) -> AppResult<()> {
    if !state.config.k8s_simulation_mode {
        tracing::warn!(
            "K8S_SIMULATION_MODE=false: rejecting K8s read request because no real client is wired"
        );
        return Err(AppError::internal(
            "real Kubernetes client is not wired; mock data is disabled by K8S_SIMULATION_MODE=false",
        ));
    }
    Ok(())
}

/// `GET /api/v1/k8s/clusters/:id/pods` — 列出 Pod（mock）。
#[utoipa::path(get,path="/api/v1/k8s/clusters/{id}/pods",tag="k8s",responses((status=200,description="pods",body=Vec<crate::services::k8s::Pod>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_pods(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Vec<crate::services::k8s::Pod>>>> {
    ensure_mock_allowed(&state)?;
    let client = MockK8sClient::new();
    let pods = client.list_pods(id).await?;
    Ok(Json(ApiResponse::success(pods)))
}

/// `GET /api/v1/k8s/clusters/:id/nodes` — 列出节点（mock）。
#[utoipa::path(get,path="/api/v1/k8s/clusters/{id}/nodes",tag="k8s",responses((status=200,description="nodes",body=Vec<crate::services::k8s::K8sNode>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_nodes(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Vec<crate::services::k8s::K8sNode>>>> {
    ensure_mock_allowed(&state)?;
    let client = MockK8sClient::new();
    let nodes = client.list_nodes(id).await?;
    Ok(Json(ApiResponse::success(nodes)))
}

/// `GET /api/v1/k8s/clusters/:id/health` — 集群健康（mock，字段对齐 Go ClusterStatus）。
#[utoipa::path(get,path="/api/v1/k8s/clusters/{id}/health",tag="k8s",responses((status=200,description="cluster health",body=crate::services::k8s::ClusterHealth),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn cluster_health(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<crate::services::k8s::ClusterHealth>>> {
    ensure_mock_allowed(&state)?;
    let client = MockK8sClient::new();
    let health = client.get_cluster_health(id).await?;
    Ok(Json(ApiResponse::success(health)))
}

/// `GET /api/v1/clusters/:id/status` — 集群状态（对齐 Go `k8sController.GetClusterStatus`，
/// 返回 Go `ClusterStatus`：gpus_total/gpus_used/gpus_free/cpu_usage/memory_usage）。
///
/// 复用 MockK8sClient 的聚合口径；Go 版该路由挂在 `/clusters/:id/status`（JWT，无额外权限）。
pub async fn cluster_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<crate::services::k8s::ClusterHealth>>> {
    ensure_mock_allowed(&state)?;
    let client = MockK8sClient::new();
    let status = client.get_cluster_health(id).await?;
    Ok(Json(ApiResponse::success(status)))
}
