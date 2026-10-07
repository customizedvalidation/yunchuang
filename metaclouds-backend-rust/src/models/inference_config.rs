//! InferenceConfig 模型。
//!
//! 字段：id / name / description / model_path / runtime(triton|tfserving|onnx) /
//! replicas(i32) / gpu_per_replica(i32) / cpu_per_replica(i32) /
//! memory_per_replica_gb(i32) / port(i32) / health_check_path /
//! tenant_id(FK) / created_by(FK) / status / created_at / updated_at / deleted_at。

use crate::db::DatabasePool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, PaginatedResult, PaginationParams, SoftDelete,
};

/// `inference_configs` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub model_path: String,
    pub runtime: String,
    pub replicas: i32,
    pub gpu_per_replica: i32,
    pub cpu_per_replica: i32,
    pub memory_per_replica_gb: i32,
    pub port: i32,
    pub health_check_path: String,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: String,
}

impl HasTimestamps for InferenceConfig {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for InferenceConfig {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外视图。
#[derive(utoipa::ToSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InferenceConfigResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub description: String,
    pub model_path: String,
    pub runtime: String,
    pub replicas: i32,
    pub gpu_per_replica: i32,
    pub cpu_per_replica: i32,
    pub memory_per_replica_gb: i32,
    pub port: i32,
    pub health_check_path: String,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: String,
}

impl From<InferenceConfig> for InferenceConfigResponse {
    fn from(c: InferenceConfig) -> Self {
        Self {
            id: c.id,
            created_at: c.created_at,
            updated_at: c.updated_at,
            name: c.name,
            description: c.description,
            model_path: c.model_path,
            runtime: c.runtime,
            replicas: c.replicas,
            gpu_per_replica: c.gpu_per_replica,
            cpu_per_replica: c.cpu_per_replica,
            memory_per_replica_gb: c.memory_per_replica_gb,
            port: c.port,
            health_check_path: c.health_check_path,
            tenant_id: c.tenant_id,
            created_by: c.created_by,
            status: c.status,
        }
    }
}

/// 创建入参。
pub struct NewInferenceConfig<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub model_path: &'a str,
    pub runtime: &'a str,
    pub replicas: i32,
    pub gpu_per_replica: i32,
    pub cpu_per_replica: i32,
    pub memory_per_replica_gb: i32,
    pub port: i32,
    pub health_check_path: &'a str,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: &'a str,
}

/// INSERT。
pub async fn create(
    pool: &DatabasePool,
    input: NewInferenceConfig<'_>,
) -> AppResult<InferenceConfig> {
    let mut cfg = InferenceConfig {
        id: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        name: input.name.to_string(),
        description: input.description.to_string(),
        model_path: input.model_path.to_string(),
        runtime: input.runtime.to_string(),
        replicas: input.replicas,
        gpu_per_replica: input.gpu_per_replica,
        cpu_per_replica: input.cpu_per_replica,
        memory_per_replica_gb: input.memory_per_replica_gb,
        port: input.port,
        health_check_path: input.health_check_path.to_string(),
        tenant_id: input.tenant_id,
        created_by: input.created_by,
        status: input.status.to_string(),
    };
    cfg.before_insert();

    let res = crate::insert_id!(
        pool,
        "INSERT INTO inference_configs (created_at, updated_at, name, description, model_path, \
         runtime, replicas, gpu_per_replica, cpu_per_replica, memory_per_replica_gb, port, \
         health_check_path, tenant_id, created_by, status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        |q| q
            .bind(cfg.created_at)
            .bind(cfg.updated_at)
            .bind(&cfg.name)
            .bind(&cfg.description)
            .bind(&cfg.model_path)
            .bind(&cfg.runtime)
            .bind(cfg.replicas)
            .bind(cfg.gpu_per_replica)
            .bind(cfg.cpu_per_replica)
            .bind(cfg.memory_per_replica_gb)
            .bind(cfg.port)
            .bind(&cfg.health_check_path)
            .bind(cfg.tenant_id)
            .bind(cfg.created_by)
            .bind(&cfg.status)
    );

    let id = res;
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted inference config not found"))
}

/// 按 id 查询。
pub async fn get_by_id(
    pool: &DatabasePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<InferenceConfig>> {
    let sql = if include_deleted {
        "SELECT * FROM inference_configs WHERE id = ?1"
    } else {
        "SELECT * FROM inference_configs WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(crate::with_db!(pool, sql, |db_s, db_e| {
        crate::db::query_as_db(db_e, db_s)
            .bind(id)
            .fetch_optional(db_e)
            .await?
    }))
}

/// 分页列表（可按 tenant_id 过滤）。
pub async fn list(
    pool: &DatabasePool,
    params: PaginationParams,
    tenant_id: Option<i64>,
) -> AppResult<PaginatedResult<InferenceConfig>> {
    let params = params.normalize();
    let sql_where = match tenant_id {
        Some(_) => "deleted_at IS NULL AND tenant_id = ?".to_string(),
        None => "deleted_at IS NULL".to_string(),
    };

    let count_sql = format!("SELECT COUNT(*) FROM inference_configs WHERE {sql_where}");
    let list_sql = format!(
        "SELECT * FROM inference_configs WHERE {sql_where} ORDER BY id ASC LIMIT ? OFFSET ?"
    );

    let total: i64 = crate::with_db!(pool, &count_sql, |db_s, db_e| {
        let mut sc = crate::db::query_scalar_db(db_e, db_s);
        if let Some(tid) = tenant_id {
            sc = sc.bind(tid);
        }
        sc.fetch_one(db_e).await?
    });
    let rows: Vec<InferenceConfig> = crate::with_db!(pool, &list_sql, |db_s, db_e| {
        let mut sc = crate::db::query_as_db(db_e, db_s);
        if let Some(tid) = tenant_id {
            sc = sc.bind(tid);
        }
        sc.bind(params.limit())
            .bind(params.offset())
            .fetch_all(db_e)
            .await?
    });

    Ok(PaginatedResult::new(rows, total, params))
}

/// 软删除。
pub async fn soft_delete(pool: &DatabasePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = crate::with_db!(
        pool,
        &soft_delete_update_sql("inference_configs"),
        |db_s, db_e| {
            sqlx::query(db_s)
                .bind(now)
                .bind(now)
                .bind(id)
                .execute(db_e)
                .await?
                .rows_affected()
        }
    );
    Ok(res > 0)
}
