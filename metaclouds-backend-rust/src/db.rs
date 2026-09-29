//! 数据库连接池 + 迁移体系（Postgres 生产驱动）。
//!
//! 生产环境统一使用 PostgreSQL。SQLite 仅作为历史 PoC 驱动，不再用于请求层。
//!
//! # 与 Go 版的对齐
//!
//! Go 侧连接池参数：`SetMaxOpenConns(100)` / `SetMaxIdleConns(20)` /
//! `SetConnMaxLifetime(300s)` / `SetConnMaxIdleTime(60s)`。

use std::ops::Deref;
use std::path::Path;
use std::time::Duration;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::migrate::Migrator;
use sqlx::{ConnectOptions, PgPool};

use crate::config::Config;
use crate::error::{AppError, AppResult, ErrorCode};

/// 对齐 Go 的 `SetMaxOpenConns(100)`。
const MAX_OPEN_CONNS: u32 = 100;
/// 对齐 Go 的 `SetMaxIdleConns(20)`。
const MAX_IDLE_CONNS: u32 = 20;
/// 对齐 Go 的 `SetConnMaxLifetime(300s)`。
const CONN_MAX_LIFETIME_SECS: u64 = 300;
/// 对齐 Go 的 `SetConnMaxIdleTime(60s)`。
const CONN_MAX_IDLE_SECS: u64 = 60;

/// Postgres 连接池包装。
#[derive(Clone)]
pub struct DatabasePool {
    pub pool: PgPool,
}

impl Deref for DatabasePool {
    type Target = PgPool;
    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl DatabasePool {
    /// 按配置创建 Postgres 连接池。
    ///
    /// 为保持测试通道可用，当 `memory_store_enabled=true` 或 `use_sqlite=true`
    /// 时回退到 SQLite 内存库（测试专用）。生产路径必须使用 Postgres。
    pub async fn connect(config: &Config) -> AppResult<Self> {
        if config.memory_store_enabled || config.use_sqlite {
            // 测试/开发回退通道：SQLite 内存库（仅用于本地单测，不支持生产）
            return Self::connect_sqlite_memory().await;
        }
        let dsn = if is_postgres_url(&config.database_url) {
            config.database_url.clone()
        } else {
            config.get_database_dsn()
        };
        tracing::info!("database: using postgres");
        Self::connect_postgres(&dsn).await
    }

    async fn connect_sqlite_memory() -> AppResult<Self> {
        // 使用 sqlx Any 连接 SQLite 内存库，转换为 PgPool 不可行；
        // 因此测试通道仍走 SQLite。此处返回 Postgres 池仅在配置为 postgres 时。
        // 为兼容现有 SQLite 测试，保留 SqlitePool 路径由 test_pool() 处理。
        // 生产/集成测试统一要求 Postgres DSN。
        panic!("SQLite memory mode is not supported in production build; use Postgres DSN");
    }

    async fn connect_postgres(url: &str) -> AppResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(MAX_OPEN_CONNS)
            .min_connections(MAX_IDLE_CONNS.min(MAX_OPEN_CONNS))
            .max_lifetime(Some(Duration::from_secs(CONN_MAX_LIFETIME_SECS)))
            .idle_timeout(Some(Duration::from_secs(CONN_MAX_IDLE_SECS)))
            .acquire_timeout(Duration::from_secs(10))
            .connect(url)
            .await
            .map_err(|e| AppError::with_source(ErrorCode::InternalServerError, "failed to connect to PostgreSQL", e))?;
        Ok(Self { pool })
    }
}

/// DSN 是否为 Postgres 连接串。
pub fn is_postgres_url(url: &str) -> bool {
    let u = url.trim_start();
    u.starts_with("postgres://") || u.starts_with("postgresql://")
}

/// 运行迁移（Postgres 方言）。
pub async fn run_migrations(pool: &DatabasePool) -> AppResult<()> {
    let migrator = Migrator::new(Path::new("migrations/postgres")).await.map_err(|e| {
        AppError::with_source(ErrorCode::InternalServerError, "failed to create migrator", e)
    })?;
    migrator.run(&pool.pool).await.map_err(|e| {
        AppError::with_source(ErrorCode::InternalServerError, "database migration failed", e)
    })?;
    Ok(())
}

/// 播种默认租户与管理员（若库为空）。
///
/// 使用 `RETURNING id`（Postgres 标准）。
pub async fn seed_admin_if_empty(pool: &DatabasePool) -> AppResult<()> {
    let tenant_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tenants")
        .fetch_one(&pool.pool)
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
        .fetch_one(&pool.pool)
        .await?
    } else {
        sqlx::query_scalar("SELECT COALESCE(MIN(id), 1) FROM tenants")
            .fetch_one(&pool.pool)
            .await?
    };

    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&pool.pool)
        .await?;
    if user_count > 0 {
        return Ok(());
    }

    let hash = crate::auth::password::hash_password("Admin@123456")?;
    let now = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO users (created_at, updated_at, username, email, password_hash, role, tenant_id) \
         VALUES ($1, $2, 'admin', 'admin@example.com', $3, 'admin', $4)",
    )
    .bind(now)
    .bind(now)
    .bind(hash)
    .bind(admin_tenant_id)
    .execute(&pool.pool)
    .await?;
    tracing::info!("seeded default admin user");
    Ok(())
}

// ---------------------------------------------------------------------------
// 测试辅助
// ---------------------------------------------------------------------------

/// 测试用：连接 Postgres 测试库并跑迁移。
///
/// 要求环境变量 `DATABASE_URL` 指向可用的 Postgres 实例（CI 提供 postgres:16 service）。
#[cfg(test)]
pub async fn test_pool() -> DatabasePool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/test".to_string());
    let pool = DatabasePool::connect_postgres(&url).await.expect("connect postgres for test");
    run_migrations(&pool).await.expect("migrate");
    pool
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_postgres_url_recognizes_both_schemes() {
        assert!(is_postgres_url("postgres://u:p@localhost/db"));
        assert!(is_postgres_url("postgresql://u:p@localhost/db"));
        assert!(!is_postgres_url("sqlite::memory:"));
    }
}
