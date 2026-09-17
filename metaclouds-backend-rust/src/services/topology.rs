//! 节点拓扑服务（对齐 Go `services/topology_service.go` 的 Node CRUD 面）。
//!
//! CRUD + 分页 + 按 cluster_id / role 过滤。handler 只做参数提取与响应封装。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::topology::{self, NewNode, NodeTopology, TopologyResponse};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建节点入参。
#[derive(Debug, Clone)]
pub struct CreateNodeInput {
    pub cluster_id: i64,
    pub hostname: String,
    pub ip: String,
    pub role: String,
    pub cpu_cores: i64,
    pub memory_gb: i64,
    pub gpu_count: i64,
    pub gpu_model: String,
    pub status: String,
    pub labels: serde_json::Value,
}

/// 更新节点入参：全部可选。
#[derive(Debug, Clone, Default)]
pub struct UpdateNodeInput {
    pub hostname: Option<String>,
    pub ip: Option<String>,
    pub role: Option<String>,
    pub cpu_cores: Option<i64>,
    pub memory_gb: Option<i64>,
    pub gpu_count: Option<i64>,
    pub gpu_model: Option<String>,
    pub status: Option<String>,
    pub labels: Option<serde_json::Value>,
}

/// 创建节点。
pub async fn create_node(pool: &SqlitePool, input: CreateNodeInput) -> AppResult<TopologyResponse> {
    // hostname 在集群内唯一（对齐 Go CreateNodeTopology 的重名冲突）。
    let dup: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM topology_nodes WHERE hostname = ?1 AND cluster_id = ?2 AND deleted_at IS NULL LIMIT 1",
    )
    .bind(&input.hostname)
    .bind(input.cluster_id)
    .fetch_optional(pool)
    .await?;
    if dup.is_some() {
        return Err(AppError::conflict("node already exists for this host"));
    }

    let node = topology::create(
        pool,
        NewNode {
            cluster_id: input.cluster_id,
            hostname: &input.hostname,
            ip: &input.ip,
            role: &input.role,
            cpu_cores: input.cpu_cores,
            memory_gb: input.memory_gb,
            gpu_count: input.gpu_count,
            gpu_model: &input.gpu_model,
            status: &input.status,
            labels: input.labels,
        },
    )
    .await?;
    Ok(node.into())
}

/// 节点详情（404 若不存在或已软删除）。
pub async fn get_node(pool: &SqlitePool, id: i64) -> AppResult<TopologyResponse> {
    let node = topology::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("node topology not found"))?;
    Ok(node.into())
}

/// 分页节点列表（可按 cluster_id / role 过滤）。
pub async fn list_nodes(
    pool: &SqlitePool,
    params: PaginationParams,
    cluster_id: Option<i64>,
    role: Option<&str>,
) -> AppResult<PaginatedResult<TopologyResponse>> {
    let rows: PaginatedResult<NodeTopology> =
        topology::list(pool, params, cluster_id, role).await?;
    Ok(PaginatedResult {
        data: rows.data.into_iter().map(TopologyResponse::from).collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 更新节点：仅覆盖传入字段，自动刷 updated_at。
pub async fn update_node(
    pool: &SqlitePool,
    id: i64,
    input: UpdateNodeInput,
) -> AppResult<TopologyResponse> {
    topology::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("node topology not found"))?;

    // labels 用 JSON 字符串绑定。
    let labels_json = input
        .labels
        .as_ref()
        .map(|v| serde_json::to_string(v).unwrap_or_else(|_| "{}".to_string()));

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE topology_nodes SET \
            hostname = COALESCE(?1, hostname), \
            ip = COALESCE(?2, ip), \
            role = COALESCE(?3, role), \
            cpu_cores = COALESCE(?4, cpu_cores), \
            memory_gb = COALESCE(?5, memory_gb), \
            gpu_count = COALESCE(?6, gpu_count), \
            gpu_model = COALESCE(?7, gpu_model), \
            status = COALESCE(?8, status), \
            labels = COALESCE(?9, labels), \
            updated_at = ?10 \
         WHERE id = ?11 AND deleted_at IS NULL",
    )
    .bind(&input.hostname)
    .bind(&input.ip)
    .bind(&input.role)
    .bind(input.cpu_cores)
    .bind(input.memory_gb)
    .bind(input.gpu_count)
    .bind(&input.gpu_model)
    .bind(&input.status)
    .bind(labels_json)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let node = topology::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("node disappeared after update"))?;
    Ok(node.into())
}

/// 软删除节点（404 若不存在或已软删除）。
pub async fn delete_node(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = topology::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("node topology not found"));
    }
    Ok(())
}
