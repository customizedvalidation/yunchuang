//! Postgres 集成测试（`#[ignore]`，需真实 Postgres）。
//!
//! # 运行方式
//!
//! 本机无 Postgres 时默认跳过。CI 通过 postgres:16 service 容器运行：
//! ```text
//! DATABASE_URL=postgres://postgres:postgres@localhost:5432/test \
//!   cargo test --test postgres_integration_test -- --ignored --nocapture
//! ```
//!
//! # 验证目标
//!
//! 1. `Migrator::new("migrations/postgres/")` 运行时加载并执行全部 9 个 PG 方言迁移；
//! 2. `orm::Json<T>` 的 Postgres codec（JSONB OID=3802）可直接 bind 到 JSONB 列；
//! 3. `query_as` 读回 JSONB 列时 `Json<Vec<String>>` 正确反序列化；
//! 4. 核心 CRUD（users / clusters）在 PG 方言下可读写。
//!
//! 与 `postgres_smoke_test.rs` 的区别：smoke test 内联建表、用 `::jsonb` 字符串转换；
//! 本测试走真实迁移目录 + `Json<T>` codec 直接绑定，验证生产路径。

use metaclouds_backend_rust::orm::Json;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPool;
use std::path::Path;

/// 读取 Postgres 连接串；非 postgres:// 开头时直接返回（本测试已 `#[ignore]`，
/// 这里再做一次防御，避免误在 SQLite 环境手动 `--ignored` 时连接失败 panic）。
fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| u.starts_with("postgres://"))
}

#[tokio::test]
#[ignore = "requires a live Postgres at $DATABASE_URL; run in CI via --ignored"]
async fn postgres_migrations_jsonb_codec_core_crud() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL missing or not postgres://; skipping");
        return;
    };

    let pool = PgPool::connect(&url)
        .await
        .expect("connect to postgres at DATABASE_URL");

    // 1) 运行 Postgres 方言迁移（migrations/postgres/，9 个文件）。
    let migrator = Migrator::new(Path::new("migrations/postgres"))
        .await
        .expect("create Migrator from migrations/postgres/");
    migrator.run(&pool).await.expect("run postgres migrations");

    // 2) 验证核心表已创建。
    for table in ["users", "tenants", "clusters", "resources"] {
        let exists: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM information_schema.tables WHERE table_name = $1")
                .bind(table)
                .fetch_one(&pool)
                .await
                .expect("check table exists");
        assert_eq!(
            exists.0, 1,
            "table {table} missing after postgres migration"
        );
    }

    // 3) 验证 JSONB 列类型：clusters.gpu_vendors 必须是 jsonb。
    let col_type: (String,) = sqlx::query_as(
        "SELECT data_type FROM information_schema.columns \
         WHERE table_name = 'clusters' AND column_name = 'gpu_vendors'",
    )
    .fetch_one(&pool)
    .await
    .expect("check gpu_vendors column type");
    assert_eq!(
        col_type.0, "jsonb",
        "gpu_vendors must be JSONB in postgres migration"
    );

    // 4) 核心 CRUD：插入用户（Postgres $N 占位符）。
    sqlx::query(
        "INSERT INTO users (username, email, password_hash, role, tenant_id) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (username) DO NOTHING",
    )
    .bind("pg_int_admin")
    .bind("pg_int_admin@example.com")
    .bind("hash_placeholder")
    .bind("admin")
    .bind(1_i64)
    .execute(&pool)
    .await
    .expect("insert user");

    let (username, role): (String, String) =
        sqlx::query_as("SELECT username, role FROM users WHERE username = $1")
            .bind("pg_int_admin")
            .fetch_one(&pool)
            .await
            .expect("select user");
    assert_eq!(username, "pg_int_admin");
    assert_eq!(role, "admin");

    // 5) JSONB 读写：插入 cluster，Json<Vec<String>> 直接 bind 到 JSONB 列。
    let gpu_vendors: Json<Vec<String>> = Json(vec!["nvidia".into(), "amd".into()]);
    let scheduler_types: Json<Vec<String>> = Json(vec!["kubernetes".into(), "slurm".into()]);

    sqlx::query(
        "INSERT INTO clusters (name, description, status, gpu_vendors, scheduler_types) \
         VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT (name) DO NOTHING",
    )
    .bind("pg-int-cluster")
    .bind("integration test cluster")
    .bind("active")
    .bind(&gpu_vendors)
    .bind(&scheduler_types)
    .execute(&pool)
    .await
    .expect("insert cluster with Json<T> codec");

    // 6) 读回 JSONB 列：query_as 用 Json<Vec<String>> 反序列化。
    let (name, read_vendors, read_schedulers): (String, Json<Vec<String>>, Json<Vec<String>>) =
        sqlx::query_as("SELECT name, gpu_vendors, scheduler_types FROM clusters WHERE name = $1")
            .bind("pg-int-cluster")
            .fetch_one(&pool)
            .await
            .expect("select cluster with Json<T> codec");

    assert_eq!(name, "pg-int-cluster");
    assert_eq!(
        read_vendors.inner(),
        &vec!["nvidia".to_string(), "amd".to_string()],
        "JSONB roundtrip must preserve gpu_vendors via Json<T> codec"
    );
    assert_eq!(
        read_schedulers.inner(),
        &vec!["kubernetes".to_string(), "slurm".to_string()],
        "JSONB roundtrip must preserve scheduler_types via Json<T> codec"
    );

    // 7) 清理（只删本测试创建的行，不 drop 表——迁移表由 Migrator 管理）。
    sqlx::query("DELETE FROM clusters WHERE name = $1")
        .bind("pg-int-cluster")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM users WHERE username = $1")
        .bind("pg_int_admin")
        .execute(&pool)
        .await
        .unwrap();
}
