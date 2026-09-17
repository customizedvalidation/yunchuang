//! 集群服务（对齐 Go `services/cluster_service.go`）。
//!
//! CRUD + 分页 + name 搜索 + 软删除 + 集群资源状态统计。
//! handler 只做参数提取与响应封装。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::cluster::{self, Cluster, ClusterResponse};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建集群入参（对齐 Go `CreateClusterRequest`）。
#[derive(Debug, Clone)]
pub struct CreateClusterInput {
    pub name: String,
    pub description: String,
    pub nodes: i64,
    pub gpus: i64,
    pub cpus: i64,
    pub memory: i64,
    pub storage: i64,
    pub network_type: String,
    pub location: String,
}

/// 更新集群入参：全部可选。
#[derive(Debug, Clone, Default)]
pub struct UpdateClusterInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub nodes: Option<i64>,
    pub gpus: Option<i64>,
    pub cpus: Option<i64>,
    pub memory: Option<i64>,
    pub storage: Option<i64>,
    pub network_type: Option<String>,
    pub location: Option<String>,
}

/// 集群资源状态统计（对齐 Go `K8SService.GetClusterStatus` 的 GPU 聚合口径）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ClusterResourceStats {
    pub cluster_id: i64,
    pub total_resources: i64,
    pub total_gpu: i64,
    pub used_gpu: i64,
    pub available_gpu: i64,
}

/// 按名称检查是否已存在未删除的集群。
async fn name_taken(pool: &SqlitePool, name: &str, except_id: i64) -> AppResult<bool> {
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM clusters WHERE name = ?1 AND deleted_at IS NULL AND id != ?2 LIMIT 1",
    )
    .bind(name)
    .bind(except_id)
    .fetch_optional(pool)
    .await?;
    Ok(exists.is_some())
}

/// 创建集群（status 固定 active，对齐 Go `CreateCluster`）。
pub async fn create_cluster(
    pool: &SqlitePool,
    input: CreateClusterInput,
) -> AppResult<ClusterResponse> {
    if name_taken(pool, &input.name, 0).await? {
        return Err(AppError::conflict("cluster name already exists"));
    }
    let c = cluster::create(
        pool,
        cluster::NewCluster {
            name: &input.name,
            description: &input.description,
            status: "active",
            nodes: input.nodes,
            gpus: input.gpus,
            cpus: input.cpus,
            memory: input.memory,
            storage: input.storage,
            network_type: &input.network_type,
            location: &input.location,
            gpu_vendors: Vec::new(),
            scheduler_types: Vec::new(),
            multi_cluster_enabled: false,
            federation_id: "",
        },
    )
    .await?;
    Ok(c.into())
}

/// 集群详情（404 若不存在或已软删除）。
pub async fn get_cluster(pool: &SqlitePool, id: i64) -> AppResult<ClusterResponse> {
    let c = cluster::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("cluster not found"))?;
    Ok(c.into())
}

/// 分页集群列表（可按 name 模糊搜索；过滤软删除）。
pub async fn list_clusters(
    pool: &SqlitePool,
    params: PaginationParams,
    search: Option<&str>,
) -> AppResult<PaginatedResult<ClusterResponse>> {
    let params = params.normalize();

    let (where_clause, has_search) = match search {
        Some(s) if !s.is_empty() => (
            "deleted_at IS NULL AND name LIKE ?".to_string(),
            Some(format!("%{s}%")),
        ),
        _ => ("deleted_at IS NULL".to_string(), None),
    };

    let total: i64 = {
        let q = format!("SELECT COUNT(*) FROM clusters WHERE {where_clause}");
        let mut sc = sqlx::query_scalar::<_, i64>(&q);
        if let Some(ref l) = has_search {
            sc = sc.bind(l);
        }
        sc.fetch_one(pool).await?
    };

    let rows: Vec<Cluster> = {
        let q =
            format!("SELECT * FROM clusters WHERE {where_clause} ORDER BY id ASC LIMIT ? OFFSET ?");
        let mut sq = sqlx::query_as::<_, Cluster>(&q);
        if let Some(ref l) = has_search {
            sq = sq.bind(l);
        }
        sq = sq.bind(params.limit()).bind(params.offset());
        sq.fetch_all(pool).await?
    };

    Ok(PaginatedResult {
        data: rows.into_iter().map(ClusterResponse::from).collect(),
        total,
        page: params.page,
        page_size: params.page_size,
        total_pages: crate::orm::total_pages(total, params.page_size),
    })
}

/// 更新集群：仅覆盖传入字段；重名校验；自动刷 updated_at。
pub async fn update_cluster(
    pool: &SqlitePool,
    id: i64,
    input: UpdateClusterInput,
) -> AppResult<ClusterResponse> {
    cluster::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("cluster not found"))?;

    if let Some(name) = input.name.as_deref() {
        if name_taken(pool, name, id).await? {
            return Err(AppError::conflict("cluster name already exists"));
        }
    }

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE clusters SET \
            name = COALESCE(?1, name), \
            description = COALESCE(?2, description), \
            status = COALESCE(?3, status), \
            nodes = COALESCE(?4, nodes), \
            gpus = COALESCE(?5, gpus), \
            cpus = COALESCE(?6, cpus), \
            memory = COALESCE(?7, memory), \
            storage = COALESCE(?8, storage), \
            network_type = COALESCE(?9, network_type), \
            location = COALESCE(?10, location), \
            updated_at = ?11 \
         WHERE id = ?12 AND deleted_at IS NULL",
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.status)
    .bind(input.nodes)
    .bind(input.gpus)
    .bind(input.cpus)
    .bind(input.memory)
    .bind(input.storage)
    .bind(&input.network_type)
    .bind(&input.location)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let c = cluster::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("cluster disappeared after update"))?;
    Ok(c.into())
}

/// 软删除集群（404 若不存在或已软删除）。
pub async fn delete_cluster(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = cluster::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("cluster not found"));
    }
    Ok(())
}

/// 集群资源状态统计（按 GPU 资源聚合，对齐 Go GetClusterStatus 口径）。
pub async fn cluster_stats(pool: &SqlitePool, id: i64) -> AppResult<ClusterResourceStats> {
    // 404 早判。
    cluster::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("cluster not found"))?;

    let total_resources: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resources WHERE cluster_id = ?1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    let (total_gpu, used_gpu): (Option<i64>, Option<i64>) = sqlx::query_as(
        "SELECT COALESCE(SUM(total), 0), COALESCE(SUM(used), 0) \
         FROM resources WHERE cluster_id = ?1 AND type = 'gpu' AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;

    let total_gpu = total_gpu.unwrap_or(0);
    let used_gpu = used_gpu.unwrap_or(0);
    Ok(ClusterResourceStats {
        cluster_id: id,
        total_resources,
        total_gpu,
        used_gpu,
        available_gpu: total_gpu - used_gpu,
    })
}
