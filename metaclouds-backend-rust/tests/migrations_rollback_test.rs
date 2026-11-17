//! 回滚 SQL 的离线验证：在内存 SQLite 上跑 up → down → 重新 up。
//!
//! 目的仅是证明 `scripts/sql/rollback.sqlite.sql` 可执行且回滚干净，
//! 不代表生产已演练；真实回滚仍需先备份并用 `scripts/rollback-migrations.sh`。
//! PostgreSQL 的 down SQL 需在 CI 的 dual-driver job 或生产预发环境验证。

use sqlx::sqlite::SqlitePoolOptions;

const DOMAIN_TABLES_SQL: &str = "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' \
     AND name NOT LIKE 'sqlite_%' AND name != '_sqlx_migrations'";

#[tokio::test]
async fn test_sqlite_down_migrations_are_clean() {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("connect sqlite memory");

    // 1) 正常 up。
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("run up migrations");

    let before: i64 = sqlx::query_scalar(DOMAIN_TABLES_SQL)
        .fetch_one(&pool)
        .await
        .expect("count tables before rollback");
    assert!(before > 0, "up migrations must create domain tables");

    // 2) 完整 down（文件内段落已按版本降序排列）。
    let down = include_str!("../../scripts/sql/rollback.sqlite.sql");
    sqlx::raw_sql(down)
        .execute(&pool)
        .await
        .expect("run down migrations");

    let after: i64 = sqlx::query_scalar(DOMAIN_TABLES_SQL)
        .fetch_one(&pool)
        .await
        .expect("count tables after rollback");
    assert_eq!(
        after, 0,
        "full rollback must drop every domain table, found {after}"
    );

    // 3) 清空 SQLx 元数据后重新 up，证明回滚未留下脏状态、可再次迁移。
    sqlx::query("DELETE FROM _sqlx_migrations")
        .execute(&pool)
        .await
        .expect("clear migration metadata");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("re-run up migrations after rollback");

    let again: i64 = sqlx::query_scalar(DOMAIN_TABLES_SQL)
        .fetch_one(&pool)
        .await
        .expect("count tables after re-up");
    assert_eq!(again, before, "re-up must recreate the same table count");
}
