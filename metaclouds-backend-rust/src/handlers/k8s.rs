//! HTTP handlers for `/api/v1/k8s/clusters/{id}/*`（对齐 Go `controllers/k8s_controller.go` 的只读面）。
//!
//! 本机无 Kubernetes，全部走 [`crate::services::k8s::MockK8sClient`]。
//! 路由上均需 JWT + `cluster:read`。返回结构字段名对齐 Go 版。

use axum::extract::{Path, State};
use axum::Json;

use crate::auth::middleware::AppState;
use crate::error::AppResult;
use crate::response::ApiResponse;
use crate::services::k8s::{K8sClient, MockK8sClient};

/// `GET /api/v1/k8s/clusters/:id/pods` — 列出 Pod（mock）。
#[utoipa::path(get,path="/api/v1/k8s/clusters/{id}/pods",tag="k8s",responses((status=200,description="pods",body=Vec<crate::services::k8s::Pod>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_pods(
    State(_state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Vec<crate::services::k8s::Pod>>>> {
    let client = MockK8sClient::new();
    let pods = client.list_pods(id).await?;
    Ok(Json(ApiResponse::success(pods)))
}

/// `GET /api/v1/k8s/clusters/:id/nodes` — 列出节点（mock）。
#[utoipa::path(get,path="/api/v1/k8s/clusters/{id}/nodes",tag="k8s",responses((status=200,description="nodes",body=Vec<crate::services::k8s::K8sNode>),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn list_nodes(
    State(_state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<Vec<crate::services::k8s::K8sNode>>>> {
    let client = MockK8sClient::new();
    let nodes = client.list_nodes(id).await?;
    Ok(Json(ApiResponse::success(nodes)))
}

/// `GET /api/v1/k8s/clusters/:id/health` — 集群健康（mock，字段对齐 Go ClusterStatus）。
#[utoipa::path(get,path="/api/v1/k8s/clusters/{id}/health",tag="k8s",responses((status=200,description="cluster health",body=crate::services::k8s::ClusterHealth),(status=401,description="unauthorized",body=crate::openapi::ErrorResponse),(status=403,description="forbidden",body=crate::openapi::ErrorResponse),(status=404,description="not found",body=crate::openapi::ErrorResponse)),security(("bearer_auth"=[])))]
pub async fn cluster_health(
    State(_state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ApiResponse<crate::services::k8s::ClusterHealth>>> {
    let client = MockK8sClient::new();
    let health = client.get_cluster_health(id).await?;
    Ok(Json(ApiResponse::success(health)))
}
