//! GORM 特性显式层（WP-P1-06）。
//!
//! 用 trait + 小工具把 Go GORM 的“魔法行为”显式化、可审查化，使后续领域模型不必
//! 在每一处手写软删除过滤 / 时间戳填充：
//!
//! - [`HasTimestamps`]：对齐 GORM 的 `CreatedAt` / `UpdatedAt` 自动时间戳；
//! - [`SoftDelete`]：对齐 GORM 的 `gorm.DeletedAt`（查询默认过滤 `deleted_at IS NULL`，
//!   DELETE 改写为 `UPDATE SET deleted_at = now()`）；
//! - [`PaginationParams`] / [`PaginatedResult`] / [`total_pages`]：对齐
//!   Go `pkg/response/pagination.go`（page 默认 1，page_size 默认 10、上限 100）；
//! - [`Json`]：对齐 GORM `type:json` tag 的 JSON 字段包装（SQLite 用 TEXT，Postgres 用 JSONB）。
//!
//! # 关联预加载约定
//!
//! 本层**不**实现 GORM 那套 magic `Preload`。约定如下：
//!
//! 1. 需要关联数据时**手写 JOIN 查询**，或定义嵌套的 `FromRow` 投影结构
//!    （例如 `TenantWithUsers`），在 repository 层显式组装；
//! 2. 列表接口默认只查主表，详情接口才按需 JOIN；
//! 3. 禁止在模型结构体里挂“半加载”的关联字段，避免出现 Go 侧
//!    `Tenant.Users` 未预加载即 `nil` 的静默歧义。
//!
//! # 自包含说明
//!
//! 本模块不依赖 crate 内部其他模块（error / config / models），可独立编译。

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::postgres::{PgArgumentBuffer, PgTypeInfo, PgValueRef, Postgres};
use sqlx::sqlite::{Sqlite, SqliteArgumentValue, SqliteTypeInfo};
use sqlx::{Decode, Encode, Type};

// ---------------------------------------------------------------------------
// HasTimestamps：对齐 GORM CreatedAt / UpdatedAt
// ---------------------------------------------------------------------------

/// 具备自动时间戳的模型应实现本 trait。
///
/// GORM 行为：
/// - INSERT 时同时写入 `created_at` 与 `updated_at`；
/// - UPDATE 时只刷新 `updated_at`，`created_at` 保持不变。
pub trait HasTimestamps {
    /// 写入 created_at。
    fn set_created_at(&mut self, dt: DateTime<Utc>);
    /// 写入 updated_at。
    fn set_updated_at(&mut self, dt: DateTime<Utc>);

    /// INSERT 前调用：同时填充 created_at 与 updated_at（对齐 GORM 新建行为）。
    fn before_insert(&mut self) {
        let now = Utc::now();
        self.set_created_at(now);
        self.set_updated_at(now);
    }

    /// UPDATE 前调用：仅刷新 updated_at（对齐 GORM 更新行为）。
    fn before_update(&mut self) {
        self.set_updated_at(Utc::now());
    }
}

// ---------------------------------------------------------------------------
// SoftDelete：对齐 GORM gorm.DeletedAt
// ---------------------------------------------------------------------------

/// 软删除默认过滤条件片段：所有列表 / 按 id 查询默认追加 `deleted_at IS NULL`。
///
/// 用法：`format!("SELECT * FROM users {SOFT_DELETE_WHERE}")`。
pub const SOFT_DELETE_WHERE: &str = "deleted_at IS NULL";

/// 软删除模型应实现本 trait。
///
/// 约定：
/// - 普通查询必须带 [`SOFT_DELETE_WHERE`]；
/// - 需要查看回收站时使用 `include_deleted = true`（对应 GORM 的 `Unscoped()`）；
/// - DELETE 操作改写为 `UPDATE ... SET deleted_at = ?1 WHERE id = ?2 AND deleted_at IS NULL`。
pub trait SoftDelete {
    /// 当前行的软删除时间（None 表示未删除）。
    fn deleted_at(&self) -> Option<DateTime<Utc>>;
    /// 写入软删除时间。
    fn set_deleted_at(&mut self, dt: Option<DateTime<Utc>>);

    /// 是否已被软删除。
    fn is_deleted(&self) -> bool {
        self.deleted_at().is_some()
    }

    /// 把记录标记为已删除（仅修改内存对象；落库由 repository 层执行 UPDATE）。
    fn mark_deleted(&mut self) {
        self.set_deleted_at(Some(Utc::now()));
    }
}

/// 构造软删除 UPDATE 语句（表名由调用方给出，避免注入风险：表名必须是代码常量）。
///
/// 返回 `"UPDATE <table> SET deleted_at = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL"`。
/// 绑定参数顺序：deleted_at(now)、updated_at(now)、id。
pub fn soft_delete_update_sql(table: &'static str) -> String {
    format!(
        "UPDATE {table} SET deleted_at = ?1, updated_at = ?2 \
         WHERE id = ?3 AND {SOFT_DELETE_WHERE}"
    )
}

// ---------------------------------------------------------------------------
// 分页：对齐 Go pkg/response/pagination.go
// ---------------------------------------------------------------------------

/// 分页请求参数（归一化：page >= 1，page_size ∈ [1, 1000]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaginationParams {
    /// 页码，从 1 开始。
    pub page: i64,
    /// 每页条数。
    pub page_size: i64,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 1,
            page_size: 10,
        }
    }
}

impl PaginationParams {
    /// 最大允许的 page_size（放宽至 1000：支撑 386 节点 / 390 GPU 全量列表展示）。
    pub const MAX_PAGE_SIZE: i64 = 1000;

    /// 用原始请求值构造并归一化。
    pub fn new(page: i64, page_size: i64) -> Self {
        Self { page, page_size }.normalize()
    }

    /// 把非法值夹到合法区间：page < 1 → 1；page_size < 1 → 10；page_size > 1000 → 1000。
    pub fn normalize(self) -> Self {
        let page = self.page.max(1);
        let page_size = match self.page_size {
            v if v < 1 => 10,
            v => v.min(Self::MAX_PAGE_SIZE),
        };
        Self { page, page_size }
    }

    /// SQL OFFSET 值。
    pub fn offset(&self) -> i64 {
        (self.page - 1) * self.page_size
    }

    /// SQL LIMIT 值。
    pub fn limit(&self) -> i64 {
        self.page_size
    }
}

/// 计算总页数：`ceil(total / page_size)`，page_size <= 0 时返回 0。
pub fn total_pages(total: i64, page_size: i64) -> i64 {
    if page_size <= 0 || total <= 0 {
        0
    } else {
        (total + page_size - 1) / page_size
    }
}

/// 分页查询结果包装。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaginatedResult<T> {
    /// 当前页数据。
    pub data: Vec<T>,
    /// 满足过滤条件的总条数。
    pub total: i64,
    /// 当前页码。
    pub page: i64,
    /// 每页条数。
    pub page_size: i64,
    /// 总页数。
    pub total_pages: i64,
}

impl<T> PaginatedResult<T> {
    /// 用当前页数据 + 总数 + 请求参数组装结果。
    pub fn new(data: Vec<T>, total: i64, params: PaginationParams) -> Self {
        Self {
            data,
            total,
            page: params.page,
            page_size: params.page_size,
            total_pages: total_pages(total, params.page_size),
        }
    }
}

// ---------------------------------------------------------------------------
// Json<T>：对齐 GORM type:json 字段
// ---------------------------------------------------------------------------

/// 以 TEXT 列存储 JSON 的 newtype 包装。
///
/// - SQLite：落库为 TEXT（UTF-8 JSON 字符串）；
/// - Postgres：未来迁移到 migrations/postgres/ 时改用 JSONB，编解码层不变。
///
/// 配合 `#[derive(FromRow)]` 使用，例如：`pub gpu_vendors: Json<Vec<String>>`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    /// 解包内部值。
    pub fn into_inner(self) -> T {
        self.0
    }

    /// 借用内部值。
    pub fn inner(&self) -> &T {
        &self.0
    }
}

// sqlx 在 SQLite 上把 Json<T> 当作 TEXT 处理：序列化 → String → TEXT；
// 读取时 TEXT → String → 反序列化出 T。
impl<T> Type<Sqlite> for Json<T>
where
    T: Serialize + DeserializeOwned,
{
    fn type_info() -> SqliteTypeInfo {
        <String as Type<Sqlite>>::type_info()
    }
}

impl<'q, T> Encode<'q, Sqlite> for Json<T>
where
    T: Serialize,
{
    fn encode_by_ref(&self, buf: &mut Vec<SqliteArgumentValue<'q>>) -> Result<IsNull, BoxDynError> {
        // 序列化为 JSON 字符串后，委托给 String 的 SQLite 编码器。
        let s = serde_json::to_string(&self.0).expect("Json<T> 序列化失败");
        <String as Encode<Sqlite>>::encode(s, buf)
    }
}

impl<'r, T> Decode<'r, Sqlite> for Json<T>
where
    T: DeserializeOwned,
{
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, BoxDynError> {
        let s: String = <String as Decode<Sqlite>>::decode(value)?;
        Ok(Json(serde_json::from_str(&s)?))
    }
}

// sqlx `json` feature 为 `serde_json::Value` 实现了 Postgres JSONB 编解码。
// Json<T> 委托给 `serde_json::Value`：Type 返回 JSONB OID(3802)，
// Encode 先把 T 序列化为 Value 再写入 JSONB 二进制，Decode 反之。
impl<T> Type<Postgres> for Json<T>
where
    T: Serialize + DeserializeOwned,
{
    fn type_info() -> PgTypeInfo {
        <serde_json::Value as Type<Postgres>>::type_info()
    }
}

impl<'q, T> Encode<'q, Postgres> for Json<T>
where
    T: Serialize,
{
    fn encode_by_ref(&self, buf: &mut PgArgumentBuffer) -> Result<IsNull, BoxDynError> {
        let value = serde_json::to_value(&self.0)?;
        <serde_json::Value as Encode<Postgres>>::encode_by_ref(&value, buf)
    }
}

impl<'r, T> Decode<'r, Postgres> for Json<T>
where
    T: DeserializeOwned,
{
    fn decode(value: PgValueRef<'r>) -> Result<Self, BoxDynError> {
        let v: serde_json::Value = <serde_json::Value as Decode<Postgres>>::decode(value)?;
        let t: T = serde_json::from_value(v)?;
        Ok(Json(t))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn json_type_postgres_oid_is_jsonb() {
        // JSONB OID = 3802；JSON OID = 114。
        // migrations/postgres/ 中 JSON 列均为 JSONB，codec 必须返回 3802。
        let type_info = <Json<serde_json::Value> as Type<Postgres>>::type_info();
        let oid = type_info.oid().expect("JSONB type must have a known OID");
        assert_eq!(
            oid.0, 3802,
            "Json<T> Postgres type must be JSONB (OID 3802)"
        );
    }

    #[test]
    fn json_type_sqlite_is_text() {
        let type_info = <Json<serde_json::Value> as Type<Sqlite>>::type_info();
        assert_eq!(
            type_info,
            <String as Type<Sqlite>>::type_info(),
            "Json<T> SQLite type must be TEXT"
        );
    }

    #[test]
    fn json_postgres_encode_decode_serialization_roundtrip() {
        // 验证编解码逻辑：序列化 T → Value → 反序列化回 T。
        // （真实 PG 二进制读写由 CI postgres_integration_test 验证）
        let mut map = HashMap::new();
        map.insert("region".to_string(), "cn-sh".to_string());
        map.insert("gpu".to_string(), "A100".to_string());
        let original: Json<HashMap<String, String>> = Json(map);

        // 模拟 encode 路径：T → serde_json::Value
        let value = serde_json::to_value(&original.0).expect("serialize T to Value");
        assert_eq!(value["region"], "cn-sh");
        assert_eq!(value["gpu"], "A100");

        // 模拟 decode 路径：serde_json::Value → T
        let decoded: HashMap<String, String> =
            serde_json::from_value(value).expect("deserialize Value back to T");
        assert_eq!(decoded.get("region"), Some(&"cn-sh".to_string()));
        assert_eq!(decoded.get("gpu"), Some(&"A100".to_string()));
    }

    #[test]
    fn json_sqlite_encode_decode_roundtrip() {
        // 验证 SQLite 路径的序列化/反序列化逻辑。
        let vendors: Vec<String> = vec!["nvidia".into(), "amd".into()];
        let original = Json(vendors);

        // SQLite encode: T → JSON String → TEXT
        let s = serde_json::to_string(&original.0).expect("serialize to json string");
        assert!(s.contains("nvidia"));
        assert!(s.contains("amd"));

        // SQLite decode: TEXT → String → T
        let decoded: Vec<String> = serde_json::from_str(&s).expect("deserialize from json string");
        assert_eq!(decoded, vec!["nvidia".to_string(), "amd".to_string()]);
    }
}
