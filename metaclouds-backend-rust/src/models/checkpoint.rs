//! Checkpoint 检查点模型。
//!
//! 字段：id / name / description / job_id(FK, nullable) / dataset_id(FK, nullable) /
//! path / format / size_bytes / step(i64) / epoch(i64) / metrics(JSON TEXT) /
//! tenant_id(FK) / created_by(FK) / status / created_at / updated_at / deleted_at。

use crate::db::DatabasePool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, Json, PaginatedResult, PaginationParams, SoftDelete,
};

/// `checkpoints` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub job_id: Option<i64>,
    pub dataset_id: Option<i64>,
    pub path: String,
    pub format: String,
    pub size_bytes: i64,
    pub step: i64,
    pub epoch: i64,
    pub metrics: Json<serde_json::Value>,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: String,
}

impl HasTimestamps for Checkpoint {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for Checkpoint {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外 Checkpoint 视图。
#[derive(utoipa::ToSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CheckpointResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub description: String,
    pub job_id: Option<i64>,
    pub dataset_id: Option<i64>,
    pub path: String,
    pub format: String,
    pub size_bytes: i64,
    pub step: i64,
    pub epoch: i64,
    pub metrics: serde_json::Value,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: String,
}

impl From<Checkpoint> for CheckpointResponse {
    fn from(c: Checkpoint) -> Self {
        Self {
            id: c.id,
            created_at: c.created_at,
            updated_at: c.updated_at,
            name: c.name,
            description: c.description,
            job_id: c.job_id,
            dataset_id: c.dataset_id,
            path: c.path,
            format: c.format,
            size_bytes: c.size_bytes,
            step: c.step,
            epoch: c.epoch,
            metrics: c.metrics.0,
            tenant_id: c.tenant_id,
            created_by: c.created_by,
            status: c.status,
        }
    }
}

/// 创建 Checkpoint 入参。
pub struct NewCheckpoint<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub job_id: Option<i64>,
    pub dataset_id: Option<i64>,
    pub path: &'a str,
    pub format: &'a str,
    pub size_bytes: i64,
    pub step: i64,
    pub epoch: i64,
    pub metrics: serde_json::Value,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: &'a str,
}

/// INSERT。
pub async fn create(pool: &DatabasePool, input: NewCheckpoint<'_>) -> AppResult<Checkpoint> {
    let mut ckpt = Checkpoint {
        id: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        name: input.name.to_string(),
        description: input.description.to_string(),
        job_id: input.job_id,
        dataset_id: input.dataset_id,
        path: input.path.to_string(),
        format: input.format.to_string(),
        size_bytes: input.size_bytes,
        step: input.step,
        epoch: input.epoch,
        metrics: Json(input.metrics),
        tenant_id: input.tenant_id,
        created_by: input.created_by,
        status: input.status.to_string(),
    };
    ckpt.before_insert();

    let res = crate::insert_id!(
        pool,
        "INSERT INTO checkpoints (created_at, updated_at, name, description, job_id, dataset_id, \
         path, format, size_bytes, step, epoch, metrics, tenant_id, created_by, status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        |q| q
            .bind(ckpt.created_at)
            .bind(ckpt.updated_at)
            .bind(&ckpt.name)
            .bind(&ckpt.description)
            .bind(ckpt.job_id)
            .bind(ckpt.dataset_id)
            .bind(&ckpt.path)
            .bind(&ckpt.format)
            .bind(ckpt.size_bytes)
            .bind(ckpt.step)
            .bind(ckpt.epoch)
            .bind(&ckpt.metrics)
            .bind(ckpt.tenant_id)
            .bind(ckpt.created_by)
            .bind(&ckpt.status)
    );

    let id = res;
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted checkpoint not found"))
}

/// 按 id 查询。
pub async fn get_by_id(
    pool: &DatabasePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<Checkpoint>> {
    let sql = if include_deleted {
        "SELECT * FROM checkpoints WHERE id = ?1"
    } else {
        "SELECT * FROM checkpoints WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(crate::with_db!(pool, sql, |db_s, db_e| {
        crate::db::query_as_db(db_e, db_s)
            .bind(id)
            .fetch_optional(db_e)
            .await?
    }))
}

/// 分页列表（可按 job_id / dataset_id 过滤）。
pub async fn list(
    pool: &DatabasePool,
    params: PaginationParams,
    job_id: Option<i64>,
    dataset_id: Option<i64>,
) -> AppResult<PaginatedResult<Checkpoint>> {
    let params = params.normalize();
    let mut sql_where = String::from("deleted_at IS NULL");
    if job_id.is_some() {
        sql_where.push_str(" AND job_id = ?");
    }
    if dataset_id.is_some() {
        sql_where.push_str(" AND dataset_id = ?");
    }

    let count_sql = format!("SELECT COUNT(*) FROM checkpoints WHERE {sql_where}");
    let list_sql =
        format!("SELECT * FROM checkpoints WHERE {sql_where} ORDER BY id ASC LIMIT ? OFFSET ?");

    let total: i64 = crate::with_db!(pool, &count_sql, |db_s, db_e| {
        let mut sc = crate::db::query_scalar_db(db_e, db_s);
        if let Some(jid) = job_id {
            sc = sc.bind(jid);
        }
        if let Some(did) = dataset_id {
            sc = sc.bind(did);
        }
        sc.fetch_one(db_e).await?
    });
    let rows: Vec<Checkpoint> = crate::with_db!(pool, &list_sql, |db_s, db_e| {
        let mut sc = crate::db::query_as_db(db_e, db_s);
        if let Some(jid) = job_id {
            sc = sc.bind(jid);
        }
        if let Some(did) = dataset_id {
            sc = sc.bind(did);
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
        &soft_delete_update_sql("checkpoints"),
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
