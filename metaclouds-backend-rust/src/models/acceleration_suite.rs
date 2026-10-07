//! AccelerationSuite 模型（组合对象，关联 dataset / training_config /
//! inference_config / fluid_cache）。
//!
//! 字段：id / name(unique) / description / suite_type(training|inference|hybrid) /
//! dataset_id(FK, nullable) / training_config_id(FK, nullable) /
//! inference_config_id(FK, nullable) / fluid_cache_id(FK, nullable) /
//! acceleration_config(JSON TEXT) / tenant_id(FK) / created_by(FK) /
//! status / created_at / updated_at / deleted_at。

use crate::db::DatabasePool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, Json, PaginatedResult, PaginationParams, SoftDelete,
};

/// `acceleration_suites` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AccelerationSuite {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub suite_type: String,
    pub dataset_id: Option<i64>,
    pub training_config_id: Option<i64>,
    pub inference_config_id: Option<i64>,
    pub fluid_cache_id: Option<i64>,
    pub acceleration_config: Json<serde_json::Value>,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: String,
}

impl HasTimestamps for AccelerationSuite {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for AccelerationSuite {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 组合查询返回的嵌套关联对象详情。
#[derive(utoipa::ToSchema, Debug, Clone, Serialize, Deserialize, Default)]
pub struct SuiteAssociations {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub training_config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_config: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fluid_cache: Option<serde_json::Value>,
}

/// 对外 AccelerationSuite 视图（含关联对象详情）。
#[derive(utoipa::ToSchema, Debug, Clone, Serialize, Deserialize)]
pub struct AccelerationSuiteResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
    pub status: String,
    #[serde(flatten)]
    pub associations: SuiteAssociations,
}

impl From<AccelerationSuite> for AccelerationSuiteResponse {
    fn from(s: AccelerationSuite) -> Self {
        Self {
            id: s.id,
            created_at: s.created_at,
            updated_at: s.updated_at,
            name: s.name,
            description: s.description,
            suite_type: s.suite_type,
            dataset_id: s.dataset_id,
            training_config_id: s.training_config_id,
            inference_config_id: s.inference_config_id,
            fluid_cache_id: s.fluid_cache_id,
            acceleration_config: s.acceleration_config.0,
            tenant_id: s.tenant_id,
            created_by: s.created_by,
            status: s.status,
            associations: SuiteAssociations::default(),
        }
    }
}

/// 创建 AccelerationSuite 入参。
pub struct NewAccelerationSuite<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub suite_type: &'a str,
    pub dataset_id: Option<i64>,
    pub training_config_id: Option<i64>,
    pub inference_config_id: Option<i64>,
    pub fluid_cache_id: Option<i64>,
    pub acceleration_config: serde_json::Value,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: &'a str,
}

/// INSERT。
pub async fn create(
    pool: &DatabasePool,
    input: NewAccelerationSuite<'_>,
) -> AppResult<AccelerationSuite> {
    let mut suite = AccelerationSuite {
        id: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        name: input.name.to_string(),
        description: input.description.to_string(),
        suite_type: input.suite_type.to_string(),
        dataset_id: input.dataset_id,
        training_config_id: input.training_config_id,
        inference_config_id: input.inference_config_id,
        fluid_cache_id: input.fluid_cache_id,
        acceleration_config: Json(input.acceleration_config),
        tenant_id: input.tenant_id,
        created_by: input.created_by,
        status: input.status.to_string(),
    };
    suite.before_insert();

    let res = crate::insert_id!(
        pool,
        "INSERT INTO acceleration_suites (created_at, updated_at, name, description, suite_type, \
         dataset_id, training_config_id, inference_config_id, fluid_cache_id, acceleration_config, \
         tenant_id, created_by, status) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        |q| q
            .bind(suite.created_at)
            .bind(suite.updated_at)
            .bind(&suite.name)
            .bind(&suite.description)
            .bind(&suite.suite_type)
            .bind(suite.dataset_id)
            .bind(suite.training_config_id)
            .bind(suite.inference_config_id)
            .bind(suite.fluid_cache_id)
            .bind(&suite.acceleration_config)
            .bind(suite.tenant_id)
            .bind(suite.created_by)
            .bind(&suite.status)
    );

    let id = res;
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted acceleration suite not found"))
}

/// 按 id 查询。
pub async fn get_by_id(
    pool: &DatabasePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<AccelerationSuite>> {
    let sql = if include_deleted {
        "SELECT * FROM acceleration_suites WHERE id = ?1"
    } else {
        "SELECT * FROM acceleration_suites WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(crate::with_db!(pool, sql, |db_s, db_e| {
        crate::db::query_as_db(db_e, db_s)
            .bind(id)
            .fetch_optional(db_e)
            .await?
    }))
}

/// 按名称检查是否已存在未删除的 suite。
pub async fn name_taken(pool: &DatabasePool, name: &str, except_id: i64) -> AppResult<bool> {
    let exists: Option<i64> = crate::with_db!(pool, "SELECT id FROM acceleration_suites WHERE name = ?1 AND deleted_at IS NULL AND id != ?2 LIMIT 1", |db_s, db_e| { crate::db::query_scalar_db(db_e, db_s)
    .bind(name)
    .bind(except_id)
    .fetch_optional(db_e).await? });
    Ok(exists.is_some())
}

/// 分页列表（可按 tenant_id / status 过滤）。
pub async fn list(
    pool: &DatabasePool,
    params: PaginationParams,
    tenant_id: Option<i64>,
    status: Option<&str>,
) -> AppResult<PaginatedResult<AccelerationSuite>> {
    let params = params.normalize();
    let mut sql_where = String::from("deleted_at IS NULL");
    if tenant_id.is_some() {
        sql_where.push_str(" AND tenant_id = ?");
    }
    if status.is_some() {
        sql_where.push_str(" AND status = ?");
    }

    let count_sql = format!("SELECT COUNT(*) FROM acceleration_suites WHERE {sql_where}");
    let list_sql = format!(
        "SELECT * FROM acceleration_suites WHERE {sql_where} ORDER BY id ASC LIMIT ? OFFSET ?"
    );

    let total: i64 = crate::with_db!(pool, &count_sql, |db_s, db_e| {
        let mut sc = crate::db::query_scalar_db(db_e, db_s);
        if let Some(tid) = tenant_id {
            sc = sc.bind(tid);
        }
        if let Some(st) = status {
            sc = sc.bind(st);
        }
        sc.fetch_one(db_e).await?
    });
    let rows: Vec<AccelerationSuite> = crate::with_db!(pool, &list_sql, |db_s, db_e| {
        let mut sc = crate::db::query_as_db(db_e, db_s);
        if let Some(tid) = tenant_id {
            sc = sc.bind(tid);
        }
        if let Some(st) = status {
            sc = sc.bind(st);
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
        &soft_delete_update_sql("acceleration_suites"),
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
