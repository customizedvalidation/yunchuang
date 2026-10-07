//! 数据库连接池 + 迁移体系（WP-P1-05，sqlx 双驱动）。
//!
//! Phase 0：仅 SQLite 单驱动 + `001_initial.sql`。
//! Phase 1：在此之上补齐双驱动连接池封装、按驱动选择的迁移运行、默认种子数据。
//!
//! # 与 Go 版的对齐
//!
//! Go 侧 `models/db.go` 的连接池参数：
//! - `SetMaxOpenConns(100)` / `SetMaxIdleConns(20)`
//! - `SetConnMaxLifetime(300s)` / `SetConnMaxIdleTime(60s)`
//! - Postgres DSN：`postgres://user:password@host:port/dbname?sslmode=disable`
//!
//! # 兼容性约束
//!
//! - `connect_and_migrate(url)` / `seed_admin_if_empty(&SqlitePool)` 的签名保持不变，
//!   main.rs 与 lib.rs 继续使用；
//! - [`DatabasePool`] 是新增的双驱动枚举，AppState 的 pool 字段当前仍是
//!   `sqlx::SqlitePool`（位于 src/auth/middleware.rs，属于另一个工作包范围）。
//!   后续整合时，AppState.pool 可直接替换为 `DatabasePool::Sqlite(pool)`。

use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use sqlx::migrate::Migrator;
use sqlx::postgres::PgPool;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

use crate::config::Config;
use crate::error::{AppError, AppResult, ErrorCode};

/// 对齐 Go 的 `SetMaxOpenConns(100)`。
const MAX_OPEN_CONNS: u32 = 100;
/// 对齐 Go 的 `SetMaxIdleConns(20)`（sqlx 没有独立 idle 上限，整体 max_connections 对齐之）。
const MAX_IDLE_CONNS: u32 = 20;
/// 对齐 Go 的 `SetConnMaxLifetime(300s)`。
const CONN_MAX_LIFETIME_SECS: u64 = 300;
/// 对齐 Go 的 `SetConnMaxIdleTime(60s)`。
const CONN_MAX_IDLE_SECS: u64 = 60;
/// Phase 0 既有的 SQLite 常规连接数。
const SQLITE_MAX_CONNS: u32 = 5;

/// 双驱动连接池。
///
/// 现阶段 SQLite 分支是唯一会真正落地数据的实现；Postgres 分支为代码层面预留，
/// 待 Postgres 方言迁移（migrations/postgres/）就绪后切换。
#[derive(Debug, Clone)]
pub enum DatabasePool {
    /// SQLite 池（当前默认）。
    Sqlite(SqlitePool),
    /// Postgres 池（预留）。
    Postgres(PgPool),
}

impl DatabasePool {
    /// 是否为 SQLite 驱动。
    pub fn is_sqlite(&self) -> bool {
        matches!(self, DatabasePool::Sqlite(_))
    }

    /// 取内部的 SQLite 池（Postgres 变体返回 None）。
    pub fn as_sqlite(&self) -> Option<&SqlitePool> {
        match self {
            DatabasePool::Sqlite(p) => Some(p),
            DatabasePool::Postgres(_) => None,
        }
    }

    /// 取内部的 Postgres 池（SQLite 变体返回 None）。
    pub fn as_postgres(&self) -> Option<&PgPool> {
        match self {
            DatabasePool::Postgres(p) => Some(p),
            DatabasePool::Sqlite(_) => None,
        }
    }
}

/// 把 SQLite 风格的位置占位符 `?N` 改写为 Postgres 风格的 `$N`。
///
/// 仅匹配 `?` 紧跟数字（`?1`、`?12` …），不动字符串字面量里可能出现的 `?`
/// （若 SQL 真有字面量 `?5`，SQLite 也会把它当成参数而报参数不匹配，故可安全改写）。
/// SQLite 分支直接用原文，零拷贝；Postgres 分支走本函数。
pub fn pg_placeholders(sql: &str) -> String {
    let bytes = sql.as_bytes();
    let mut out = String::with_capacity(sql.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'?' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            out.push('$');
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                out.push(bytes[i] as char);
                i += 1;
            }
        } else {
            // 复制当前字节（SQL 为 ASCII 子集，逐字节安全）。
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// 渲染 SQL：SQLite 原样返回；Postgres 把 `?N` 改写为 `$N`。
///
/// 既可作为 `DatabasePool` 的方法调用，也可被宏在编译期按驱动分支选择，
/// 保证 SQLite 路径与改动前逐字节一致（从而 143 项 lib 测试不受影响）。
impl DatabasePool {
    pub fn render_sql<'a>(&self, sql: &'a str) -> std::borrow::Cow<'a, str> {
        match self {
            DatabasePool::Sqlite(_) => std::borrow::Cow::Borrowed(sql),
            DatabasePool::Postgres(_) => std::borrow::Cow::Owned(pg_placeholders(sql)),
        }
    }
}

/// 双驱动 SQL 执行宏（请求层方言移植的核心）。
///
/// 用法：`with_db!(&pool, SQL_EXPR, |sql, p| { sqlx::query_as::<_, T>(sql)....fetch_X(p).await? })`
///
/// - `sql`：渲染后的 `&str`（`?N` → `$N` 仅对 Postgres 生效）。
/// - `p`：具体驱动连接池（`&SqlitePool` / `&PgPool`），由宏按 `&pool` 的变体拆分，
///   sqlx 据此推断数据库类型，故无需为两库写两份 SQL。
/// - SQLite 分支与改动前完全相同，确保零回归。
#[macro_export]
macro_rules! with_db {
    ($pool:expr, $db_sql:expr, |$db_s:ident, $db_e:ident| $body:expr) => {{
        let __db_pool = $pool;
        match __db_pool {
            $crate::db::DatabasePool::Sqlite($db_e) => {
                let $db_s: &str = $db_sql;
                $body
            }
            $crate::db::DatabasePool::Postgres($db_e) => {
                let __pg_sql = $crate::db::pg_placeholders($db_sql);
                let $db_s: &str = &__pg_sql;
                $body
            }
        }
    }};
}

/// 双驱动 INSERT 取自增 id 宏（替代 SQLite 专属的 `last_insert_rowid()`）。
///
/// - SQLite：`.execute()` 后 `.last_insert_rowid()`；
/// - Postgres：`... RETURNING id` + `fetch_one::<(i64,)>()` 取 `.0`。
///
/// `$binds` 为以 `q` 开头的 `.bind(...).bind(...)` 链（如 `q.bind(a).bind(b)`）。
#[macro_export]
macro_rules! insert_id {
    ($pool:expr, $db_sql:expr, |$q:ident| $binds:expr) => {{
        let __db_pool = $pool;
        match __db_pool {
            $crate::db::DatabasePool::Sqlite(p) => {
                let $q = ::sqlx::query($db_sql);
                let __b = $binds;
                let __res = __b.execute(p).await?;
                __res.last_insert_rowid()
            }
            $crate::db::DatabasePool::Postgres(p) => {
                let __pg_sql = format!("{} RETURNING id", $crate::db::pg_placeholders($db_sql));
                let $q = $crate::db::query_as_db::<_, (i64,), _>(p, &__pg_sql);
                let __b = $binds;
                let __row = __b.fetch_one(p).await?;
                __row.0
            }
        }
    }};
}

/// 双驱动事务宏：按驱动开启并提交一个事务，体内 `tx` 为具体的事务对象。
#[macro_export]
macro_rules! with_tx {
    ($pool:expr, |$tx:ident| $body:expr) => {{
        let __db_pool = $pool;
        match __db_pool {
            $crate::db::DatabasePool::Sqlite(p) => {
                let mut $tx = p.begin().await?;
                let __r = $body;
                $tx.commit().await?;
                __r
            }
            $crate::db::DatabasePool::Postgres(p) => {
                let mut $tx = p.begin().await?;
                let __r = $body;
                $tx.commit().await?;
                __r
            }
        }
    }};
}

/// 事务内的双驱动 SQL 执行宏（executor 为具体事务 `&mut *tx`）。
///
/// `$tx` 是 `with_tx!` 提供的具体事务表达式（如 `&mut *tx`）；方言仍由 `$pool` 决定。
#[macro_export]
macro_rules! with_db_tx {
    ($pool:expr, $tx:expr, $db_sql:expr, |$db_s:ident, $db_e:ident| $body:expr) => {{
        let __db_pool = $pool;
        let $db_e = $tx;
        let __db_sql: std::borrow::Cow<'_, str> = match __db_pool {
            $crate::db::DatabasePool::Sqlite(_) => std::borrow::Cow::Borrowed($db_sql),
            $crate::db::DatabasePool::Postgres(_) => {
                std::borrow::Cow::Owned($crate::db::pg_placeholders($db_sql))
            }
        };
        let $db_s: &str = &__db_sql;
        $body
    }};
}

/// 事务内的双驱动 INSERT 取自增 id 宏（executor 为具体事务 `&mut *tx`）。
///
/// 注意：本宏始终在 `with_tx!` 内部展开，`$tx` 已是单一驱动的具体事务类型
/// （`&mut SqliteTransaction` 或 `&mut PgTransaction`），无法再按 `&pool` 做两套
/// 不同执行路径（否则非匹配臂会因 executor 类型不符而编译失败，例如 SQLite 臂对
/// `PgTransaction` 调用 `last_insert_rowid()`）。因此这里统一采用
/// `RETURNING id` + `fetch_one::<(i64,)>` 的方式——SQLite 3.35+ 与 Postgres 均支持
/// `RETURNING`，驱动差异仅靠占位符 `?N`→`$N` 与执行器类型区分。
#[macro_export]
macro_rules! insert_id_tx {
    ($pool:expr, $tx:expr, $db_sql:expr, |$q:ident| $binds:expr) => {{
        let __db_pool = $pool;
        let __tx = $tx;
        let __sql: String = match __db_pool {
            $crate::db::DatabasePool::Sqlite(_) => {
                format!("{} RETURNING id", $db_sql)
            }
            $crate::db::DatabasePool::Postgres(_) => {
                format!("{} RETURNING id", $crate::db::pg_placeholders($db_sql))
            }
        };
        let $q = $crate::db::query_as_db::<_, (i64,), _>(&mut *__tx, &__sql);
        let __b = $binds;
        let __row = __b.fetch_one(__tx).await?;
        __row.0
    }};
}

/// 双驱动安全版的 `sqlx::query_as`：借 executor 固定数据库类型，消除
/// `query_as::<_, O>` 在 `Sqlite` / `Postgres` 两套 `FromRow` 实现之间的推断歧义
/// （否则 `with_db!` 两臂的 `impl Future` 类型不兼容，无法统一，编译失败）。
///
/// `executor` 仅用于类型推断（不实际消费），DB 会据此解析为 `Sqlite` 或 `Postgres`，
/// 与 `O: FromRow<DB>` 自动对齐。
pub fn query_as_db<'q, 'c, DB, O, E>(
    _executor: E,
    sql: &'q str,
) -> sqlx::query::QueryAs<'q, DB, O, <DB as sqlx::Database>::Arguments<'q>>
where
    DB: sqlx::Database,
    O: for<'r> sqlx::FromRow<'r, DB::Row>,
    E: sqlx::Executor<'c, Database = DB>,
    usize: sqlx::ColumnIndex<<DB as sqlx::Database>::Row>,
{
    sqlx::query_as::<DB, O>(sql)
}

/// 双驱动安全版的 `sqlx::query_scalar`：同上，用 executor 固定数据库类型。
pub fn query_scalar_db<'q, 'c, DB, T, E>(
    _executor: E,
    sql: &'q str,
) -> sqlx::query::QueryScalar<'q, DB, T, <DB as sqlx::Database>::Arguments<'q>>
where
    DB: sqlx::Database,
    T: for<'r> sqlx::Decode<'r, DB> + sqlx::Type<DB>,
    E: sqlx::Executor<'c, Database = DB>,
    usize: sqlx::ColumnIndex<<DB as sqlx::Database>::Row>,
{
    sqlx::query_scalar::<DB, T>(sql)
}

/// DSN 是否为 Postgres 连接串：`postgres://...` 与 `postgresql://...` 均识别
/// （docker-compose 下发的是 `postgresql://...`，sqlx 两种 scheme 都接受）。
pub fn is_postgres_url(url: &str) -> bool {
    let u = url.trim_start();
    u.starts_with("postgres://") || u.starts_with("postgresql://")
}

/// 根据 DSN scheme 判断驱动：`postgres(s)://...` → Postgres，其余一律按 SQLite 处理。
pub fn is_sqlite_url(url: &str) -> bool {
    !is_postgres_url(url)
}

/// 启动路径是否应走 Postgres 驱动：
/// `USE_SQLITE=false`（[`Config::use_sqlite`]）或 `DATABASE_URL` 为 `postgres(s)://` 即走 Postgres。
///
/// 此前 main.rs 无条件调 [`connect_and_migrate`]（只连 SQLite），导致 docker-compose
/// 下发 `postgresql://...` 时被静默忽略、错误地按 SQLite 打开了一个同名文件——本函数
/// 让启动路径真正按配置选择驱动。
pub fn wants_postgres(config: &Config) -> bool {
    !config.use_sqlite || is_postgres_url(&config.database_url)
}

/// 连接池工厂：按 Go `InitDB` 的三分支语义创建对应驱动的池。
///
/// - `memory_store_enabled = true`：走 SQLite 内存库（对齐 Go “内存存储模式”，
///   多连接池下数据不共享，自动降级为单连接）；
/// - 否则 `use_sqlite = true`：走 `database_url` 指向的 SQLite 文件；
/// - 否则：走 Postgres，DSN 由 [`Config::get_database_dsn`] 按
///   `postgres://user:password@host:port/dbname?sslmode=disable` 语义拼装。
pub async fn connect_pool(config: &Config) -> AppResult<DatabasePool> {
    if config.memory_store_enabled {
        tracing::info!("database: using in-memory sqlite store");
        let pool = connect_sqlite("sqlite::memory:").await?;
        return Ok(DatabasePool::Sqlite(pool));
    }

    if config.use_sqlite {
        tracing::info!(url = %config.database_url, "database: using sqlite file");
        let pool = connect_sqlite(&config.database_url).await?;
        return Ok(DatabasePool::Sqlite(pool));
    }

    // 优先使用显式 DATABASE_URL（docker-compose 下发完整 postgresql:// 连接串）；
    // 否则按 host/port/user/password/db 字段拼装 libpq DSN（对齐 Go GetDatabaseDSN）。
    let dsn = if is_postgres_url(&config.database_url) {
        tracing::info!(url = %config.database_url, "database: using postgres (explicit DATABASE_URL)");
        config.database_url.clone()
    } else {
        tracing::info!(host = %config.database_host, db = %config.database_name, "database: using postgres");
        config.get_database_dsn()
    };
    let pool = connect_postgres(&dsn).await?;
    Ok(DatabasePool::Postgres(pool))
}

/// 按 url 创建 SQLite 池。内存模式自动单连接。
async fn connect_sqlite(url: &str) -> AppResult<SqlitePool> {
    let is_memory = url.contains(":memory:");
    let options = SqliteConnectOptions::from_str(url)
        .map_err(|e| {
            AppError::with_source(ErrorCode::InternalServerError, "invalid DATABASE_URL", e)
        })?
        .create_if_missing(true);

    // 内存库在多连接池下彼此不可见（每个连接一份私有内存库），
    // 因此内存模式强制单连接；文件库保持 Phase 0 的 5 连接。
    let max_connections = if is_memory { 1 } else { SQLITE_MAX_CONNS };

    sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(10))
        .connect_with(options)
        .await
        .map_err(|e| {
            AppError::with_source(
                ErrorCode::InternalServerError,
                "failed to connect to database",
                e,
            )
        })
}

/// 按 Go 的 `postgres://user:password@host:port/dbname?sslmode=disable` 创建 Postgres 池。
///
/// 对齐 Go `sqlDB.SetMaxOpenConns(100)` 等连接池参数。
async fn connect_postgres(url: &str) -> AppResult<PgPool> {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(MAX_OPEN_CONNS)
        .min_connections(MAX_IDLE_CONNS.min(MAX_OPEN_CONNS))
        .max_lifetime(Some(Duration::from_secs(CONN_MAX_LIFETIME_SECS)))
        .idle_timeout(Some(Duration::from_secs(CONN_MAX_IDLE_SECS)))
        .acquire_timeout(Duration::from_secs(10))
        .connect(url)
        .await
        .map_err(|e| {
            AppError::with_source(
                ErrorCode::InternalServerError,
                "failed to connect to PostgreSQL",
                e,
            )
        })
}

/// 根据 `DATABASE_URL` 的 scheme 选择迁移目录。
///
/// - `postgres://...` → `migrations/postgres`（Postgres 方言：BIGSERIAL / TIMESTAMPTZ / JSONB）
/// - 其余（`sqlite://...`、`sqlite::memory:`、空串等）→ `migrations`（SQLite 方言）
///
/// 这是一个纯函数，不触碰文件系统，便于单元测试。
pub fn resolve_migration_dir(database_url: &str) -> &'static str {
    if is_postgres_url(database_url) {
        "migrations/postgres"
    } else {
        "migrations"
    }
}

/// 对双驱动池运行迁移。
///
/// 按池变体选择迁移目录：
/// - [`DatabasePool::Sqlite`] → `migrations/`（SQLite 方言，与 Phase 0 行为一致）
/// - [`DatabasePool::Postgres`] → `migrations/postgres/`（Postgres 方言）
///
/// 使用运行时 [`Migrator::new`] 从文件系统加载迁移脚本（编译时宏 `sqlx::migrate!`
/// 只能嵌入单一目录，无法按驱动切换）。
pub async fn run_migrations(pool: &DatabasePool) -> Result<(), sqlx::migrate::MigrateError> {
    let dir = match pool {
        DatabasePool::Sqlite(_) => "migrations",
        DatabasePool::Postgres(_) => "migrations/postgres",
    };
    let migrator = Migrator::new(Path::new(dir)).await?;
    match pool {
        DatabasePool::Sqlite(p) => migrator.run(p).await,
        DatabasePool::Postgres(p) => migrator.run(p).await,
    }
}

// ---------------------------------------------------------------------------
// 以下为 Phase 0 兼容入口（main.rs / lib.rs / 集成测试仍在使用）
// ---------------------------------------------------------------------------

/// 连接到 `url` 描述的 SQLite 数据库，运行待执行迁移。
///
/// 与 Phase 0 行为保持一致：返回 `SqlitePool`，main.rs 继续直接使用。
/// 迁移目录由 [`resolve_migration_dir`] 按 url scheme 决定（sqlite url → `migrations/`）。
pub async fn connect_and_migrate(url: &str) -> AppResult<SqlitePool> {
    let pool = connect_sqlite(url).await?;

    let dir = resolve_migration_dir(url);
    let migrator = Migrator::new(Path::new(dir)).await.map_err(|e| {
        AppError::with_source(
            ErrorCode::InternalServerError,
            "failed to create migrator",
            e,
        )
    })?;
    migrator.run(&pool).await.map_err(|e| {
        AppError::with_source(
            ErrorCode::InternalServerError,
            "database migration failed",
            e,
        )
    })?;

    Ok(pool)
}

/// 若 `users` 表为空则播种默认管理员，并对齐 Go `InitData`：
/// `tenants` 表为空时先播种默认租户，admin 挂到该租户下。
pub async fn seed_admin_if_empty(pool: &SqlitePool) -> AppResult<()> {
    // 1) 确保默认租户存在（Go init_data.go：默认租户 id=1，名称“默认租户”）。
    let tenant_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenants")
        .fetch_one(pool)
        .await?;
    let admin_tenant_id = if tenant_count == 0 {
        let now = chrono::Utc::now();
        let res = sqlx::query(
            "INSERT INTO tenants (created_at, updated_at, name, description, status, \
             gpu_quota, cpu_quota, memory_quota, storage_quota) \
             VALUES (?1, ?2, '默认租户', '系统默认租户', 'active', 10, 100, 1000, 10000)",
        )
        .bind(now)
        .bind(now)
        .execute(pool)
        .await?;
        res.last_insert_rowid()
    } else {
        sqlx::query_scalar("SELECT COALESCE(MIN(id), 1) FROM tenants")
            .fetch_one(pool)
            .await?
    };

    // 2) 确保默认管理员存在。
    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if user_count > 0 {
        return Ok(());
    }

    // 播种口令走 bootstrap_password：env 已设且 ≥12 字符用 env；生产未设则 Err
    // （fail-secure 拒绝启动）；非生产未设回退 DEFAULT_DEV_ADMIN_PASSWORD。
    let admin_password =
        crate::models::bootstrap_credentials::bootstrap_password("DEFAULT_ADMIN_PASSWORD", "admin")
            .map_err(AppError::bad_request)?;
    let hash = crate::auth::password::hash_password(&admin_password)?;
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) \
         VALUES (?1, ?2, 'admin', 'admin@example.com', ?3, 'admin', ?4)",
    )
    .bind(now)
    .bind(now)
    .bind(hash)
    .bind(admin_tenant_id)
    .execute(pool)
    .await?;
    tracing::info!("seeded default admin user");
    Ok(())
}

/// Postgres 方言的播种：与 [`seed_admin_if_empty`] 语义一致，占位符用 `$N`、
/// 自增 id 用 `RETURNING id`。仅在 Postgres 启动路径调用。
pub async fn seed_admin_if_empty_postgres(pool: &PgPool) -> AppResult<()> {
    let tenant_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenants")
        .fetch_one(pool)
        .await?;
    let admin_tenant_id: i64 = if tenant_count == 0 {
        let now = chrono::Utc::now();
        sqlx::query_scalar(
            "INSERT INTO tenants (created_at, updated_at, name, description, status, \
             gpu_quota, cpu_quota, memory_quota, storage_quota) \
             VALUES ($1, $2, '默认租户', '系统默认租户', 'active', 10, 100, 1000, 10000) \
             RETURNING id",
        )
        .bind(now)
        .bind(now)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar("SELECT COALESCE(MIN(id), 1) FROM tenants")
            .fetch_one(pool)
            .await?
    };

    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await?;
    if user_count > 0 {
        return Ok(());
    }

    // 播种口令走 bootstrap_password：env 已设且 ≥12 字符用 env；生产未设则 Err
    // （fail-secure 拒绝启动）；非生产未设回退 DEFAULT_DEV_ADMIN_PASSWORD。
    let admin_password =
        crate::models::bootstrap_credentials::bootstrap_password("DEFAULT_ADMIN_PASSWORD", "admin")
            .map_err(AppError::bad_request)?;
    let hash = crate::auth::password::hash_password(&admin_password)?;
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) \
         VALUES ($1, $2, 'admin', 'admin@example.com', $3, 'admin', $4)",
    )
    .bind(now)
    .bind(now)
    .bind(hash)
    .bind(admin_tenant_id)
    .execute(pool)
    .await?;
    tracing::info!("seeded default admin user (postgres)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_migration_dir_postgres_url_with_auth() {
        assert_eq!(
            resolve_migration_dir("postgres://user:pass@localhost:5432/test"),
            "migrations/postgres"
        );
    }

    #[test]
    fn resolve_migration_dir_postgres_url_no_auth() {
        assert_eq!(
            resolve_migration_dir("postgres://localhost:5432/test"),
            "migrations/postgres"
        );
    }

    #[test]
    fn resolve_migration_dir_postgres_url_trim_whitespace() {
        assert_eq!(
            resolve_migration_dir("  postgres://localhost/db"),
            "migrations/postgres"
        );
    }

    #[test]
    fn resolve_migration_dir_sqlite_file_url() {
        assert_eq!(
            resolve_migration_dir("sqlite://metaclouds.db"),
            "migrations"
        );
    }

    #[test]
    fn resolve_migration_dir_sqlite_memory_url() {
        assert_eq!(resolve_migration_dir("sqlite::memory:"), "migrations");
    }

    #[test]
    fn resolve_migration_dir_empty_url_defaults_sqlite() {
        assert_eq!(resolve_migration_dir(""), "migrations");
    }

    #[test]
    fn resolve_migration_dir_unknown_scheme_defaults_sqlite() {
        assert_eq!(resolve_migration_dir("mysql://localhost/db"), "migrations");
    }

    #[test]
    fn is_postgres_url_recognizes_both_schemes() {
        assert!(is_postgres_url("postgres://user:pass@localhost:5432/db"));
        // docker-compose 实际下发的 scheme。
        assert!(is_postgres_url(
            "postgresql://metaclouds_user:pw@postgres:5432/metaclouds?sslmode=disable"
        ));
        assert!(is_postgres_url("  postgres://localhost/db"));
        assert!(!is_postgres_url("sqlite::memory:"));
        assert!(!is_postgres_url("sqlite://metaclouds.db"));
        assert!(!is_postgres_url(""));
    }

    #[test]
    fn wants_postgres_default_config_is_false() {
        // 默认 USE_SQLITE=true + sqlite::memory: → 必须保持 SQLite 启动路径。
        let cfg = Config::default();
        assert!(!wants_postgres(&cfg));
    }

    #[test]
    fn wants_postgres_triggered_by_flag_or_dsn() {
        // USE_SQLITE=false 即走 Postgres。
        let cfg = Config {
            use_sqlite: false,
            ..Config::default()
        };
        assert!(wants_postgres(&cfg));

        // 仅 DATABASE_URL 为 postgresql:// 也走 Postgres（compose 下发形态）。
        let cfg = Config {
            database_url: "postgresql://u:p@postgres:5432/metaclouds".to_string(),
            ..Config::default()
        };
        assert!(wants_postgres(&cfg));
    }
}
