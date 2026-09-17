//! Dataset 数据集模型（对齐 Go `models/acceleration_config.go` 中的 Dataset）。
//!
//! 字段：id / name(unique) / description / type(public|private) / source_path /
//! format / size_bytes / tenant_id(FK) / created_by(FK→users) / status /
//! labels(JSON TEXT) / created_at / updated_at / deleted_at。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use sqlx::SqlitePool;

use crate::error::{AppError, AppResult};
use crate::orm::{
    soft_delete_update_sql, HasTimestamps, Json, PaginatedResult, PaginationParams, SoftDelete,
};

/// `datasets` 表行。
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Dataset {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub name: String,
    pub description: String,
    pub r#type: String,
    pub source_path: String,
    pub format: String,
    pub size_bytes: i64,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: String,
    pub labels: Json<serde_json::Value>,
}

impl HasTimestamps for Dataset {
    fn set_created_at(&mut self, dt: DateTime<Utc>) {
        self.created_at = dt;
    }
    fn set_updated_at(&mut self, dt: DateTime<Utc>) {
        self.updated_at = dt;
    }
}

impl SoftDelete for Dataset {
    fn deleted_at(&self) -> Option<DateTime<Utc>> {
        self.deleted_at
    }
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>) {
        self.deleted_at = dt;
    }
}

/// 对外 Dataset 视图（剔除 deleted_at）。
#[derive(utoipa::ToSchema, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DatasetResponse {
    pub id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub dataset_type: String,
    pub source_path: String,
    pub format: String,
    pub size_bytes: i64,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: String,
    pub labels: serde_json::Value,
}

impl From<Dataset> for DatasetResponse {
    fn from(d: Dataset) -> Self {
        Self {
            id: d.id,
            created_at: d.created_at,
            updated_at: d.updated_at,
            name: d.name,
            description: d.description,
            dataset_type: d.r#type,
            source_path: d.source_path,
            format: d.format,
            size_bytes: d.size_bytes,
            tenant_id: d.tenant_id,
            created_by: d.created_by,
            status: d.status,
            labels: d.labels.0,
        }
    }
}

/// 创建 Dataset 入参。
pub struct NewDataset<'a> {
    pub name: &'a str,
    pub description: &'a str,
    pub dataset_type: &'a str,
    pub source_path: &'a str,
    pub format: &'a str,
    pub size_bytes: i64,
    pub tenant_id: i64,
    pub created_by: i64,
    pub status: &'a str,
    pub labels: serde_json::Value,
}

/// INSERT dataset（自动时间戳）。
pub async fn create(pool: &SqlitePool, input: NewDataset<'_>) -> AppResult<Dataset> {
    let mut dataset = Dataset {
        id: 0,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        deleted_at: None,
        name: input.name.to_string(),
        description: input.description.to_string(),
        r#type: input.dataset_type.to_string(),
        source_path: input.source_path.to_string(),
        format: input.format.to_string(),
        size_bytes: input.size_bytes,
        tenant_id: input.tenant_id,
        created_by: input.created_by,
        status: input.status.to_string(),
        labels: Json(input.labels),
    };
    dataset.before_insert();

    let res = sqlx::query(
        "INSERT INTO datasets (created_at, updated_at, name, description, type, \
         source_path, format, size_bytes, tenant_id, created_by, status, labels) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )
    .bind(dataset.created_at)
    .bind(dataset.updated_at)
    .bind(&dataset.name)
    .bind(&dataset.description)
    .bind(&dataset.r#type)
    .bind(&dataset.source_path)
    .bind(&dataset.format)
    .bind(dataset.size_bytes)
    .bind(dataset.tenant_id)
    .bind(dataset.created_by)
    .bind(&dataset.status)
    .bind(&dataset.labels)
    .execute(pool)
    .await?;

    let id = res.last_insert_rowid();
    get_by_id(pool, id, false)
        .await?
        .ok_or_else(|| AppError::internal("inserted dataset not found"))
}

/// 按 id 查询（默认排除软删除）。
pub async fn get_by_id(
    pool: &SqlitePool,
    id: i64,
    include_deleted: bool,
) -> AppResult<Option<Dataset>> {
    let sql = if include_deleted {
        "SELECT * FROM datasets WHERE id = ?1"
    } else {
        "SELECT * FROM datasets WHERE id = ?1 AND deleted_at IS NULL"
    };
    Ok(sqlx::query_as(sql).bind(id).fetch_optional(pool).await?)
}

/// 按名称检查是否已存在未删除的 dataset。
pub async fn name_taken(pool: &SqlitePool, name: &str, except_id: i64) -> AppResult<bool> {
    let exists: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM datasets WHERE name = ?1 AND deleted_at IS NULL AND id != ?2 LIMIT 1",
    )
    .bind(name)
    .bind(except_id)
    .fetch_optional(pool)
    .await?;
    Ok(exists.is_some())
}

/// 分页列表（可按 tenant_id / type 过滤）。
pub async fn list(
    pool: &SqlitePool,
    params: PaginationParams,
    tenant_id: Option<i64>,
    dataset_type: Option<&str>,
) -> AppResult<PaginatedResult<Dataset>> {
    let params = params.normalize();

    let mut sql_where = String::from("WHERE deleted_at IS NULL");
    if tenant_id.is_some() {
        sql_where.push_str(" AND tenant_id = ?");
    }
    if dataset_type.is_some() {
        sql_where.push_str(" AND type = ?");
    }

    let count_sql = format!("SELECT COUNT(*) FROM datasets {sql_where}");
    let list_sql = format!("SELECT * FROM datasets {sql_where} ORDER BY id ASC LIMIT ? OFFSET ?");

    let mut query_count = sqlx::query_scalar::<_, i64>(&count_sql);
    let mut query_list = sqlx::query_as::<_, Dataset>(&list_sql);

    if let Some(tid) = tenant_id {
        query_count = query_count.bind(tid);
        query_list = query_list.bind(tid);
    }
    if let Some(dt) = dataset_type {
        query_count = query_count.bind(dt);
        query_list = query_list.bind(dt);
    }
    query_list = query_list.bind(params.limit()).bind(params.offset());

    let total: i64 = query_count.fetch_one(pool).await?;
    let rows: Vec<Dataset> = query_list.fetch_all(pool).await?;

    Ok(PaginatedResult::new(rows, total, params))
}

/// 软删除。
pub async fn soft_delete(pool: &SqlitePool, id: i64) -> AppResult<bool> {
    let now = Utc::now();
    let res = sqlx::query(&soft_delete_update_sql("datasets"))
        .bind(now)
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}
