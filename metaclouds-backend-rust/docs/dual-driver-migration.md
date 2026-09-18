# 双驱动迁移说明（SQLite ↔ PostgreSQL）

本文档说明 `metaclouds-backend-rust` 的双数据库驱动架构、两套迁移目录的关系，
以及如何在生产环境用 **PostgreSQL** 运行应用级迁移。

> 状态：`migrations/postgres/` 下 9 个迁移文件已创建并通过**静态校验**（本机无 psql）。
> **待 CI / 目标环境 psql 实跑验证。**

---

## 1. 双驱动架构

`src/db.rs` 提供 `DatabasePool` 枚举，按连接串选择驱动：

```rust
pub enum DatabasePool {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}
```

分支规则（`connect_pool` / `is_sqlite_url`）：

- `MEMORY_STORE_ENABLED=true` → 内存 SQLite（开发用）；
- 否则 `USE_SQLITE=true`（默认）或 DSN 非 `postgres://` → SQLite 文件；
- `USE_SQLITE=false` 且 DSN 为 `postgres://user:password@host:port/dbname?sslmode=...` → PostgreSQL。

Rust 侧时间戳字段统一为 `chrono::DateTime<Utc>`（或 `Option<...>`），
字符串为 `String`，数值计数为 `i64`，少量小整数/端口为 `i32`，
开关标志为 `bool`，JSON 字段为 `orm::Json<T>`。这些 Rust 类型决定了下方的列类型映射。

---

## 2. 迁移目录结构

```
migrations/
├── 001_initial.sql                 # SQLite 方言（默认，应用启动自动跑）
├── 002_tenants_clusters_resources.sql
├── 003_b1_seed.sql
├── 004_b2_resources.sql
├── 005_b5_acceleration.sql
├── 006_b3_jobs.sql
├── 007_b4_scheduler.sql
├── 008_b6_governance.sql
├── 009_legacy_fixes.sql
└── postgres/                       # PostgreSQL 方言变体（与 SQLite 同序号）
    ├── 001_initial.sql
    ├── 002_tenants_clusters_resources.sql
    ├── 003_b1_seed.sql
    ├── 004_b2_resources.sql
    ├── 005_b5_acceleration.sql
    ├── 006_b3_jobs.sql
    ├── 007_b4_scheduler.sql
    ├── 008_b6_governance.sql
    └── 009_legacy_fixes.sql
```

- 命名沿用现有 SQLite 约定 `{编号}_{名称}.sql`（sqlx 按数字前缀识别版本）。
- 两套文件**语义等价**：表名（20）、索引名（53）、列名（逐表）已脚本对账，diff 为空。
- SQLite 版本**不改动**，现有 SQLite 测试与开发流程不受影响。

---

## 3. 类型映射表（SQLite → PostgreSQL）

| SQLite | PostgreSQL | 适用列 |
|--------|------------|--------|
| `INTEGER PRIMARY KEY AUTOINCREMENT` | `BIGSERIAL PRIMARY KEY` | 全部 `id`（对齐 GORM uint64） |
| `TEXT`（时间戳，NOT NULL） | `TIMESTAMPTZ NOT NULL DEFAULT NOW()` | `created_at` / `updated_at` |
| `TEXT`（时间戳，可空） | `TIMESTAMPTZ NULL` | `deleted_at` / `last_login_at` / `start_time` / `end_time` / `started_at` / `ended_at` / `expires_at` / `acknowledged_at` / `resolved_at` / `last_heartbeat` |
| `TEXT`（时间戳，NOT NULL） | `TIMESTAMPTZ NOT NULL` | `granted_at` / `triggered_at`（由应用写入，不给库默认值） |
| `TEXT`（JSON，`orm::Json<T>`） | `JSONB DEFAULT '{}'::jsonb` / `'[]'::jsonb` | 见下方 JSONB 清单 |
| `TEXT`（普通字符串） | `TEXT` | 其余字符串列 |
| `INTEGER`（`i64` 计数/外键） | `BIGINT` | 大多数整数列 |
| `INTEGER`（`i32` 小整数/端口） | `INTEGER` | `replicas`、`worker_replicas`、`gpu_per_worker`、`cpu_per_worker`、`memory_per_worker_gb`、`gpu_per_replica`、`cpu_per_replica`、`memory_per_replica_gb`、`port`、`security_policies.priority` |
| `INTEGER 0/1`（`bool`） | `BOOLEAN DEFAULT FALSE` / `TRUE` | `multi_cluster_enabled`、`mig_enabled`、`elastic_enabled`、`checkpoint_enabled`、`topology_anti_affinity`（FALSE）；`security_policies.enabled`（TRUE） |
| `REAL`（`f64`） | `DOUBLE PRECISION` | `utilization`、`vram_oversubscription_ratio`、`gpu_fraction`、`fraction`、`cpu_cores`、`memory_gb`、`resource_quotas.*limit/used` |

### 映射要点（易错处）

1. **`Json<T>` 列才用 JSONB**（共 15 列）：
   `clusters.gpu_vendors/scheduler_types`、`topology_nodes.labels`、`datasets.labels`、
   `distributed_training_configs.env_vars`、`acceleration_suites.acceleration_config`、
   `checkpoints.metrics`、`partitions.node_selector/labels`、
   `scheduler_integrations.credentials/config`、`alerts.metadata`、
   `security_policies.resources/actions/conditions`。
2. **JSON 文本但 Rust 侧是 `String` 的列保持 `TEXT`**（不要用 JSONB）：
   `jobs.node_selector/affinity/tolerations/scaling_policy`、`gpu_devices.mig_profiles`、
   各 `details` 列。原因：sqlx 不会把 JSONB 值直接解码进 Rust `String`。
3. **枚举字段保持 `TEXT`**（如 `status`/`role`/`partition_type` 等），不创建 PostgreSQL
   `ENUM` 类型——与 Go GORM 的 string 列一致，避免迁移复杂度。
4. `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` 在 PostgreSQL 下可用（SQLite 无此语法），
   002/003/009 的 ALTER 用它保证幂等。

---

## 4. 如何运行 PostgreSQL 迁移

### 方式 A：sqlx-cli 手动执行（推荐用于首次部署）

```powershell
# 安装（如未安装）：cargo install sqlx-cli --no-default-features --features postgres

sqlx migrate run `
  --source migrations/postgres `
  --database-url "postgres://user:password@host:5432/metaclouds?sslmode=disable"
```

- `--source migrations/postgres` 关键：指向 PG 变体目录，而非默认的 `migrations/`。
- 回滚/新建迁移时同样用该 `--source`。

### 方式 B：应用启动自动迁移（需源码接线，见第 7 节）

当前 `src/db.rs::run_migrations` 的两个分支都指向 `sqlx::migrate!("./migrations")`。
要让应用在 Postgres 下自动选用 `migrations/postgres/`，需把 `DatabasePool::Postgres`
分支改为加载 `./migrations/postgres` 的 migrator（本任务不改源码，仅记录接线点）。

---

## 5. CI `test-postgres` job 说明

- `.github/workflows/ci-rust.yml` 的 `test-postgres` job 起 `postgres:16` service 容器，
  跑 `#[ignore]` 标注的 `postgres_smoke_test`。
- 当前该 smoke 测试**内联建表**（绕过迁移文件），仅验证连通性与最小读写。
- `migrations/postgres/` 是**应用级真实迁移**，供生产与后续端到端 PG 验证使用；
  后续应把 smoke / 集成测试从"内联建表"切换为"跑 `migrations/postgres/`"，形成闭环。

---

## 6. 生产环境部署步骤

1. 准备 PostgreSQL 14+ 实例，建库建用户。
2. 设置环境变量：
   ```powershell
   $env:USE_SQLITE = "false"
   $env:DATABASE_HOST = "pg-host"
   $env:DATABASE_PORT = "5432"
   $env:DATABASE_USER = "metaclouds"
   $env:DATABASE_PASSWORD = "***"
   $env:DATABASE_NAME = "metaclouds"
   $env:DATABASE_SSLMODE = "disable"   # 或 require，按环境
   $env:JWT_SECRET = "<>=32 字节随机串"
   ```
3. **先跑迁移**（方式 A），确认 9 个版本全部入库。
4. 启动应用（`cargo run` / 容器）。管理员账户由运行时幂等播种
   （`seed_admin_if_empty`），不在迁移里写死 argon2 哈希。
5. 验证：`GET /health`、`GET /metrics`、登录并拉一个业务列表。

---

## 7. 已知限制与待办（诚实记录）

- **本机无 psql**，未实际执行迁移；本批仅做静态校验（见第 8 节）。**待 CI/目标环境实跑。**
- **`orm::Json<T>` 目前只有 SQLite 的 Encode/Decode 实现**（`src/orm/mod.rs`）。
  在 PostgreSQL 端真正读写 JSONB 列前，需要为 `Json<T>` 补 `Type<Postgres>` /
  `Encode<Postgres>` / `Decode<Postgres>` 实现（编译期即会暴露）。本任务不改源码。
- **`run_migrations` 尚未按驱动选目录**：PG 分支仍指向 `./migrations`。生产自动迁移前，
  需按第 4 节方式 B 接线（或改用方式 A 的 sqlx-cli）。
- **枚举用 TEXT 而非 PG ENUM 类型**：靠应用层约束取值，未加 DB 层 `CHECK`。
- **JSONB 索引**：`alerts.metadata`、`partitions.labels` 等 JSONB 列当前无 GIN 索引；
  若后续按 JSON 字段查询，建议按需 `CREATE INDEX ... USING GIN (col jsonb_path_ops)`。
- 内联 `UNIQUE`（如 `users.username`）会自动建唯一索引，紧随其后的同名普通 `CREATE INDEX`
  为冗余索引（与 SQLite 版保持一致），如需可在生产再清理。

---

## 8. 静态校验结果（已执行）

用 PowerShell 脚本对 `migrations/postgres/*.sql` 逐文件检查（UTF-8 读取、剥离行内 `--` 注释后扫描纯 SQL）：

| 检查项 | 结果 |
|--------|------|
| 文件数量 | 9（与 SQLite 001–009 一一对应） |
| 纯 SQL 含 `AUTOINCREMENT` | 0 |
| 纯 SQL 含反引号 / `?` 占位符 / `DATETIME` | 0 |
| 裸 `SERIAL`（应为 `BIGSERIAL`） | 0 |
| 每个文件以 `;` 结尾 | 9/9 |
| 表名对账（SQLite vs PG） | 20/20 完全一致，diff 为空 |
| 索引名对账 | 53/53 完全一致，diff 为空 |
| 逐表列名对账 | 20/20 表零差异 |

> 说明：迁移文件头注释里出现的 `AUTOINCREMENT`/`DATETIME` 字样仅用于说明映射规则，
> 不在可执行 SQL 中；上表统计的是剥离注释后的纯 SQL。

---

## 9. 相关文件

- SQLite 迁移：`migrations/001..009*.sql`
- PostgreSQL 变体：`migrations/postgres/001..009*.sql`
- 双驱动池与迁移入口：`src/db.rs`（`DatabasePool` / `run_migrations`）
- JSON 包装与编解码：`src/orm/mod.rs`（`Json<T>`）
- 部署/回滚 runbook：`docs/runbook-v2-rust.md`
