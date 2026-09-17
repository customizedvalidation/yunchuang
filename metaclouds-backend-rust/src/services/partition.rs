//! 分区服务（对齐 WP-P2-B4 调度域规格）。
//!
//! CRUD + 分页 + name 搜索 + 按 cluster_id/status/partition_type 过滤 + 软删除 +
//! 分区资源使用情况统计。handler 只做参数提取与响应封装。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::partition::{self, NewPartition, Partition, PartitionResponse};
use crate::orm::{PaginatedResult, PaginationParams};
use serde_json::Value;

/// 创建分区入参。
#[derive(Debug, Clone)]
pub struct CreatePartitionInput {
    pub cluster_id: i64,
    pub name: String,
    pub description: String,
    pub partition_type: String,
    pub gpu_count: i64,
    pub cpu_cores: f64,
    pub memory_gb: f64,
    pub status: Option<String>,
    pub node_selector: Value,
    pub labels: Value,
    pub tenant_id: Option<i64>,
}

/// 更新分区入参：全部可选。
#[derive(Debug, Clone, Default)]
pub struct UpdatePartitionInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub partition_type: Option<String>,
    pub gpu_count: Option<i64>,
    pub cpu_cores: Option<f64>,
    pub memory_gb: Option<f64>,
    pub status: Option<String>,
    pub node_selector: Option<Value>,
    pub labels: Option<Value>,
    pub tenant_id: Option<Option<i64>>,
}

/// 分区资源使用情况（对齐 get_partition_resources）。
#[derive(utoipa::ToSchema, Debug, Clone, serde::Serialize)]
pub struct PartitionResources {
    pub partition_id: i64,
    pub gpu_count: i64,
    pub gpu_used: i64,
    pub gpu_available: i64,
    pub cpu_cores: f64,
    pub cpu_used: f64,
    pub memory_gb: f64,
    pub memory_used_gb: f64,
}

/// 按集群 + 名称检查是否已存在未删除的分区。
async fn name_taken(
    pool: &SqlitePool,
    cluster_id: i64,
    name: &str,
    except_id: i64,
) -> AppResult<bool> {
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM partitions WHERE cluster_id = ?1 AND name = ?2 AND deleted_at IS NULL AND id != ?3 LIMIT 1",
    )
    .bind(cluster_id)
    .bind(name)
    .bind(except_id)
    .fetch_optional(pool)
    .await?;
    Ok(exists.is_some())
}

/// 创建分区（status 默认 active，对齐 Go CreatePartition）。
pub async fn create_partition(
    pool: &SqlitePool,
    input: CreatePartitionInput,
) -> AppResult<PartitionResponse> {
    if name_taken(pool, input.cluster_id, &input.name, 0).await? {
        return Err(AppError::conflict(
            "partition name already exists in this cluster",
        ));
    }
    let status = input.status.unwrap_or_else(|| "active".to_string());
    let p = partition::create(
        pool,
        NewPartition {
            cluster_id: input.cluster_id,
            name: input.name,
            description: input.description,
            partition_type: input.partition_type,
            gpu_count: input.gpu_count,
            cpu_cores: input.cpu_cores,
            memory_gb: input.memory_gb,
            status,
            node_selector: input.node_selector,
            labels: input.labels,
            tenant_id: input.tenant_id,
        },
    )
    .await?;
    Ok(p.into())
}

/// 分区详情（404 若不存在或已软删除）。
pub async fn get_partition(pool: &SqlitePool, id: i64) -> AppResult<PartitionResponse> {
    let p = partition::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("partition not found"))?;
    Ok(p.into())
}

/// 分页分区列表（name 模糊搜索 + 按 cluster_id/status/partition_type 过滤；排除软删除）。
#[allow(clippy::too_many_arguments)]
pub async fn list_partitions(
    pool: &SqlitePool,
    params: PaginationParams,
    cluster_id: Option<i64>,
    status: Option<&str>,
    partition_type: Option<&str>,
    search: Option<&str>,
) -> AppResult<PaginatedResult<PartitionResponse>> {
    let res = partition::list(pool, params, cluster_id, status, partition_type, search).await?;
    Ok(PaginatedResult {
        data: res.data.into_iter().map(PartitionResponse::from).collect(),
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })
}

/// 更新分区：仅覆盖传入字段；重名校验；自动刷 updated_at。
pub async fn update_partition(
    pool: &SqlitePool,
    id: i64,
    input: UpdatePartitionInput,
) -> AppResult<PartitionResponse> {
    let existing = partition::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("partition not found"))?;

    if let Some(name) = input.name.as_deref() {
        if name_taken(pool, existing.cluster_id, name, id).await? {
            return Err(AppError::conflict(
                "partition name already exists in this cluster",
            ));
        }
    }

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE partitions SET \
            name = COALESCE(?1, name), \
            description = COALESCE(?2, description), \
            partition_type = COALESCE(?3, partition_type), \
            gpu_count = COALESCE(?4, gpu_count), \
            cpu_cores = COALESCE(?5, cpu_cores), \
            memory_gb = COALESCE(?6, memory_gb), \
            status = COALESCE(?7, status), \
            node_selector = COALESCE(?8, node_selector), \
            labels = COALESCE(?9, labels), \
            tenant_id = COALESCE(?10, tenant_id), \
            updated_at = ?11 \
         WHERE id = ?12 AND deleted_at IS NULL",
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.partition_type)
    .bind(input.gpu_count)
    .bind(input.cpu_cores)
    .bind(input.memory_gb)
    .bind(&input.status)
    .bind(input.node_selector.map(crate::orm::Json))
    .bind(input.labels.map(crate::orm::Json))
    .bind(input.tenant_id.flatten())
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let p = partition::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("partition disappeared after update"))?;
    Ok(p.into())
}

/// 软删除分区（404 若不存在或已软删除）。
pub async fn delete_partition(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = partition::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("partition not found"));
    }
    // 级联物理删除分区授权（对齐 Go DeletePartition 级联）。
    sqlx::query("DELETE FROM partition_permissions WHERE partition_id = ?1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

/// 分区资源使用情况：分区声明总量 + 按 gpu_allocations 聚合已用 GPU。
pub async fn get_partition_resources(pool: &SqlitePool, id: i64) -> AppResult<PartitionResources> {
    let p: Partition = partition::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("partition not found"))?;

    // 已用 GPU：从 gpu_allocations 按 partition 关联（jobs.partition_id）聚合 active 分配。
    // 若无 gpu_allocations/jobs 表数据，聚合为 0（SQLite 跨表 JOIN 安全）。
    let gpu_used: Option<i64> = sqlx::query_scalar(
        "SELECT COALESCE(SUM(ga.fraction), 0) \
         FROM gpu_allocations ga \
         JOIN jobs j ON j.id = ga.job_id \
         WHERE j.partition_id = ?1 AND ga.status = 'active' AND ga.deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;
    let gpu_used = gpu_used.unwrap_or(0);

    Ok(PartitionResources {
        partition_id: id,
        gpu_count: p.gpu_count,
        gpu_used,
        gpu_available: (p.gpu_count - gpu_used).max(0),
        cpu_cores: p.cpu_cores,
        cpu_used: 0.0,
        memory_gb: p.memory_gb,
        memory_used_gb: 0.0,
    })
}
