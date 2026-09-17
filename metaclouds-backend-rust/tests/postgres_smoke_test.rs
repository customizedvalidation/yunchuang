//! P3-01 Postgres 双驱动冒烟测试（`#[ignore]`，需真实 Postgres）。
//!
//! # 运行方式
//!
//! 本机无 Postgres 时默认跳过。CI 通过 postgres:16 service 容器运行：
//! ```text
//! DATABASE_URL=postgres://postgres:postgres@localhost:5432/test \
//!   cargo test --test postgres_smoke_test -- --ignored --nocapture
//! ```
//!
//! # 验证目标
//!
//! SQLite 与 Postgres 在 JSON 字段、时间戳、软删除、UPSERT 语法上有差异。
//! 本测试内联建一张 Postgres 原生类型小表（BIGSERIAL / JSONB / TIMESTAMPTZ），
//! 不走 `migrations/`（那是 SQLite 方言），专门验证以下关键路径：
//! - JSONB 字段写入与读回（用 `$2::jsonb` 绑定 + `payload::text` 读回，
//!   避免依赖 sqlx 的 `json` feature）；
//! - TIMESTAMPTZ 时间戳往返；
//! - 软删除（`deleted_at` 置位 + 普通过滤 / 含已删除两条路径）；
//! - UPSERT（`ON CONFLICT ... DO UPDATE`，Postgres 特有，区别于 SQLite
//!   的 `INSERT OR REPLACE`）。

use sqlx::PgPool;

/// 读取 Postgres 连接串；非 postgres:// 开头时直接返回（本测试已 `#[ignore]`，
/// 这里再做一次防御，避免误在 SQLite 环境手动 `--ignored` 时连接失败 panic）。
fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .filter(|u| u.starts_with("postgres://"))
}

#[tokio::test]
#[ignore = "requires a live Postgres at $DATABASE_URL; run in CI via --ignored"]
async fn postgres_connect_json_timestamp_softdelete_upsert() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL missing or not postgres://; skipping");
        return;
    };

    let pool = PgPool::connect(&url)
        .await
        .expect("connect to postgres at DATABASE_URL");

    // 内联建表：Postgres 原生类型。
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS p3_smoke (
            id         BIGSERIAL PRIMARY KEY,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
            deleted_at TIMESTAMPTZ NULL,
            name       TEXT NOT NULL UNIQUE,
            payload    JSONB NOT NULL DEFAULT '{}'::jsonb
        )",
    )
    .execute(&pool)
    .await
    .expect("create p3_smoke table");

    // 1) INSERT：JSONB 以字符串绑定 + ::jsonb 转换（绕开 sqlx json feature）。
    sqlx::query("INSERT INTO p3_smoke (name, payload) VALUES ($1, $2::jsonb)")
        .bind("smoke-alice")
        .bind(r#"{"region":"cn-sh","gpu":"A100"}"#)
        .execute(&pool)
        .await
        .expect("insert row");

    // 2) SELECT：JSONB 用 ::text 读出，时间戳用 DateTime<Utc> 往返。
    let row: (i64, String, String, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as("SELECT id, name, payload::text, created_at FROM p3_smoke WHERE name = $1")
            .bind("smoke-alice")
            .fetch_one(&pool)
            .await
            .expect("select row");
    assert!(row.0 > 0, "BIGSERIAL id must be positive");
    assert_eq!(row.1, "smoke-alice");
    let payload: serde_json::Value = serde_json::from_str(&row.2).expect("payload is valid json");
    assert_eq!(payload["region"], "cn-sh");
    assert_eq!(payload["gpu"], "A100");
    // created_at 是 TIMESTAMPTZ，往返成功即验证时间戳兼容。
    assert!(row.3.timestamp() > 0);

    // 3) UPDATE：touch updated_at + 改 JSON。
    sqlx::query("UPDATE p3_smoke SET updated_at = now(), payload = $2::jsonb WHERE name = $1")
        .bind("smoke-alice")
        .bind(r#"{"region":"cn-sh","gpu":"H100"}"#)
        .execute(&pool)
        .await
        .expect("update row");

    let (gpu_text,): (String,) =
        sqlx::query_as("SELECT payload::text FROM p3_smoke WHERE name = $1")
            .bind("smoke-alice")
            .fetch_one(&pool)
            .await
            .unwrap();
    let gpu: serde_json::Value = serde_json::from_str(&gpu_text).unwrap();
    assert_eq!(gpu["gpu"], "H100", "UPSERT/UPDATE should overwrite JSONB");

    // 4) 软删除：置 deleted_at；普通查询过滤，include-deleted 仍可见。
    sqlx::query("UPDATE p3_smoke SET deleted_at = now() WHERE name = $1")
        .bind("smoke-alice")
        .execute(&pool)
        .await
        .unwrap();
    let visible: Option<(i64,)> =
        sqlx::query_as("SELECT id FROM p3_smoke WHERE name = $1 AND deleted_at IS NULL")
            .bind("smoke-alice")
            .fetch_optional(&pool)
            .await
            .unwrap();
    assert!(
        visible.is_none(),
        "soft-deleted row must be filtered when deleted_at IS NOT NULL"
    );
    let included: Option<(i64,)> = sqlx::query_as("SELECT id FROM p3_smoke WHERE name = $1")
        .bind("smoke-alice")
        .fetch_optional(&pool)
        .await
        .unwrap();
    assert!(
        included.is_some(),
        "soft-deleted row must still exist when not filtering deleted_at"
    );

    // 5) UPSERT：ON CONFLICT ... DO UPDATE（Postgres 特有语法）。
    sqlx::query(
        "INSERT INTO p3_smoke (name, payload) VALUES ($1, $2::jsonb) \
         ON CONFLICT (name) DO UPDATE SET payload = EXCLUDED.payload, updated_at = now()",
    )
    .bind("smoke-bob")
    .bind(r#"{"upsert":true}"#)
    .execute(&pool)
    .await
    .expect("upsert first insert");
    sqlx::query(
        "INSERT INTO p3_smoke (name, payload) VALUES ($1, $2::jsonb) \
         ON CONFLICT (name) DO UPDATE SET payload = EXCLUDED.payload, updated_at = now()",
    )
    .bind("smoke-bob")
    .bind(r#"{"upsert":false}"#)
    .execute(&pool)
    .await
    .expect("upsert second conflict");
    let (bob_text,): (String,) =
        sqlx::query_as("SELECT payload::text FROM p3_smoke WHERE name = $1")
            .bind("smoke-bob")
            .fetch_one(&pool)
            .await
            .unwrap();
    let bob: serde_json::Value = serde_json::from_str(&bob_text).unwrap();
    assert_eq!(
        bob["upsert"], false,
        "ON CONFLICT DO UPDATE must overwrite existing row"
    );

    // 清理。
    sqlx::query("DROP TABLE IF EXISTS p3_smoke")
        .execute(&pool)
        .await
        .unwrap();
}
