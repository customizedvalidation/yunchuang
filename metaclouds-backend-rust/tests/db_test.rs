//! WP-P1-05 集成测试：双驱动连接池 + 迁移 + 种子数据 + 基础 CRUD。
//!
//! 全部用 SQLite 内存库（单连接）跑；Postgres 分支仅做编译层面验证，
//! 真实 Postgres 联调留给后续 CI（testcontainers）。

use metaclouds_backend_rust::config::Config;
use metaclouds_backend_rust::db::{self, DatabasePool};
use metaclouds_backend_rust::models::user::{self, NewUser};
use metaclouds_backend_rust::TestConfig;
use sqlx::sqlite::SqlitePool;

/// 构造单连接内存 SQLite 池并运行迁移。
async fn memory_pool() -> SqlitePool {
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1) // 内存库必须单连接，否则各连接数据彼此不可见
        .connect("sqlite::memory:")
        .await
        .expect("open in-memory sqlite");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run migrations");
    pool
}

#[tokio::test]
async fn connect_and_migrate_creates_all_tables() {
    let pool = db::connect_and_migrate("sqlite::memory:")
        .await
        .expect("connect_and_migrate");

    // Phase 1 的四张核心表都应存在。
    for table in ["users", "tenants", "clusters", "resources"] {
        let exists: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?1")
                .bind(table)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(exists.0, 1, "table {table} missing after migration");
    }

    // users 表应有 Phase 1 新增的 deleted_at 列。
    let row: (String,) =
        sqlx::query_as("SELECT name FROM pragma_table_info('users') WHERE name = 'deleted_at'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(row.0, "deleted_at");
}

#[tokio::test]
async fn seed_admin_creates_default_tenant_and_admin() {
    let pool = memory_pool().await;
    db::seed_admin_if_empty(&pool).await.expect("seed admin");

    // 默认租户：name='默认租户'，id=1。
    let (name,): (String,) = sqlx::query_as("SELECT name FROM tenants WHERE id = 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(name, "默认租户");

    // admin 用户挂到默认租户。
    let (username, tenant_id, role): (String, i64, String) =
        sqlx::query_as("SELECT username, tenant_id, role FROM users WHERE username = 'admin'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(username, "admin");
    assert_eq!(role, "admin");
    assert_eq!(tenant_id, 1);

    // 再次调用应当幂等，不重复播种。
    db::seed_admin_if_empty(&pool).await.unwrap();
    let (cnt,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(cnt, 1);
}

#[tokio::test]
async fn connect_pool_resolves_sqlite_from_url_scheme() {
    let cfg: Config = TestConfig::default().into();
    // TestConfig 默认 database_url = "sqlite::memory:"，应解析为 SQLite 变体。
    let pool = db::connect_pool(&cfg).await.expect("connect_pool");
    assert!(pool.is_sqlite());
    assert!(pool.as_sqlite().is_some());
}

#[tokio::test]
async fn model_layer_basic_crud_roundtrip() {
    let pool = memory_pool().await;

    // INSERT（走 user::create，自动时间戳）。
    let created = user::create(
        &pool,
        NewUser {
            username: "crud_alice",
            email: "crud_alice@example.com",
            password_hash: "hash",
            role: "user",
            tenant_id: 1,
        },
    )
    .await
    .expect("create user");
    assert!(created.id > 0);

    // SELECT。
    let fetched = user::get_by_id(&pool, created.id, false)
        .await
        .unwrap()
        .expect("fetch user");
    assert_eq!(fetched.username, "crud_alice");

    // UPDATE（touch updated_at）。
    user::touch_updated_at(&pool, created.id).await.unwrap();

    // DELETE（软删除）→ 普通查不到，include_deleted 能查到。
    let deleted = user::soft_delete(&pool, created.id).await.unwrap();
    assert!(deleted);
    assert!(user::get_by_id(&pool, created.id, false)
        .await
        .unwrap()
        .is_none());
    assert!(user::get_by_id(&pool, created.id, true)
        .await
        .unwrap()
        .is_some());
}

#[tokio::test]
async fn run_migrations_on_database_pool_sqlite_variant() {
    // 直接用 DatabasePool::Sqlite 走 run_migrations 路径。
    let raw = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    let pool = DatabasePool::Sqlite(raw);
    db::run_migrations(&pool)
        .await
        .expect("run_migrations via DatabasePool");

    let c: &SqlitePool = pool.as_sqlite().unwrap();
    let (cnt,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tenants")
        .fetch_one(c)
        .await
        .unwrap();
    assert_eq!(cnt, 0);
}
