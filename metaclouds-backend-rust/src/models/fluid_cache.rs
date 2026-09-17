//! FluidCache 模型（对齐 Go `models/acceleration_config.go` 中的 FluidCache）。
//!
//! 字段：id / name / dataset_id(FK) / namespace / path / cache_class /
//! replicas(i32) / status / created_at / updated_at / deleted_at。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, PaginatedResult, PaginationParams, SoftDelete,
};

/// `fluid_caches` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FluidCache {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
    pub dataset_id: i64,
    pub namespace: String,
    pub path: String,
    pub cache_class: String,
    pub replicas: i32,
    pub status: String,
}

impl HasTimestamps for FluidCache {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for FluidCache {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外 FluidCache 视图。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FluidCacheResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub dataset_id: i64,
    pub namespace: String,
    pub path: String,
    pub cache_class: String,
    pub replicas: i32,
    pub status: String,
}

impl From<FluidCache> for FluidCacheResponse {
    fn from(c: FluidCache) -> Self {
        Self {
            id: c.id,
            created_at: c.created_at,
            updated_at: c.updated_at,
            name: c.name,
            dataset_id: c.dataset_id,
            namespace: c.namespace,
            path: c.path,
            cache_class: c.cache_class,
            replicas: c.replicas,
            status: c.status,
        }
    }
}

/// 创建 FluidCache 入参。
pub struct NewFluidCache<'a> {
    pub name: &'a str,
    pub dataset_id: i64,
    pub namespace: &'a str,
    pub path: &'a str,
    pub cache_class: &'a str,
    pub replicas: i32,
    pub status: &'a str,
}

/// INSERT fluid_cache。
pub async fn create(pool: &SqlitePool, input: NewFluidCache<'_>) -> AppResult<FluidCache> {
    let mut cache = FluidCache {
        id: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        name: input.name.to_string(),
        dataset_id: input.dataset_id,
        namespace: input.namespace.to_string(),
        path: input.path.to_string(),
        cache_class: input.cache_class.to_string(),
        replicas: input.replicas,
        status: input.status.to_string(),
    };
    cache.before_insert();

    let res = sqlx::query(
        "INSERT INTO fluid_caches (created_at, updated_at, name, dataset_id, namespace, \
         path, cache_class, replicas, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )
    .bind(cache.created_at)
    .bind(cache.updated_at)
    .bind(&cache.name)
    .bind(cache.dataset_id)
    .bind(&cache.namespace)
    .bind(&cache.path)
    .bind(&cache.cache_class)
    .bind(cache.replicas)
    .bind(&cache.status)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted fluid cache not found"))
}

/// 按 id 查询。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<FluidCache>> {
    let sql = if include_deleted {
        "SELECT * FROM fluid_caches WHERE id = ?1"
    } else {
        "SELECT * FROM fluid_caches WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 按 dataset_id 过滤分页列表。
pub async fn list_by_dataset(
    pool: &SqlitePool,
    params: PaginationParams,
    dataset_id: Option<i64>,
) -> AppResult<PaginatedResult<FluidCache>> {
    let params = params.normalize();
    let sql_where = match dataset_id {
        Some(_) => "deleted_at IS NULL AND dataset_id = ?".to_string(),
        None => "deleted_at IS NULL".to_string(),
    };

    let count_sql = format!("SELECT COUNT(*) FROM fluid_caches WHERE {sql_where}");
    let list_sql =
        format!("SELECT * FROM fluid_caches WHERE {sql_where} ORDER BY id ASC LIMIT ? OFFSET ?");

    let mut query_count = sqlx::query_scalar::<_, i64>(&count_sql);
    let mut query_list = sqlx::query_as::<_, FluidCache>(&list_sql);

    if let Some(did) = dataset_id {
        query_count = query_count.bind(did);
        query_list = query_list.bind(did);
    }
    query_list = query_list.bind(params.limit()).bind(params.offset());

    let total: i64 = query_count.fetch_one(pool).await?;
    let rows: Vec<FluidCache> = query_list.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("fluid_caches"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
