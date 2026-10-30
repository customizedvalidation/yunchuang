//! 配额服务（对齐 WP-P2-B4 调度域规格）。
//!
//! CRUD + 分页 + 按 tenant_id/partition_id/status 过滤 + 软删除 +
//! check_quota（验证资源使用是否超限，返回 allowed + 超限项）+
//! allocate/release（更新 used 字段）。

use crate::db::DatabasePool;

use crate::error::{AppError, AppResult};
use crate::models::resource_quota::{
    self, quota_status, NewQuota, ResourceQuota, ResourceQuotaResponse,
};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建配额入参。
#[derive(Debug, Clone)]
pub struct CreateQuotaInput {
    pub name: String,
    pub description: String,
    pub tenant_id: i64,
    pub partition_id: Option<i64>,
    pub gpu_limit: i64,
    pub cpu_limit: f64,
    pub memory_limit_gb: f64,
    pub storage_limit_gb: f64,
    pub status: Option<String>,
}

/// 更新配额入参：全部可选（limit 字段）。
#[derive(Debug, Clone, Default)]
pub struct UpdateQuotaInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub gpu_limit: Option<i64>,
    pub cpu_limit: Option<f64>,
    pub memory_limit_gb: Option<f64>,
    pub storage_limit_gb: Option<f64>,
    pub status: Option<String>,
}

/// check_quota 请求入参（资源请求量）。
#[derive(Debug, Clone, Copy)]
pub struct QuotaRequest {
    pub gpu: i64,
    pub cpu: f64,
    pub memory_gb: f64,
    pub storage_gb: f64,
}

/// check_quota 结果：是否允许 + 超限项列表。
#[derive(utoipa::ToSchema, Debug, Clone, serde::Serialize)]
pub struct QuotaCheckResult {
    pub allowed: bool,
    pub exceeded: Vec<String>,
}

/// 创建配额（status 默认 active，同名同 scope 冲突 409）。
pub async fn create_quota(
    pool: &DatabasePool,
    input: CreateQuotaInput,
) -> AppResult<ResourceQuotaResponse> {
    // 同名租户内唯一。
    let dup: Option<i64> = crate::with_db!(pool, "SELECT id FROM resource_quotas WHERE tenant_id = ?1 AND name = ?2 AND deleted_at IS NULL LIMIT 1", |db_s, db_e| { crate::db::query_scalar_db(db_e, db_s)
    .bind(input.tenant_id)
    .bind(&input.name)
    .fetch_optional(db_e).await? });
    if dup.is_some() {
        return Err(AppError::conflict(
            "quota name already exists for this tenant",
        ));
    }

    let status = input
        .status
        .unwrap_or_else(|| quota_status::ACTIVE.to_string());
    let q = resource_quota::create(
        pool,
        NewQuota {
            name: input.name,
            description: input.description,
            tenant_id: input.tenant_id,
            partition_id: input.partition_id,
            gpu_limit: input.gpu_limit,
            cpu_limit: input.cpu_limit,
            memory_limit_gb: input.memory_limit_gb,
            storage_limit_gb: input.storage_limit_gb,
            status,
        },
    )
    .await?;
    Ok(q.into())
}

/// 配额详情（404 若不存在或已软删除）。
pub async fn get_quota(pool: &DatabasePool, id: i64) -> AppResult<ResourceQuotaResponse> {
    let q = resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("resource quota not found"))?;
    Ok(q.into())
}

/// 分页配额列表（按 tenant_id/partition_id/status 过滤；排除软删除）。
pub async fn list_quotas(
    pool: &DatabasePool,
    params: PaginationParams,
    tenant_id: Option<i64>,
    partition_id: Option<i64>,
    status: Option<&str>,
) -> AppResult<PaginatedResult<ResourceQuotaResponse>> {
    let res = resource_quota::list(pool, params, tenant_id, partition_id, status).await?;
    Ok(PaginatedResult {
        data: res
            .data
            .into_iter()
            .map(ResourceQuotaResponse::from)
            .collect(),
        total: res.total,
        page: res.page,
        page_size: res.page_size,
        total_pages: res.total_pages,
    })
}

/// 更新配额：仅覆盖传入字段；自动刷 updated_at。
pub async fn update_quota(
    pool: &DatabasePool,
    id: i64,
    input: UpdateQuotaInput,
) -> AppResult<ResourceQuotaResponse> {
    resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("resource quota not found"))?;

    let now = chrono::Utc::now();
    crate::with_db!(
        pool,
        "UPDATE resource_quotas SET \
            name = COALESCE(?1, name), \
            description = COALESCE(?2, description), \
            gpu_limit = COALESCE(?3, gpu_limit), \
            cpu_limit = COALESCE(?4, cpu_limit), \
            memory_limit_gb = COALESCE(?5, memory_limit_gb), \
            storage_limit_gb = COALESCE(?6, storage_limit_gb), \
            status = COALESCE(?7, status), \
            updated_at = ?8 \
         WHERE id = ?9 AND deleted_at IS NULL",
        |db_s, db_e| {
            sqlx::query(db_s)
                .bind(&input.name)
                .bind(&input.description)
                .bind(input.gpu_limit)
                .bind(input.cpu_limit)
                .bind(input.memory_limit_gb)
                .bind(input.storage_limit_gb)
                .bind(&input.status)
                .bind(now)
                .bind(id)
                .execute(db_e)
                .await?
                .rows_affected()
        }
    );

    let q = resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("quota disappeared after update"))?;
    Ok(q.into())
}

/// 软删除配额（404 若不存在或已软删除）。
pub async fn delete_quota(pool: &DatabasePool, id: i64) -> AppResult<()> {
    let hit = resource_quota::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("resource quota not found"));
    }
    Ok(())
}

/// 校验资源请求是否在配额内（不修改 used）。
///
/// 对每类资源：若 limit > 0，则 used + requested > limit 记为超限。
/// 返回 allowed（全部未超限）+ 超限项名称列表。
pub async fn check_quota(
    pool: &DatabasePool,
    id: i64,
    request: QuotaRequest,
) -> AppResult<QuotaCheckResult> {
    let q = resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("resource quota not found"))?;

    let mut exceeded = Vec::new();
    if q.gpu_limit > 0 && q.gpu_used + request.gpu > q.gpu_limit {
        exceeded.push("gpu".to_string());
    }
    if q.cpu_limit > 0.0 && q.cpu_used + request.cpu > q.cpu_limit {
        exceeded.push("cpu".to_string());
    }
    if q.memory_limit_gb > 0.0 && q.memory_used_gb + request.memory_gb > q.memory_limit_gb {
        exceeded.push("memory".to_string());
    }
    if q.storage_limit_gb > 0.0 && q.storage_used_gb + request.storage_gb > q.storage_limit_gb {
        exceeded.push("storage".to_string());
    }

    Ok(QuotaCheckResult {
        allowed: exceeded.is_empty(),
        exceeded,
    })
}

/// 按 scope 校验资源请求是否在配额内（对齐 Go `QuotaService.CheckQuota`）。
///
/// 查找匹配 (scope_type, scope_id, status=active) 的配额；无配额视为允许。
/// 根据 resource_type 检查对应的 limit/used。
pub async fn check_quota_by_scope(
    pool: &DatabasePool,
    scope_type: &str,
    scope_id: i64,
    resource_type: &str,
    requested: i64,
    gpu_fraction: f64,
) -> AppResult<bool> {
    // 根据 scope_type 构建查询条件。
    let row: Option<ResourceQuota> = match scope_type {
        "tenant" => {
            crate::with_db!(
                pool,
                "SELECT * FROM resource_quotas \
                 WHERE tenant_id = ?1 AND status = 'active' AND deleted_at IS NULL \
                 ORDER BY id ASC LIMIT 1",
                |db_s, db_e| {
                    crate::db::query_as_db(db_e, db_s)
                        .bind(scope_id)
                        .fetch_optional(db_e)
                        .await?
                }
            )
        }
        "partition" => {
            crate::with_db!(
                pool,
                "SELECT * FROM resource_quotas \
                 WHERE partition_id = ?1 AND status = 'active' AND deleted_at IS NULL \
                 ORDER BY id ASC LIMIT 1",
                |db_s, db_e| {
                    crate::db::query_as_db(db_e, db_s)
                        .bind(scope_id)
                        .fetch_optional(db_e)
                        .await?
                }
            )
        }
        _ => {
            // 未知 scope_type：无配额限制视为允许（对齐 Go findQuota 返回 nil）。
            return Ok(true);
        }
    };

    let q = match row {
        Some(q) => q,
        None => return Ok(true), // 无配额限制视为允许。
    };

    // 根据 resource_type 检查对应资源 limit。
    // gpu 按整卡当量计：gpu_used（整卡数）+ requested（整卡）+ gpu_fraction（部分卡当量）。
    let allowed = match resource_type {
        "gpu" => q.gpu_limit <= 0
            || (q.gpu_used as f64 + requested as f64 + gpu_fraction) <= q.gpu_limit as f64,
        "cpu" => q.cpu_limit <= 0.0 || q.cpu_used + requested as f64 <= q.cpu_limit,
        "memory" => {
            q.memory_limit_gb <= 0.0 || q.memory_used_gb + requested as f64 <= q.memory_limit_gb
        }
        "storage" => {
            q.storage_limit_gb <= 0.0 || q.storage_used_gb + requested as f64 <= q.storage_limit_gb
        }
        _ => true, // 未知 resource_type 不限制。
    };

    Ok(allowed)
}

/// 分配资源：单条条件原子 UPDATE 累加 used；gpu/cpu/memory/storage 四类资源
/// 均在同一条 UPDATE 的 WHERE 子句内做条件校验，消除"先 check 再 update"的并发
/// 超卖窗口（SQLite 串行写 + PG 行级条件更新）。rows_affected == 0 即配额超限。
///
/// 预读配额仅用于生成精确的错误信息（哪些资源超限）；真正并发防护由下方
/// 条件原子 UPDATE 承担，二者配合做到既安全又可读。
pub async fn allocate_resources(
    pool: &DatabasePool,
    id: i64,
    request: QuotaRequest,
) -> AppResult<ResourceQuotaResponse> {
    let q = resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("resource quota not found"))?;

    let mut exceeded = Vec::new();
    if q.gpu_limit > 0 && q.gpu_used + request.gpu > q.gpu_limit {
        exceeded.push("gpu");
    }
    if q.cpu_limit > 0.0 && q.cpu_used + request.cpu > q.cpu_limit {
        exceeded.push("cpu");
    }
    if q.memory_limit_gb > 0.0 && q.memory_used_gb + request.memory_gb > q.memory_limit_gb {
        exceeded.push("memory");
    }
    if q.storage_limit_gb > 0.0 && q.storage_used_gb + request.storage_gb > q.storage_limit_gb {
        exceeded.push("storage");
    }
    if !exceeded.is_empty() {
        return Err(AppError::forbidden(format!(
            "quota exceeded for: {}",
            exceeded.join(", ")
        )));
    }

    let now = chrono::Utc::now();
    let affected = crate::with_db!(
        pool,
        "UPDATE resource_quotas SET \
            gpu_used = gpu_used + ?1, \
            cpu_used = cpu_used + ?2, \
            memory_used_gb = memory_used_gb + ?3, \
            storage_used_gb = storage_used_gb + ?4, \
            updated_at = ?5 \
         WHERE id = ?6 AND deleted_at IS NULL \
           AND (gpu_limit <= 0 OR gpu_used + ?1 <= gpu_limit) \
           AND (cpu_limit <= 0.0 OR cpu_used + ?2 <= cpu_limit) \
           AND (memory_limit_gb <= 0.0 OR memory_used_gb + ?3 <= memory_limit_gb) \
           AND (storage_limit_gb <= 0.0 OR storage_used_gb + ?4 <= storage_limit_gb)",
        |db_s, db_e| {
            sqlx::query(db_s)
                .bind(request.gpu)
                .bind(request.cpu)
                .bind(request.memory_gb)
                .bind(request.storage_gb)
                .bind(now)
                .bind(id)
                .execute(db_e)
                .await?
                .rows_affected()
        }
    );

    if affected == 0 {
        // 并发场景下预检通过但条件 UPDATE 未命中（极少见），兜底报错。
        return Err(AppError::forbidden("quota exceeded (concurrent)"));
    }

    let q = resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("quota disappeared after allocate"))?;
    Ok(q.into())
}

/// 按租户/分区查找作业对应的有效配额（status=active）。
///
/// 优先匹配 `(tenant_id, partition_id)` 精确分区配额；无精确分区配额时回退到
/// 租户级（`partition_id IS NULL`）配额。找不到任何 active 配额返回 `None`，
/// 调用方应视为"无限额、允许调度"（对齐 `check_quota_by_scope` 的无配额语义）。
pub async fn find_quota_for_scope(
    pool: &DatabasePool,
    tenant_id: i64,
    partition_id: Option<i64>,
) -> AppResult<Option<ResourceQuota>> {
    // 精确分区配额优先。
    if let Some(pid) = partition_id {
        if pid > 0 {
            let exact: Option<ResourceQuota> = crate::with_db!(
                pool,
                "SELECT * FROM resource_quotas \
                 WHERE tenant_id = ?1 AND partition_id = ?2 AND status = 'active' AND deleted_at IS NULL \
                 ORDER BY id ASC LIMIT 1",
                |db_s, db_e| {
                    crate::db::query_as_db(db_e, db_s)
                        .bind(tenant_id)
                        .bind(pid)
                        .fetch_optional(db_e)
                        .await?
                }
            );
            if exact.is_some() {
                return Ok(exact);
            }
        }
    }
    // 回退租户级配额。
    let tenant: Option<ResourceQuota> = crate::with_db!(
        pool,
        "SELECT * FROM resource_quotas \
         WHERE tenant_id = ?1 AND status = 'active' AND deleted_at IS NULL \
         ORDER BY id ASC LIMIT 1",
        |db_s, db_e| {
            crate::db::query_as_db(db_e, db_s)
                .bind(tenant_id)
                .fetch_optional(db_e)
                .await?
        }
    );
    Ok(tenant)
}

/// 释放资源：扣减 used（不为负）。
pub async fn release_resources(
    pool: &DatabasePool,
    id: i64,
    request: QuotaRequest,
) -> AppResult<ResourceQuotaResponse> {
    resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("resource quota not found"))?;

    let now = chrono::Utc::now();
    crate::with_db!(
        pool,
        "UPDATE resource_quotas SET \
            gpu_used = MAX(gpu_used - ?1, 0), \
            cpu_used = MAX(cpu_used - ?2, 0.0), \
            memory_used_gb = MAX(memory_used_gb - ?3, 0.0), \
            storage_used_gb = MAX(storage_used_gb - ?4, 0.0), \
            updated_at = ?5 \
         WHERE id = ?6 AND deleted_at IS NULL",
        |db_s, db_e| {
            sqlx::query(db_s)
                .bind(request.gpu)
                .bind(request.cpu)
                .bind(request.memory_gb)
                .bind(request.storage_gb)
                .bind(now)
                .bind(id)
                .execute(db_e)
                .await?
                .rows_affected()
        }
    );

    let q = resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("quota disappeared after release"))?;
    Ok(q.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::DatabasePool;
    use crate::error::ErrorCode;

    async fn setup_pool() -> DatabasePool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect sqlite memory");
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("run migrations");
        DatabasePool::Sqlite(pool)
    }

    async fn make_quota(
        pool: &DatabasePool,
        gpu: i64,
        cpu: f64,
        mem: f64,
        storage: f64,
    ) -> i64 {
        let q = create_quota(
            pool,
            CreateQuotaInput {
                name: "q".into(),
                description: "".into(),
                tenant_id: 1,
                partition_id: None,
                gpu_limit: gpu,
                cpu_limit: cpu,
                memory_limit_gb: mem,
                storage_limit_gb: storage,
                status: Some("active".into()),
            },
        )
        .await
        .expect("create quota");
        q.id
    }

    #[tokio::test]
    async fn test_allocate_blocks_cpu_oversell() {
        let pool = setup_pool().await;
        // 仅 cpu 限额 2，gpu/mem/storage 无限。
        let id = make_quota(&pool, 0, 2.0, 0.0, 0.0).await;
        assert!(allocate_resources(
            &pool,
            id,
            QuotaRequest { gpu: 0, cpu: 2.0, memory_gb: 0.0, storage_gb: 0.0 }
        )
        .await
        .is_ok());
        // 再要 cpu=1 -> 2+1>2 必须被拒绝（P0-2 修复点）。
        let err = allocate_resources(
            &pool,
            id,
            QuotaRequest { gpu: 0, cpu: 1.0, memory_gb: 0.0, storage_gb: 0.0 },
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
        assert!(err.message.contains("cpu"));
    }

    #[tokio::test]
    async fn test_allocate_blocks_memory_and_storage_oversell() {
        let pool = setup_pool().await;
        let id = make_quota(&pool, 0, 0.0, 4.0, 8.0).await;
        assert!(allocate_resources(
            &pool,
            id,
            QuotaRequest { gpu: 0, cpu: 0.0, memory_gb: 4.0, storage_gb: 0.0 }
        )
        .await
        .is_ok());
        let e = allocate_resources(
            &pool,
            id,
            QuotaRequest { gpu: 0, cpu: 0.0, memory_gb: 1.0, storage_gb: 0.0 },
        )
        .await
        .unwrap_err();
        assert!(e.message.contains("memory"));

        assert!(allocate_resources(
            &pool,
            id,
            QuotaRequest { gpu: 0, cpu: 0.0, memory_gb: 0.0, storage_gb: 8.0 }
        )
        .await
        .is_ok());
        let e2 = allocate_resources(
            &pool,
            id,
            QuotaRequest { gpu: 0, cpu: 0.0, memory_gb: 0.0, storage_gb: 1.0 },
        )
        .await
        .unwrap_err();
        assert!(e2.message.contains("storage"));
    }

    #[tokio::test]
    async fn test_allocate_blocks_gpu_oversell() {
        let pool = setup_pool().await;
        let id = make_quota(&pool, 1, 0.0, 0.0, 0.0).await;
        assert!(allocate_resources(
            &pool,
            id,
            QuotaRequest { gpu: 1, cpu: 0.0, memory_gb: 0.0, storage_gb: 0.0 }
        )
        .await
        .is_ok());
        let e = allocate_resources(
            &pool,
            id,
            QuotaRequest { gpu: 1, cpu: 0.0, memory_gb: 0.0, storage_gb: 0.0 },
        )
        .await
        .unwrap_err();
        assert!(e.message.contains("gpu"));
    }

    #[tokio::test]
    async fn test_find_quota_for_scope_prefers_partition() {
        let pool = setup_pool().await;
        // 租户级配额
        let tenant_q = create_quota(
            &pool,
            CreateQuotaInput {
                name: "tenant".into(),
                description: "".into(),
                tenant_id: 7,
                partition_id: None,
                gpu_limit: 1,
                cpu_limit: 1.0,
                memory_limit_gb: 1.0,
                storage_limit_gb: 1.0,
                status: Some("active".into()),
            },
        )
        .await
        .expect("tenant quota");
        // 分区配额
        let _partition_q = create_quota(
            &pool,
            CreateQuotaInput {
                name: "partition".into(),
                description: "".into(),
                tenant_id: 7,
                partition_id: Some(3),
                gpu_limit: 99,
                cpu_limit: 99.0,
                memory_limit_gb: 99.0,
                storage_limit_gb: 99.0,
                status: Some("active".into()),
            },
        )
        .await
        .expect("partition quota");
        // 精确分区优先
        let found = find_quota_for_scope(&pool, 7, Some(3))
            .await
            .expect("find")
            .expect("some quota");
        assert_eq!(found.id, _partition_q.id);
        assert_eq!(found.gpu_limit, 99);
        // 无分区回退租户级
        let fallback = find_quota_for_scope(&pool, 7, None)
            .await
            .expect("find")
            .expect("some quota");
        assert_eq!(fallback.id, tenant_q.id);
    }
}
