//! Acceleration 服务（对齐 Go `services/acceleration_service.go` +
//! `acceleration_extensions.go`）。
//!
//! Suite CRUD + 组合查询（含关联对象详情）+ suite 状态流转（start/stop）。

use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::models::acceleration_suite::{self, AccelerationSuite, AccelerationSuiteResponse};
use crate::orm::{PaginatedResult, PaginationParams};

/// 创建 Suite 入参。
#[derive(Debug, Clone)]
pub struct CreateSuiteInput {
    pub name: String,
    pub description: String,
    pub suite_type: String,
    pub dataset_id: Option<i64>,
    pub training_config_id: Option<i64>,
    pub inference_config_id: Option<i64>,
    pub fluid_cache_id: Option<i64>,
    pub acceleration_config: serde_json::Value,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: Option<String>,
}

/// 更新 Suite 入参。
#[derive(Debug, Clone, Default)]
pub struct UpdateSuiteInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub suite_type: Option<String>,
    pub dataset_id: Option<Option<i64>>,
    pub training_config_id: Option<Option<i64>>,
    pub inference_config_id: Option<Option<i64>>,
    pub fluid_cache_id: Option<Option<i64>>,
    pub acceleration_config: Option<serde_json::Value>,
    pub status: Option<String>,
}

/// 组装带关联对象的响应。
async fn build_response(
    pool: &SqlitePool,
    suite: AccelerationSuite,
) -> AppResult<AccelerationSuiteResponse> {
    let mut resp: AccelerationSuiteResponse = suite.into();

    // 组合查询：按需加载关联对象
    if let Some(ds_id) = resp.dataset_id {
        if let Ok(Some(row)) = sqlx::query_as::<_, crate::models::dataset::Dataset>(
            "SELECT * FROM datasets WHERE id = ?1 AND deleted_at IS NULL",
        )
        .bind(ds_id)
        .fetch_optional(pool)
        .await
        {
            resp.associations.dataset = Some(
                serde_json::to_value(crate::models::dataset::DatasetResponse::from(row))
                    .unwrap_or(serde_json::Value::Null),
            );
        }
    }
    if let Some(tc_id) = resp.training_config_id {
        if let Ok(Some(row)) = sqlx::query_as::<
            _,
            crate::models::distributed_training_config::DistributedTrainingConfig,
        >(
            "SELECT * FROM distributed_training_configs WHERE id = ?1 AND deleted_at IS NULL",
        )
        .bind(tc_id)
        .fetch_optional(pool)
        .await
        {
            resp.associations.training_config = Some(
                serde_json::to_value(crate::models::distributed_training_config::DistributedTrainingConfigResponse::from(row))
                    .unwrap_or(serde_json::Value::Null),
            );
        }
    }
    if let Some(ic_id) = resp.inference_config_id {
        if let Ok(Some(row)) =
            sqlx::query_as::<_, crate::models::inference_config::InferenceConfig>(
                "SELECT * FROM inference_configs WHERE id = ?1 AND deleted_at IS NULL",
            )
            .bind(ic_id)
            .fetch_optional(pool)
            .await
        {
            resp.associations.inference_config = Some(
                serde_json::to_value(
                    crate::models::inference_config::InferenceConfigResponse::from(row),
                )
                .unwrap_or(serde_json::Value::Null),
            );
        }
    }
    if let Some(fc_id) = resp.fluid_cache_id {
        if let Ok(Some(row)) = sqlx::query_as::<_, crate::models::fluid_cache::FluidCache>(
            "SELECT * FROM fluid_caches WHERE id = ?1 AND deleted_at IS NULL",
        )
        .bind(fc_id)
        .fetch_optional(pool)
        .await
        {
            resp.associations.fluid_cache = Some(
                serde_json::to_value(crate::models::fluid_cache::FluidCacheResponse::from(row))
                    .unwrap_or(serde_json::Value::Null),
            );
        }
    }

    Ok(resp)
}

/// 创建 suite。
pub async fn create_suite(
    pool: &SqlitePool,
    input: CreateSuiteInput,
) -> AppResult<AccelerationSuiteResponse> {
    if acceleration_suite::name_taken(pool, &input.name, 0).await? {
        return Err(AppError::conflict("acceleration suite name already exists"));
    }
    let status = input.status.unwrap_or_else(|| "active".to_string());
    let suite = acceleration_suite::create(
        pool,
        acceleration_suite::NewAccelerationSuite {
            name: &input.name,
            description: &input.description,
            suite_type: &input.suite_type,
            dataset_id: input.dataset_id,
            training_config_id: input.training_config_id,
            inference_config_id: input.inference_config_id,
            fluid_cache_id: input.fluid_cache_id,
            acceleration_config: input.acceleration_config,
            tenant_id: input.tenant_id,
            created_by: input.created_by,
            status: &status,
        },
    )
    .await?;
    build_response(pool, suite).await
}

/// Suite 详情（含关联对象）。
pub async fn get_suite(pool: &SqlitePool, id: i64) -> AppResult<AccelerationSuiteResponse> {
    let suite = acceleration_suite::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("acceleration suite not found"))?;
    build_response(pool, suite).await
}

/// 分页列表（不含关联对象，仅主表）。
pub async fn list_suites(
    pool: &SqlitePool,
    params: PaginationParams,
    tenant_id: Option<i64>,
    status: Option<&str>,
) -> AppResult<PaginatedResult<AccelerationSuiteResponse>> {
    let rows: PaginatedResult<AccelerationSuite> =
        acceleration_suite::list(pool, params, tenant_id, status).await?;
    Ok(PaginatedResult {
        data: rows
            .data
            .into_iter()
            .map(AccelerationSuiteResponse::from)
            .collect(),
        total: rows.total,
        page: rows.page,
        page_size: rows.page_size,
        total_pages: rows.total_pages,
    })
}

/// 更新 suite。
pub async fn update_suite(
    pool: &SqlitePool,
    id: i64,
    input: UpdateSuiteInput,
) -> AppResult<AccelerationSuiteResponse> {
    acceleration_suite::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::not_found("acceleration suite not found"))?;

    if let Some(name) = input.name.as_deref() {
        if acceleration_suite::name_taken(pool, name, id).await? {
            return Err(AppError::conflict("acceleration suite name already exists"));
        }
    }

    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE acceleration_suites SET \
            name = COALESCE(?1, name), \
            description = COALESCE(?2, description), \
            suite_type = COALESCE(?3, suite_type), \
            dataset_id = COALESCE(?4, dataset_id), \
            training_config_id = COALESCE(?5, training_config_id), \
            inference_config_id = COALESCE(?6, inference_config_id), \
            fluid_cache_id = COALESCE(?7, fluid_cache_id), \
            acceleration_config = COALESCE(?8, acceleration_config), \
            status = COALESCE(?9, status), \
            updated_at = ?10 \
         WHERE id = ?11 AND deleted_at IS NULL",
    )
    .bind(input.name)
    .bind(input.description)
    .bind(input.suite_type)
    .bind(input.dataset_id.flatten())
    .bind(input.training_config_id.flatten())
    .bind(input.inference_config_id.flatten())
    .bind(input.fluid_cache_id.flatten())
    .bind(
        input
            .acceleration_config
            .map(|v| serde_json::to_string(&v).unwrap_or_else(|_| "{}".into())),
    )
    .bind(input.status)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;

    let suite = acceleration_suite::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("suite disappeared after update"))?;
    build_response(pool, suite).await
}

/// 软删除。
pub async fn delete_suite(pool: &SqlitePool, id: i64) -> AppResult<()> {
    let hit = acceleration_suite::soft_delete(pool, id).await?;
    if !hit {
        return Err(AppError::not_found("acceleration suite not found"));
    }
    Ok(())
}

/// 启动 suite：状态 → running。
pub async fn start_suite(pool: &SqlitePool, id: i64) -> AppResult<AccelerationSuiteResponse> {
    let now = chrono::Utc::now();
    let res = sqlx::query(
        "UPDATE acceleration_suites SET status = 'running', updated_at = ?1 \
         WHERE id = ?2 AND deleted_at IS NULL",
    )
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("acceleration suite not found"));
    }
    let suite = acceleration_suite::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("suite disappeared after start"))?;
    build_response(pool, suite).await
}

/// 停止 suite：状态 → stopped。
pub async fn stop_suite(pool: &SqlitePool, id: i64) -> AppResult<AccelerationSuiteResponse> {
    let now = chrono::Utc::now();
    let res = sqlx::query(
        "UPDATE acceleration_suites SET status = 'stopped', updated_at = ?1 \
         WHERE id = ?2 AND deleted_at IS NULL",
    )
    .bind(now)
    .bind(id)
    .execute(pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::not_found("acceleration suite not found"));
    }
    let suite = acceleration_suite::get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("suite disappeared after stop"))?;
    build_response(pool, suite).await
}
