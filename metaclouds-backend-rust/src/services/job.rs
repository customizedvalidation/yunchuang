//! 作业服务（对齐 Go `services/job_service.go`）。
//!
//! CRUD + 分页 + 过滤 + 状态机 + 软删除 + 取消 + 按状态统计。
//! handler 只做参数提取与响应封装，所有 DB 操作在本层。
//!
//! 租户隔离：非管理员只能看到/操作本租户（tenant_id）的作业；越权访问返回 404
//! （对齐 Go `GetJobVisibleTo` / `UpdateJobForTenant`，避免泄露资源是否存在）。

use std::collections::HashMap;

use chrono::Utc;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::job::{self, status as job_status, Job, JobFilter, JobResponse, NewJob};
use crate::orm::{PaginatedResult, PaginationParams};

/// 调用方身份（从 JWT claims 提取）。
#[derive(Debug, Clone, Copy)]
pub struct Actor {
    pub tenant_id: i64,
    pub user_id: i64,
    pub is_admin: bool,
}

/// 创建作业入参（对应 Go `CreateJobRequest`）。
#[derive(Debug, Clone)]
pub struct CreateJobInput {
    pub name: String,
    pub description: String,
    pub kind: String,
    pub priority: i64,
    pub gpus: i64,
    pub cpus: i64,
    pub memory: i64,
    pub duration: i64,
    pub cluster_id: i64,
}

/// 更新作业入参（对应 Go `UpdateJobRequest`；全部可选）。
#[derive(Debug, Clone, Default)]
pub struct UpdateJobInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: Option<String>,
    pub priority: Option<i64>,
    pub progress: Option<i64>,
    pub output_path: Option<String>,
    pub error_msg: Option<String>,
}

/// 创建作业：初始化状态 pending，priority 夹紧到 [0,3]。
pub async fn create_job(
    pool: &SqlitePool,
    input: CreateJobInput,
    actor: Actor,
) -> AppResult<JobResponse> {
    // 对齐 Go：priority 越界则归 0。
    let priority = if (0..=3).contains(&input.priority) {
        input.priority
    } else {
        0
    };

    let job = job::create(
        pool,
        NewJob {
            cluster_id: input.cluster_id,
            tenant_id: actor.tenant_id,
            user_id: actor.user_id,
            name: input.name,
            description: input.description,
            kind: input.kind,
            priority,
            gpus: input.gpus,
            cpus: input.cpus,
            memory: input.memory,
            duration: input.duration,
        },
    )
    .await?;
    Ok(job.into())
}

/// 作业详情（租户隔离：越权返回 404）。
pub async fn get_job(pool: &SqlitePool, id: i64, actor: Actor) -> AppResult<JobResponse> {
    let job = job::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("job not found"))?;
    if !actor.is_admin && job.tenant_id != actor.tenant_id {
        return Err(AppError::not_found("job not found"));
    }
    Ok(job.into())
}

/// 分页作业列表（按 status/type/cluster_id/user_id 过滤，按 name 搜索；非管理员限本租户）。
#[allow(clippy::too_many_arguments)]
pub async fn list_jobs(
    pool: &SqlitePool,
    params: PaginationParams,
    status: Option<&str>,
    kind: Option<&str>,
    cluster_id: Option<i64>,
    user_id: Option<i64>,
    search: Option<&str>,
    actor: Actor,
) -> AppResult<PaginatedResult<JobResponse>> {
    let tenant_filter = if actor.is_admin {
        None
    } else {
        Some(actor.tenant_id)
    };
    let filter = JobFilter {
        status,
        kind,
        cluster_id,
        user_id,
        tenant_id: tenant_filter,
        search,
    };
    let rows = job::list(pool, params, filter).await?;
    Ok(PaginatedResult {
        data: rows.data.into_iter().map(JobResponse::from).collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 校验状态机流转。返回 true 表示允许。
///
/// 终态（completed/failed/cancelled）不可再流转；其余状态之间允许切换，
/// cancelled 可从任意非终态进入（对齐任务要求）。
fn allowed_transition(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    let terminal = |s: &str| {
        matches!(
            s,
            job_status::COMPLETED | job_status::FAILED | job_status::CANCELLED
        )
    };
    if terminal(from) {
        return false;
    }
    matches!(
        to,
        job_status::PENDING
            | job_status::RUNNING
            | job_status::COMPLETED
            | job_status::FAILED
            | job_status::CANCELLED
    )
}

/// 更新作业：仅覆盖传入字段；状态流转走状态机校验。
pub async fn update_job(
    pool: &SqlitePool,
    id: i64,
    input: UpdateJobInput,
    actor: Actor,
) -> AppResult<JobResponse> {
    let existing = job::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("job not found"))?;
    if !actor.is_admin && existing.tenant_id != actor.tenant_id {
        return Err(AppError::not_found("job not found"));
    }

    // 状态流转校验。
    if let Some(new_status) = input.status.as_deref() {
        if !allowed_transition(&existing.status, new_status) {
            return Err(AppError::bad_request(format!(
                "invalid status transition: {} -> {}",
                existing.status, new_status
            )));
        }
    }

    // priority 夹紧 [0,3]（对齐 Go）。
    let priority = match input.priority {
        Some(p) if (0..=3).contains(&p) => Some(p),
        Some(_) => Some(0),
        None => None,
    };

    let now = Utc::now();
    sqlx::query(
        "UPDATE jobs SET \
            name = COALESCE(?, name), \
            description = COALESCE(?, description), \
            status = COALESCE(?, status), \
            priority = COALESCE(?, priority), \
            progress = COALESCE(?, progress), \
            output_path = COALESCE(?, output_path), \
            error_msg = COALESCE(?, error_msg), \
            start_time = CASE WHEN ? = 'running' AND start_time IS NULL THEN ? ELSE start_time END, \
            end_time = CASE WHEN ? IN ('completed','failed','cancelled') AND end_time IS NULL THEN ? ELSE end_time END, \
            updated_at = ? \
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.status)
    .bind(priority)
    .bind(input.progress)
    .bind(&input.output_path)
    .bind(&input.error_msg)
    .bind(input.status.as_deref().unwrap_or(""))
    .bind(now)
    .bind(input.status.as_deref().unwrap_or(""))
    .bind(now)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let updated = job::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("job disappeared after update"))?;
    Ok(updated.into())
}

/// 软删除作业（租户隔离）。
pub async fn delete_job(pool: &SqlitePool, id: i64, actor: Actor) -> AppResult<()> {
    let existing = job::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("job not found"))?;
    if !actor.is_admin && existing.tenant_id != actor.tenant_id {
        return Err(AppError::not_found("job not found"));
    }
    let hit = job::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("job not found"));
    }
    Ok(())
}

/// 取消作业：仅 pending / running 可取消，置为 cancelled。
pub async fn cancel_job(pool: &SqlitePool, id: i64, actor: Actor) -> AppResult<JobResponse> {
    let existing = job::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("job not found"))?;
    if !actor.is_admin && existing.tenant_id != actor.tenant_id {
        return Err(AppError::not_found("job not found"));
    }
    if existing.status != job_status::PENDING && existing.status != job_status::RUNNING {
        return Err(AppError::bad_request(
            "only running or pending jobs can be cancelled",
        ));
    }

    let now = Utc::now();
    sqlx::query(
        "UPDATE jobs SET status = 'cancelled', end_time = COALESCE(end_time, ?), updated_at = ? \
         WHERE id = ? AND deleted_at IS NULL",
    )
    .bind(now)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let updated = job::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("job disappeared after cancel"))?;
    Ok(updated.into())
}

/// 按状态统计作业数量（非管理员限本租户）。
pub async fn get_job_stats(pool: &SqlitePool, actor: Actor) -> AppResult<HashMap<String, i64>> {
    let tenant_filter = if actor.is_admin {
        None
    } else {
        Some(actor.tenant_id)
    };
    let rows = job::count_by_status(pool, tenant_filter).await?;
    Ok(rows.into_iter().collect())
}

/// 供内部/调度器按 id 取作业（无租户隔离；HTTP 入口勿用）。
#[allow(dead_code)]
pub async fn get_job_unscoped(pool: &SqlitePool, id: i64) -> AppResult<Job> {
    job::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("job not found"))
}
