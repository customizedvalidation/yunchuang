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
#[derive(Debug, Clone, Default)]
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
    // P1-3 弹性借用
    pub borrow_enabled: bool,
    pub borrow_limit_gpu: i64,
    pub borrow_limit_cpu: f64,
    pub borrow_limit_memory_gb: f64,
    pub borrow_limit_storage_gb: f64,
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
    // P1-3 弹性借用（可选覆盖）
    pub borrow_enabled: Option<bool>,
    pub borrow_limit_gpu: Option<i64>,
    pub borrow_limit_cpu: Option<f64>,
    pub borrow_limit_memory_gb: Option<f64>,
    pub borrow_limit_storage_gb: Option<f64>,
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
            borrow_enabled: if input.borrow_enabled { 1 } else { 0 },
            borrow_limit_gpu: input.borrow_limit_gpu,
            borrow_limit_cpu: input.borrow_limit_cpu,
            borrow_limit_memory_gb: input.borrow_limit_memory_gb,
            borrow_limit_storage_gb: input.borrow_limit_storage_gb,
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
            borrow_enabled = COALESCE(?10, borrow_enabled), \
            borrow_limit_gpu = COALESCE(?11, borrow_limit_gpu), \
            borrow_limit_cpu = COALESCE(?12, borrow_limit_cpu), \
            borrow_limit_memory_gb = COALESCE(?13, borrow_limit_memory_gb), \
            borrow_limit_storage_gb = COALESCE(?14, borrow_limit_storage_gb), \
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
                .bind(input.borrow_enabled.map(|b| if b { 1i64 } else { 0i64 }))
                .bind(input.borrow_limit_gpu)
                .bind(input.borrow_limit_cpu)
                .bind(input.borrow_limit_memory_gb)
                .bind(input.borrow_limit_storage_gb)
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

/// 分配资源（含弹性借用，P1-3）。
///
/// 语义：先尝试在租户**自身** `limit` 内原子分配（与 [`allocate_resources`] 同一条条件
/// UPDATE）；当自身限额不足且配额开启了 `borrow_enabled` 时，允许从**集群空闲容量**
/// （所有 active 配额 `limit - used` 之和，即其他租户未用额度）借用，但必须满足：
///   1. 本租户预计借用量 ≤ 其 `borrow_limit_*`（单租户借用上限）；
///   2. 请求量 ≤ 集群当前空闲余量（集群级不超卖）。
///
/// 全部在单事务内完成；Postgres 用 `FOR UPDATE` 锁住 cohort 全部 active 配额行使空闲
/// 余量权威，SQLite 因串行写天然安全。借用成功后 `used` 可暂时超过 `limit`，释放时按
/// 申请量扣减自动归还（无需额外记账）。
pub async fn allocate_resources_with_borrow(
    pool: &DatabasePool,
    id: i64,
    request: QuotaRequest,
) -> AppResult<ResourceQuotaResponse> {
    let now = chrono::Utc::now();
    // 事务内返回是否走了借用路径（用于埋点）。
    let borrowed: bool = crate::with_tx!(pool, |tx| {
        // PG：锁住 cohort 全部 active 配额行，使下方集群空闲余量读取权威、防并发超借。
        if !pool.is_sqlite() {
            crate::with_db_tx!(
                pool,
                &mut *tx,
                "SELECT id FROM resource_quotas WHERE status = 'active' AND deleted_at IS NULL FOR UPDATE",
                |db_s, db_e| {
                    sqlx::query(db_s)
                        .fetch_all(&mut *db_e)
                        .await?
                }
            );
        }

        let q: ResourceQuota = crate::with_db_tx!(
            pool,
            &mut *tx,
            "SELECT * FROM resource_quotas WHERE id = ?1 AND deleted_at IS NULL",
            |db_s, db_e| {
                crate::db::query_as_db(&mut *db_e, db_s)
                    .bind(id)
                    .fetch_one(&mut *db_e)
                    .await?
            }
        );

        // 1) 自身限额内可行性。
        let fits = (q.gpu_limit <= 0 || q.gpu_used + request.gpu <= q.gpu_limit)
            && (q.cpu_limit <= 0.0 || q.cpu_used + request.cpu <= q.cpu_limit)
            && (q.memory_limit_gb <= 0.0
                || q.memory_used_gb + request.memory_gb <= q.memory_limit_gb)
            && (q.storage_limit_gb <= 0.0
                || q.storage_used_gb + request.storage_gb <= q.storage_limit_gb);
        let borrowed = if fits {
            let affected = crate::with_db_tx!(
                pool,
                &mut *tx,
                "UPDATE resource_quotas SET \
                    gpu_used = gpu_used + ?1, cpu_used = cpu_used + ?2, \
                    memory_used_gb = memory_used_gb + ?3, storage_used_gb = storage_used_gb + ?4, \
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
                        .execute(&mut *db_e)
                        .await?
                        .rows_affected()
                }
            );
            if affected == 0 {
                return Err(AppError::forbidden("quota exceeded (concurrent)"));
            }
            false
        } else {

        // 2) 借用路径。
        if q.borrow_enabled == 0 {
            return Err(AppError::forbidden(
                "quota exceeded (borrow disabled)",
            ));
        }

        // 集群空闲余量（跨租户共享）。
        let slack: (i64, f64, f64, f64) = crate::with_db_tx!(
            pool,
            &mut *tx,
            "SELECT COALESCE(SUM(gpu_limit - gpu_used), 0), \
                    COALESCE(SUM(cpu_limit - cpu_used), 0.0), \
                    COALESCE(SUM(memory_limit_gb - memory_used_gb), 0.0), \
                    COALESCE(SUM(storage_limit_gb - storage_used_gb), 0.0) \
             FROM resource_quotas WHERE status = 'active' AND deleted_at IS NULL",
            |db_s, db_e| {
                crate::db::query_as_db(&mut *db_e, db_s)
                    .fetch_one(&mut *db_e)
                    .await?
            }
        );
        let (gpu_slack, cpu_slack, mem_slack, sto_slack) = slack;

        // 本租户预计借用量（仅超过自身 limit 的部分计入）。
        let proj_borrow_gpu = if q.gpu_limit > 0 {
            (q.gpu_used + request.gpu - q.gpu_limit).max(0)
        } else {
            0
        };
        let proj_borrow_cpu = if q.cpu_limit > 0.0 {
            (q.cpu_used + request.cpu - q.cpu_limit).max(0.0)
        } else {
            0.0
        };
        let proj_borrow_mem = if q.memory_limit_gb > 0.0 {
            (q.memory_used_gb + request.memory_gb - q.memory_limit_gb).max(0.0)
        } else {
            0.0
        };
        let proj_borrow_sto = if q.storage_limit_gb > 0.0 {
            (q.storage_used_gb + request.storage_gb - q.storage_limit_gb).max(0.0)
        } else {
            0.0
        };

        if proj_borrow_gpu > q.borrow_limit_gpu
            || proj_borrow_cpu > q.borrow_limit_cpu
            || proj_borrow_mem > q.borrow_limit_memory_gb
            || proj_borrow_sto > q.borrow_limit_storage_gb
        {
            return Err(AppError::forbidden("quota borrow limit exceeded"));
        }
        if request.gpu > gpu_slack
            || request.cpu > cpu_slack
            || request.memory_gb > mem_slack
            || request.storage_gb > sto_slack
        {
            return Err(AppError::forbidden(
                "cluster slack insufficient for borrow",
            ));
        }

        // 3) 借用：直接累加 used（可超过自身 limit；上限与集群余量已校验）。
        crate::with_db_tx!(
            pool,
            &mut *tx,
            "UPDATE resource_quotas SET \
                gpu_used = gpu_used + ?1, cpu_used = cpu_used + ?2, \
                memory_used_gb = memory_used_gb + ?3, storage_used_gb = storage_used_gb + ?4, \
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
                    .execute(&mut *db_e)
                    .await?
                    .rows_affected()
            }
        );
            true
        };
        borrowed
    });

    if borrowed {
        crate::metrics::scheduler_metrics().borrowed_quota_total.inc();
    }

    let q = resource_quota::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("quota disappeared after borrow"))?;
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
                ..Default::default()
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
                ..Default::default()
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
                ..Default::default()
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

    #[tokio::test]
    async fn test_allocate_borrows_cluster_slack() {
        let pool = setup_pool().await;
        // 租户 B：富余 4 GPU 配额（借用来源）。
        create_quota(
            &pool,
            CreateQuotaInput {
                name: "b".into(),
                tenant_id: 2,
                gpu_limit: 4,
                status: Some("active".into()),
                ..Default::default()
            },
        )
        .await
        .expect("create b");
        // 租户 A：自身限额 1，开启借用，borrow_limit_gpu=2。
        let a = create_quota(
            &pool,
            CreateQuotaInput {
                name: "a".into(),
                tenant_id: 1,
                gpu_limit: 1,
                status: Some("active".into()),
                borrow_enabled: true,
                borrow_limit_gpu: 2,
                ..Default::default()
            },
        )
        .await
        .expect("create a");

        // 请求 3 GPU：自身 1 + 借 2，集群闲余 4，应成功。
        let res = allocate_resources_with_borrow(
            &pool,
            a.id,
            QuotaRequest { gpu: 3, cpu: 0.0, memory_gb: 0.0, storage_gb: 0.0 },
        )
        .await
        .expect("borrow allocate");
        assert_eq!(res.gpu_used, 3, "should use 1 own + 2 borrowed");

        // 释放后 used 回到 0。
        let released = release_resources(
            &pool,
            a.id,
            QuotaRequest { gpu: 3, cpu: 0.0, memory_gb: 0.0, storage_gb: 0.0 },
        )
        .await
        .expect("release");
        assert_eq!(released.gpu_used, 0);
    }

    #[tokio::test]
    async fn test_allocate_borrow_disabled_rejected() {
        let pool = setup_pool().await;
        let a = create_quota(
            &pool,
            CreateQuotaInput {
                name: "a".into(),
                tenant_id: 1,
                gpu_limit: 1,
                status: Some("active".into()),
                borrow_enabled: false,
                ..Default::default()
            },
        )
        .await
        .expect("create a");
        // 请求 2 GPU 超过自身限额且借用关闭 → 必须拒绝。
        let err = allocate_resources_with_borrow(
            &pool,
            a.id,
            QuotaRequest { gpu: 2, cpu: 0.0, memory_gb: 0.0, storage_gb: 0.0 },
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
    }

    #[tokio::test]
    async fn test_allocate_borrow_exceeds_borrow_limit_rejected() {
        let pool = setup_pool().await;
        create_quota(
            &pool,
            CreateQuotaInput {
                name: "b".into(),
                tenant_id: 2,
                gpu_limit: 10,
                status: Some("active".into()),
                ..Default::default()
            },
        )
        .await
        .expect("create b");
        let a = create_quota(
            &pool,
            CreateQuotaInput {
                name: "a".into(),
                tenant_id: 1,
                gpu_limit: 1,
                status: Some("active".into()),
                borrow_enabled: true,
                borrow_limit_gpu: 2,
                ..Default::default()
            },
        )
        .await
        .expect("create a");
        // 请求 5 GPU：自身 1 + 借 4 > borrow_limit 2 → 拒绝。
        let err = allocate_resources_with_borrow(
            &pool,
            a.id,
            QuotaRequest { gpu: 5, cpu: 0.0, memory_gb: 0.0, storage_gb: 0.0 },
        )
        .await
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
    }
}
