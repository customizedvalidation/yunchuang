//! 调度器集成服务（对齐 WP-P2-B4 调度域规格）。
//!
//! CRUD + 分页 + 按 cluster_id/status/scheduler_type 过滤 + 软删除 +
//! test_connection（mock，返回 status + latency_ms）+ sync_resources（mock）。
//!
//! 本机无真实调度器，故用 trait [`SchedulerAdapter`] 抽象出对外部调度器的探测与同步，
//! 提供 [`MockSchedulerAdapter`] 返回确定性模拟结果（对齐 Go `services/scheduler_adapter.go`
//! 的 Slurm/K8sNative/Generic mock 模式，本工作包不引入真实调度器依赖）。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::scheduler_integration::{
    self, scheduler_status, NewScheduler, SchedulerIntegrationResponse,
};
use crate::orm::{PaginatedResult, PaginationParams};
use serde_json::Value;

// ---------------------------------------------------------------------------
// SchedulerAdapter trait + Mock（对齐 Go scheduler_adapter.go 模式）
// ---------------------------------------------------------------------------

/// 连接测试结果。
#[derive(utoipa::ToSchema, Debug, Clone, serde::Serialize)]
pub struct ConnectionTest {
    pub status: String,
    pub latency_ms: i64,
}

/// 资源同步结果（mock）。
#[derive(utoipa::ToSchema, Debug, Clone, serde::Serialize)]
pub struct ResourceSync {
    pub total_nodes: i64,
    pub total_gpus: i64,
    pub pending_jobs: i64,
    pub running_jobs: i64,
    pub synced_at: String,
}

/// 外部调度器适配器抽象（真实实现后续接入；当前由 [`MockSchedulerAdapter`] 实现）。
///
/// 使用原生 `async fn in trait`（Rust 1.75+ 即 dyn-compatible），无需 async-trait crate。
#[allow(async_fn_in_trait)]
pub trait SchedulerAdapter: Send + Sync {
    /// 测试连接：返回连接状态与探测延迟。
    async fn test_connection(&self) -> AppResult<ConnectionTest>;
    /// 同步资源状态：返回从调度器观测到的资源快照。
    async fn sync_resources(&self) -> AppResult<ResourceSync>;
}

/// 确定性 mock 调度器适配器：本机无真实调度器，返回固定结构的模拟数据。
#[derive(Debug, Clone, Default)]
pub struct MockSchedulerAdapter {
    scheduler_type: String,
    cluster_id: i64,
}

impl MockSchedulerAdapter {
    pub fn new(scheduler_type: impl Into<String>, cluster_id: i64) -> Self {
        Self {
            scheduler_type: scheduler_type.into(),
            cluster_id,
        }
    }
}

impl SchedulerAdapter for MockSchedulerAdapter {
    async fn test_connection(&self) -> AppResult<ConnectionTest> {
        // mock：固定低延迟 + connected（kubernetes/slurm）/ disconnected（custom）。
        let status = match self.scheduler_type.as_str() {
            "custom" => "disconnected",
            _ => "connected",
        };
        Ok(ConnectionTest {
            status: status.to_string(),
            latency_ms: 12,
        })
    }

    async fn sync_resources(&self) -> AppResult<ResourceSync> {
        Ok(ResourceSync {
            total_nodes: self.cluster_id.max(1) * 2,
            total_gpus: self.cluster_id.max(1) * 16,
            pending_jobs: 3,
            running_jobs: 12,
            synced_at: chrono::Utc::now().to_rfc3339(),
        })
    }
}

// ---------------------------------------------------------------------------
// CRUD 服务
// ---------------------------------------------------------------------------

/// 创建调度器集成入参。
#[derive(Debug, Clone)]
pub struct CreateSchedulerInput {
    pub name: String,
    pub description: String,
    pub scheduler_type: String,
    pub endpoint: String,
    pub auth_type: String,
    pub credentials: Value,
    pub status: Option<String>,
    pub cluster_id: i64,
    pub version: String,
    pub config: Value,
}

/// 更新调度器集成入参：全部可选。
#[derive(Debug, Clone, Default)]
pub struct UpdateSchedulerInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub scheduler_type: Option<String>,
    pub endpoint: Option<String>,
    pub auth_type: Option<String>,
    pub credentials: Option<Value>,
    pub status: Option<String>,
    pub version: Option<String>,
    pub config: Option<Value>,
}

/// 按名称检查是否已存在未删除的调度器集成。
async fn name_taken(pool: &SqlitePool, name: &str, except_id: i64) -> AppResult<bool> {
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM scheduler_integrations WHERE name = ?1 AND deleted_at IS NULL AND id != ?2 LIMIT 1",
    )
    .bind(name)
    .bind(except_id)
    .fetch_optional(pool)
    .await?;
    Ok(exists.is_some())
}

/// 创建调度器集成（status 默认 disconnected，auth_type 默认 none）。
pub async fn create_scheduler(
    pool: &SqlitePool,
    input: CreateSchedulerInput,
) -> AppResult<SchedulerIntegrationResponse> {
    if name_taken(pool, &input.name, 0).await? {
        return Err(AppError::conflict(
            "scheduler integration name already exists",
        ));
    }
    let status = input
        .status
        .unwrap_or_else(|| scheduler_status::DISCONNECTED.to_string());
    let auth_type = if input.auth_type.is_empty() {
        "none".to_string()
    } else {
        input.auth_type
    };
    let s = scheduler_integration::create(
        pool,
        NewScheduler {
            name: &input.name,
            description: &input.description,
            scheduler_type: &input.scheduler_type,
            endpoint: &input.endpoint,
            auth_type: &auth_type,
            credentials: input.credentials,
            status: &status,
            cluster_id: input.cluster_id,
            version: &input.version,
            config: input.config,
        },
    )
    .await?;
    Ok(s.into())
}

/// 调度器集成详情（404 若不存在或已软删除）。
pub async fn get_scheduler(pool: &SqlitePool, id: i64) -> AppResult<SchedulerIntegrationResponse> {
    let s = scheduler_integration::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("scheduler integration not found"))?;
    Ok(s.into())
}

/// 分页调度器列表（按 cluster_id/status/scheduler_type 过滤；排除软删除）。
pub async fn list_schedulers(
    pool: &SqlitePool,
    params: PaginationParams,
    cluster_id: Option<i64>,
    status: Option<&str>,
    scheduler_type: Option<&str>,
) -> AppResult<PaginatedResult<SchedulerIntegrationResponse>> {
    let res = scheduler_integration::list(pool, params, cluster_id, status, scheduler_type).await?;
    Ok(PaginatedResult {
        data: res
            .data
            .into_iter()
            .map(SchedulerIntegrationResponse::from)
            .collect(),
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })
}

/// 更新调度器集成：仅覆盖传入字段；重名校验；自动刷 updated_at。
pub async fn update_scheduler(
    pool: &SqlitePool,
    id: i64,
    input: UpdateSchedulerInput,
) -> AppResult<SchedulerIntegrationResponse> {
    let existing = scheduler_integration::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("scheduler integration not found"))?;

    if let Some(name) = input.name.as_deref() {
        if name_taken(pool, name, id).await? {
            return Err(AppError::conflict(
                "scheduler integration name already exists",
            ));
        }
    }

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE scheduler_integrations SET \
            name = COALESCE(?1, name), \
            description = COALESCE(?2, description), \
            scheduler_type = COALESCE(?3, scheduler_type), \
            endpoint = COALESCE(?4, endpoint), \
            auth_type = COALESCE(?5, auth_type), \
            credentials = COALESCE(?6, credentials), \
            status = COALESCE(?7, status), \
            version = COALESCE(?8, version), \
            config = COALESCE(?9, config), \
            updated_at = ?10 \
         WHERE id = ?11 AND deleted_at IS NULL",
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.scheduler_type)
    .bind(&input.endpoint)
    .bind(&input.auth_type)
    .bind(input.credentials.map(crate::orm::Json))
    .bind(&input.status)
    .bind(&input.version)
    .bind(input.config.map(crate::orm::Json))
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    let _ = existing;

    let s = scheduler_integration::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("scheduler disappeared after update"))?;
    Ok(s.into())
}

/// 软删除调度器集成（404 若不存在或已软删除）。
pub async fn delete_scheduler(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = scheduler_integration::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("scheduler integration not found"));
    }
    Ok(())
}

/// 测试连接（mock）：用 [`MockSchedulerAdapter`] 探测，返回 status + latency_ms，
/// 并把调度器 status 刷新为探测结果。
pub async fn test_connection(pool: &SqlitePool, id: i64) -> AppResult<ConnectionTest> {
    let s = scheduler_integration::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("scheduler integration not found"))?;

    let adapter = MockSchedulerAdapter::new(s.scheduler_type.clone(), s.cluster_id);
    let result = adapter.test_connection().await?;

    // 把探测到的连接状态回写调度器行。
    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE scheduler_integrations SET status = ?1, last_heartbeat = ?2, updated_at = ?3 \
         WHERE id = ?4",
    )
    .bind(&result.status)
    .bind(now)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    Ok(result)
}

/// 同步资源（mock）：用 [`MockSchedulerAdapter`] 拉取资源快照，刷新 last_heartbeat。
pub async fn sync_resources(pool: &SqlitePool, id: i64) -> AppResult<ResourceSync> {
    let s = scheduler_integration::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("scheduler integration not found"))?;

    let adapter = MockSchedulerAdapter::new(s.scheduler_type.clone(), s.cluster_id);
    let result = adapter.sync_resources().await?;

    scheduler_integration::touch_heartbeat(pool, id).await?;
    Ok(result)
}
