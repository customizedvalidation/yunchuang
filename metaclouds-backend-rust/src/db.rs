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
}

/// 根据 DSN scheme 判断驱动：`postgres://...` → Postgres，其余一律按 SQLite 处理。
///
/// 说明：当前 `config::Config` 尚未包含 `use_sqlite` / `memory_store_enabled` 字段
/// （由后续整合统一接入），因此从 `DATABASE_URL` 的 scheme 推断；
/// 若 url 中出现 `:memory:`，按内存模式对待。
pub fn is_sqlite_url(url: &str) -> bool {
    !url.trim_start().starts_with("postgres://")
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

    let dsn = config.get_database_dsn();
    tracing::info!(host = %config.database_host, db = %config.database_name, "database: using postgres");
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
    if database_url.trim_start().starts_with("postgres://") {
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

    let hash = crate::auth::password::hash_password("Admin@123456")?;
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
}
