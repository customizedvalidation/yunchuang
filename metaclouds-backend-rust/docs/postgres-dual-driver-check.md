# B7 — Postgres 双驱动测试检查报告

- 任务：B7 Postgres 双驱动测试检查
- 项目：`metaclouds-backend-rust`
- 检查日期：2026-09-18
- 检查方式：纯检查验证，未修改任何源码
- 结论摘要：**本机无 PostgreSQL，双驱动冒烟测试待 CI 实跑；CI `test-postgres` job 配置 5 项确认完整。**

---

## 1. 本机 PostgreSQL 检查结果

通过 PowerShell 在本机执行以下四项检查：

| 检查项 | 命令 | 结果 |
| --- | --- | --- |
| Windows 服务 | `Get-Service *postgres*` | **无匹配服务** |
| 运行进程 | `Get-Process *postgres*` | **无 postgres 进程** |
| psql 客户端 | `Get-Command psql` | **未找到 psql** |
| 端口监听 | `Test-NetConnection localhost -Port 5432` | `TcpTestSucceeded: False`（5432 未监听） |

**结论：本机未安装 / 未运行 PostgreSQL，无 psql 客户端，5432 端口无监听。**
因此不执行"启动服务 → 建库 → 跑 smoke test"路径，转入 CI 配置确认路径。未在本机创建任何测试数据库或用户。

---

## 2. CI 配置确认清单（无 PG 路径）

文件：`.github/workflows/ci-rust.yml`，job：`test-postgres`（第 55–95 行）。

### 2.1 确认项 1 — job 存在
- `test-postgres` job 存在，`runs-on: ubuntu-latest`（第 55–56 行）。✅

### 2.2 确认项 2 — postgres:16 service 容器
- `image: postgres:16`（第 59 行）。✅
- 环境变量（第 60–63 行）：
  - `POSTGRES_USER: postgres`
  - `POSTGRES_PASSWORD: postgres`
  - `POSTGRES_DB: test`
- 端口映射 `5432:5432`（第 65 行）。✅
- 健康检查（第 66–70 行）：`--health-cmd pg_isready`，10s 间隔 / 5s 超时 / 5 次重试，确保容器就绪后再跑测试。✅

### 2.3 确认项 3 — job 环境变量
- `DATABASE_URL: postgres://postgres:postgres@localhost:5432/test`（第 72 行），与 service 容器的 user/password/db 一致。✅
- `USE_SQLITE: "false"`（第 73 行），显式关闭 SQLite 模式。✅
- `JWT_SECRET`（第 74 行）。✅
- `SQLX_OFFLINE: "false"`（第 76 行），注释说明冒烟测试全部用运行时 SQL、无 `query!` 宏，不依赖 sqlx 离线缓存。✅

### 2.4 确认项 4 — 测试命令
- 第 94–95 行：
  ```text
  cargo test --test postgres_smoke_test -- --ignored --nocapture
  ```
- 精确命中 `tests/postgres_smoke_test.rs` 集成测试 target，并通过 `--ignored` 显式启用 `#[ignore]` 标记的用例，`--nocapture` 输出运行时日志。✅

### 2.5 确认项 5 — 迁移 / SQL 方言兼容性
- `tests/postgres_smoke_test.rs` **不走 `migrations/` 目录**，而是在测试内联 `CREATE TABLE IF NOT EXISTS p3_smoke (...)`，使用 Postgres 原生类型（`BIGSERIAL` / `TIMESTAMPTZ` / `JSONB`），因此 CI smoke job 不依赖迁移文件即可自洽运行。✅
- `migrations/` 目录下的 8 个迁移文件（`001_initial.sql` … `008_b6_governance.sql`）当前为 **SQLite 方言**：
  - 自增主键统一写作 `INTEGER PRIMARY KEY AUTOINCREMENT`（如 `001_initial.sql:5`、`002_*.sql:22/39/64`、`004_*.sql:15`、`005_*.sql:9/30/48` 等），`AUTOINCREMENT` 为 SQLite 专有语法，Postgres 不支持，需 `BIGSERIAL` / `GENERATED ... AS IDENTITY`。
  - 时间戳字段统一用 `TEXT` 存储（`created_at` / `updated_at` / `deleted_at`），JSON 字段（`gpu_vendors` / `scheduler_types` / `labels`）也用 `TEXT` 存 JSON 字符串，由 `orm::Json<T>` 编解码。
  - 迁移文件头注释（`002_*.sql:13`、`005_*.sql:4`）已明确标注"后续接入 Postgres 时，应在 `migrations/postgres/` 下提供对应变体（SERIAL/BIGSERIAL、JSONB、TIMESTAMPTZ）"。
- **结论**：CI smoke job 因内联建表而兼容；但 `src/db.rs::run_migrations` 对 `DatabasePool::Postgres` 分支仍指向同一个 `./migrations`（SQLite 方言），在真实 Postgres 上执行应用级迁移会失败，属于已知限制，需后续提供 `migrations/postgres/` 变体。本项记录为待办，未修改。⚠️

---

## 3. 双驱动架构说明

### 3.1 `DatabasePool` 枚举（`src/db.rs:46–52`）
```rust
pub enum DatabasePool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}
```
- `is_sqlite()` / `as_sqlite()` 辅助方法（`src/db.rs:54–66`）。
- 文档注释明确：现阶段 SQLite 分支是唯一真正落地数据的实现，Postgres 分支为代码层面预留。

### 3.2 连接池工厂 `connect_pool`（`src/db.rs:85–102`）
三分支语义，对齐 Go `InitDB`：
1. `memory_store_enabled = true` → SQLite 内存库（`sqlite::memory:`，单连接）。
2. `use_sqlite = true` → SQLite 文件库（`SQLITE_MAX_CONNS = 5`）。
3. 否则 → Postgres，DSN 由 `Config::get_database_dsn()` 按 `postgres://user:password@host:port/dbname?sslmode=disable` 拼装。

### 3.3 Postgres 连接配置（`src/db.rs:134–150`）
`PgPoolOptions` 设置 `max_connections` / `min_connections` / `max_lifetime` / `idle_timeout` / `acquire_timeout`，对齐 Go `SetMaxOpenConns(100)` 等参数。

### 3.4 迁移入口（`src/db.rs:156–162`）
`run_migrations(&DatabasePool)` 对两个分支都执行 `sqlx::migrate!("./migrations")`。当前 Postgres 分支复用 SQLite 方言迁移目录，是已知待办（见 2.5）。

### 3.5 依赖就绪（`Cargo.toml`）
- `sqlx = { version = "0.8", features = ["sqlite", "postgres", "runtime-tokio-rustls", "chrono"] }`（第 11–15 行）—— sqlite 与 postgres feature 同时启用，smoke test 可在 CI 直接编译。
- `serde_json = "1"`、`chrono = { features = ["serde"] }` 就绪，满足 smoke test 中 `serde_json::Value` 与 `DateTime<Utc>` 往返。

---

## 4. Postgres 冒烟测试覆盖点（`tests/postgres_smoke_test.rs`）

单个 `#[tokio::test] #[ignore]` 用例 `postgres_connect_json_timestamp_softdelete_upsert`，防御性读取 `DATABASE_URL`（非 `postgres://` 开头直接 return，避免误在 SQLite 环境 panic）。内联建表 `p3_smoke` 后验证：

1. **JSONB 写入与读回**：`INSERT ... VALUES ($1, $2::jsonb)` 绑定 JSON 字符串，`SELECT payload::text` 读回后 `serde_json::from_str` 解析，断言 `region` / `gpu` 字段。绕开 sqlx `json` feature，用 `::jsonb` / `::text` 显式转换。
2. **TIMESTAMPTZ 往返**：`created_at` 为 `TIMESTAMPTZ NOT NULL DEFAULT now()`，`query_as` 反序列化为 `chrono::DateTime<Utc>`，断言 `timestamp() > 0`。
3. **UPDATE 改 JSON**：`updated_at = now()` + 覆盖 `payload`，读回断言 `gpu` 由 `A100` 变为 `H100`。
4. **软删除**：`UPDATE ... SET deleted_at = now()`；带 `AND deleted_at IS NULL` 的查询查不到该行，不带过滤的查询仍能查到该行——验证"普通过滤 / 含已删除"两条路径。
5. **UPSERT（Postgres 特有）**：`INSERT ... ON CONFLICT (name) DO UPDATE SET payload = EXCLUDED.payload, updated_at = now()`，连续两次插入 `smoke-bob`，第二次冲突后断言 `upsert` 字段被覆盖为 `false`。区别于 SQLite 的 `INSERT OR REPLACE`。
6. 收尾 `DROP TABLE IF EXISTS p3_smoke` 清理。

---

## 5. 待 CI / 目标环境实跑项清单

本机无 PG，以下项目需在 GitHub Actions `test-postgres` job（或目标 Postgres 环境）实跑确认：

| # | 待验证项 | 预期结果 | 验证点 |
| --- | --- | --- | --- |
| 1 | postgres:16 service 容器就绪 | `pg_isready` 健康检查通过，job 进入测试步骤 | workflow 日志显示 service healthy |
| 2 | `cargo test --test postgres_smoke_test -- --ignored --nocapture` 编译通过 | sqlx postgres feature 编译无错 | 编译阶段无 feature / 依赖错误 |
| 3 | JSONB 写入读回 | `payload["region"] == "cn-sh"`、`gpu` 初始 `A100`、UPDATE 后 `H100` | 断言通过 |
| 4 | TIMESTAMPTZ 往返 | `DateTime<Utc>` 反序列化成功，`timestamp() > 0` | 无类型转换错误 |
| 5 | 软删除双路径 | 带 `deleted_at IS NULL` 查不到；不带过滤查得到 | 两个 `assert!` 通过 |
| 6 | ON CONFLICT UPSERT | 第二次冲突插入后 `upsert == false` | 区别于 SQLite `INSERT OR REPLACE` |
| 7 | smoke test 清理 | `DROP TABLE` 成功，无残留 | 测试结束无 panic |
| 8 | SQLite 回归不受影响 | `lint-test` job 的 `cargo test --all-features` 仍全绿，smoke test 在无 PG 环境保持 ignored 跳过 | 主流水线不依赖 PG |
| 9 | 应用级迁移在 Postgres 下执行（后续） | 需先提供 `migrations/postgres/` 变体 | 当前为已知限制，不在本 CI job 范围内 |

---

## 6. 已知限制

1. **迁移方言未分叉**：`migrations/` 为 SQLite 方言（`INTEGER PRIMARY KEY AUTOINCREMENT`、`TEXT` 存时间戳与 JSON）。`run_migrations` 对 Postgres 分支仍指向该目录，真实 Postgres 下执行应用级迁移会因 `AUTOINCREMENT` 语法报错。需后续新增 `migrations/postgres/`（`BIGSERIAL` / `TIMESTAMPTZ` / `JSONB`），并在 `run_migrations` 中按驱动选择目录。
2. **JSON 字段处理差异**：SQLite 侧 JSON 以 `TEXT` 存储、由 `orm::Json<T>` 编解码；Postgres 侧目标是 `JSONB`。smoke test 用 `$2::jsonb` 绑定 + `payload::text` 读回，未启用 sqlx `json` feature；模型层若要直接读写 `JSONB` 列，后续需考虑 sqlx `json` feature 或保持 `::text` 转换模式。
3. **时间戳类型差异**：SQLite 侧 `TEXT` 存时间戳字符串；Postgres 侧 `TIMESTAMPTZ` + `DateTime<Utc>`。ORM 层需保持编解码一致。
4. **UPSERT 语法差异**：SQLite 用 `INSERT OR REPLACE`，Postgres 用 `ON CONFLICT ... DO UPDATE`。模型层 SQL 若要双驱动复用，需按驱动分支（当前业务模型层全部参数化 `SqlitePool`，尚未切到 `DatabasePool`）。
5. **AppState 仍为 `SqlitePool`**：`src/auth/middleware.rs:19` 与 `src/lib.rs:73` 的 `AppState.pool` / `build_app` 签名仍为 `sqlx::SqlitePool`，`DatabasePool::Postgres` 分支在 HTTP 层尚未实际接入，属后续整合范围。
6. **本机未实跑**：因本机无 PostgreSQL，第 5 节全部项目仅做静态确认，未产出实跑通过数 / 失败数 / 耗时数据。

---

## 7. 验收对照

| 验收标准 | 状态 |
| --- | --- |
| 1. 本机 PG 检查完成（有/无明确结论） | ✅ 无（服务/进程/psql/5432 四项均阴性） |
| 2. 若有 PG：smoke test 实跑通过、全量无回归 | N/A（本机无 PG，未执行） |
| 3. 若无 PG：CI 配置 5 项确认完整 | ✅ job / service / env / test command / migration compat 全部确认（迁移兼容性含 1 项已知限制，已记录未修改） |
| 4. 检查报告 `docs/postgres-dual-driver-check.md` 完整产出 | ✅ 本文件 |
| 5. 明确标注待 CI / 目标环境项 | ✅ 第 5 节清单 + 第 6 节限制 |
