# Metaclouds P0-P2 差距修复 - 实施计划

## Task 1: 数据库层 AnyPool 双驱动重构
- **Status**: `pending`
- **Priority**: high
- **Depends On**: None
- **Description**:
  - `src/db.rs` 中将 `DatabasePool` 从枚举改为持有 `sqlx::any::AnyPool` + `DbDialect` 的结构体
  - 提供 `connect()` 工厂方法按配置选择 SQLite/Postgres
  - 保留 `run_migrations()` 按方言选择迁移目录
  - 改造 `seed_admin_if_empty` 兼容 AnyPool
- **Acceptance Criteria Addressed**: AC-1
- **Test Requirements**:
  - `rule` TR-1.1: `DatabasePool::connect` 在 SQLite 与 Postgres URL 下均返回 Ok，且 dialect 正确；证据：db_test
  - `rule` TR-1.2: `run_migrations` 按 dialect 选择正确迁移目录；证据：db_test
- **Notes**: sqlx AnyPool 自动翻译 `?` → `$N`，大部分查询无需改 SQL 文本

## Task 2: AppState 与 main.rs 切换到 AnyPool
- **Status**: `pending`
- **Priority**: high
- **Depends On**: Task 1
- **Description**:
  - `src/auth/middleware.rs` 中 `AppState.pool` 类型从 `SqlitePool` 改为 `DatabasePool`
  - `src/main.rs` 移除 Postgres 路径的错误阻断，统一走 `DatabasePool::connect`
  - 修复编译错误，确保 SQLite 测试通道不变
- **Acceptance Criteria Addressed**: AC-1
- **Test Requirements**:
  - `rule` TR-2.1: `cargo build` 通过；证据：编译输出
  - `rule` TR-2.2: 现有 SQLite 集成测试全部通过；证据：cargo test

## Task 3: 服务层 SQL 方言适配（自增 ID 与 JSON 类型）
- **Status**: `pending`
- **Priority**: high
- **Depends On**: Task 2
- **Description**:
  - 抽取 `insert_returning_id!` 辅助宏，Postgres 用 `RETURNING id`，SQLite 用 `last_insert_rowid()`
  - 所有 INSERT 语句改为通过宏获取自增 ID
  - JSON 字段：SQLite 用 TEXT 存 JSON 字符串，Postgres 用 JSONB，模型层用 `serde_json::Value`
  - 按域逐个迁移 B1-B6 service
- **Acceptance Criteria Addressed**: AC-1
- **Test Requirements**:
  - `rule` TR-3.1: Postgres 集成测试覆盖全部 B1-B6 域 CRUD；证据：postgres_integration_test
  - `rule` TR-3.2: SQLite 通道无回归；证据：cargo test --features sqlite

## Task 4: Redis 分布式滑动窗口限流器
- **Status**: `pending`
- **Priority**: high
- **Depends On**: None
- **Description**:
  - 新增 `src/middleware/rate_limit_redis.rs`，用 Redis Sorted Set + Lua 脚本实现原子滑动窗口
  - 改造 `rate_limit_middleware` 优先使用分布式限流器，Redis 不可用时降级到内存限流器
  - key 格式 `metaclouds:ratelimit:{ip}`
- **Acceptance Criteria Addressed**: AC-2
- **Test Requirements**:
  - `rule` TR-4.1: Lua 脚本逻辑单测（mock Redis）；证据：rate_limit_redis_test
  - `rule` TR-4.2: 降级逻辑：Redis 不可用时回退内存限流且不 panic；证据：中间件测试

## Task 5: 调度器 Redis 租约选主
- **Status**: `pending`
- **Priority**: high
- **Depends On**: None
- **Description**:
  - 新增 `src/scheduler/leader.rs`：`try_acquire_leadership` + `renew_lease`（SET NX EX）
  - `AppState` 增加 `redis` 字段（ConnectionManager）
  - `scheduler/mod.rs` 启动前抢锁，抢到才注册 cron，每 15s 续租，丢失则退出
  - `main.rs` 中 `tokio::spawn` 调度器循环
- **Acceptance Criteria Addressed**: AC-3
- **Test Requirements**:
  - `rule` TR-5.1: 选主逻辑：两个 instance 同时抢锁只有一个成功；证据：leader_test
  - `rule` TR-5.2: 租约过期后其他 instance 可接管；证据：leader_test

## Task 6: 异步消息总线（NATS）
- **Status**: `pending`
- **Priority**: medium
- **Depends On**: None
- **Description**:
  - `Cargo.toml` 新增 `async-nats`
  - 新增 `src/messagebus/mod.rs`：MessageBus 封装 connect/publish/subscribe
  - `AppState` 增加 `message_bus` 字段
  - `services/job.rs` 的 submit 改为发布到 `jobs.submit`
  - 新增 `src/worker/job_worker.rs`：消费消息，3 次重试失败入 `jobs.dlq`
  - main.rs 启动 worker
- **Acceptance Criteria Addressed**: AC-4
- **Test Requirements**:
  - `rule` TR-6.1: submit 仅发布消息不直接调 K8s；证据：job 测试
  - `rule` TR-6.2: worker 消费失败 3 次后消息进入 DLQ；证据：worker_test

## Task 7: 审计日志中间件
- **Status**: `pending`
- **Priority**: medium
- **Depends On**: Task 2
- **Description**:
  - 新增迁移 `migrations/010_audit_logs.sql` 与 `migrations/postgres/010_audit_logs.sql`
  - 新增 `src/middleware/audit.rs`：拦截 POST/PUT/DELETE，记录 user_id/tenant_id/resource/action/status/ip
  - `routes.rs` 受保护路由层挂载 `audit_middleware`
  - 审计写入失败不影响主流程
- **Acceptance Criteria Addressed**: AC-5
- **Test Requirements**:
  - `rule` TR-7.1: 写操作后 audit_logs 表有记录；证据：audit_test
  - `rule` TR-7.2: 审计写入失败时业务响应仍正常；证据：audit_test

## Task 8: 默认管理员强制改密
- **Status**: `pending`
- **Priority**: high
- **Depends On**: Task 2
- **Description**:
  - 新增迁移 `migrations/011_force_password_change.sql`：users 表加 `must_change_password BOOLEAN DEFAULT false`
  - `models/user.rs` 增加字段
  - `db.rs` 播种 admin 时设 `must_change_password=true`
  - `error.rs` 新增 `MustChangePassword` 错误码
  - `auth/handler.rs` login 检查该字段，返回专用错误码
  - `auth/handler.rs` change_password 成功后清除标记
  - 前端 Login.vue 识别错误码跳改密页
- **Acceptance Criteria Addressed**: AC-6
- **Test Requirements**:
  - `rule` TR-8.1: admin 首次登录返回 MUST_CHANGE_PASSWORD；证据：auth_test
  - `rule` TR-8.2: 改密后正常登录；证据：auth_test

## Task 9: 登录防爆破
- **Status**: `pending`
- **Priority**: high
- **Depends On**: Task 4, Task 8
- **Description**:
  - 新增迁移 `migrations/012_account_lockout.sql`：users 表加 `locked_until TIMESTAMP`
  - `auth/handler.rs` login：IP 级 5 次/分钟限流（复用分布式限流器，key=login:ip）
  - 账户级连续失败 5 次锁定 15 分钟（Redis INCR + locked_until）
  - 登录成功清除失败计数
- **Acceptance Criteria Addressed**: AC-7
- **Test Requirements**:
  - `rule` TR-9.1: 同 IP 6 次/分钟登录返回 429；证据：auth_test
  - `rule` TR-9.2: 连续 5 次密码错误锁定 15 分钟；证据：auth_test

## Task 10: 前端质量门禁硬化
- **Status**: `pending`
- **Priority**: medium
- **Depends On**: None
- **Description**:
  - 新增 `.eslintrc.cjs`、`.prettierrc`
  - `package.json` 新增 eslint/prettier 依赖与 lint/format 脚本
  - `ci-cd.yml` 前端测试改为阻塞，新增 lint 步骤，覆盖率门槛 ≥ 70%
- **Acceptance Criteria Addressed**: AC-8
- **Test Requirements**:
  - `rule` TR-10.1: `npm run lint` 无 error；证据：本地运行
  - `rule` TR-10.2: CI 配置中测试无 continue-on-error；证据：ci-cd.yml 审查

## Task 11: OTel 生产默认开启
- **Status**: `pending`
- **Priority**: low
- **Depends On**: None
- **Description**:
  - `config.rs` 中 `otel_enabled` 默认值改为：生产环境 true，其他 false
  - `tracing/init.rs` 确保 OTel exporter 初始化失败仅告警不 panic
- **Acceptance Criteria Addressed**: AC-9
- **Test Requirements**:
  - `rule` TR-11.1: 生产环境 otel_enabled 默认为 true；证据：config_test

## Task 12: Argo Rollouts 金丝雀部署骨架
- **Status**: `pending`
- **Priority**: low
- **Depends On**: None
- **Description**:
  - 新增 `k8s/rollout.yaml`：Rollout CRD，金丝雀策略 steps 20%→50%→100%，每步 pause 60s
  - 保留原 Deployment 作为参考或注释
- **Acceptance Criteria Addressed**: AC-10
- **Test Requirements**:
  - `rule` TR-12.1: Rollout YAML 语法正确且包含金丝雀 steps；证据：YAML 校验

## Task 13: Sealed Secrets 清单与文档
- **Status**: `pending`
- **Priority**: low
- **Depends On**: None
- **Description**:
  - 新增 `k8s/12-sealed-secrets.yaml`：SealedSecret CRD 示例（JWT_SECRET/DATABASE_URL/REDIS_PASSWORD）
  - 新增 `docs/sealed-secrets-guide.md`：kubeseal 使用说明
- **Acceptance Criteria Addressed**: AC-11
- **Test Requirements**:
  - `rule` TR-13.1: SealedSecret 清单存在且字段完整；证据：文件审查

## Task 14: Postgres 备份 CronJob
- **Status**: `pending`
- **Priority**: low
- **Depends On**: None
- **Description**:
  - 新增 `k8s/backup-cronjob.yaml`：每日 02:00 执行 pg_dump，输出到 PVC，保留 7 天
- **Acceptance Criteria Addressed**: AC-12
- **Test Requirements**:
  - `rule` TR-14.1: CronJob 清单存在且 schedule 正确；证据：YAML 审查

## Task 15: 多租户行级隔离显式化
- **Status**: `pending`
- **Priority**: low
- **Depends On**: Task 3
- **Description**:
  - 审查所有 tenants 相关查询，确保 WHERE 子句带 tenant_id
  - 在 JWT claims 中提取 tenant_id，service 层注入查询
  - 新增多租户隔离测试
- **Acceptance Criteria Addressed**: AC-13
- **Test Requirements**:
  - `rubric` TR-15.1: 租户隔离显式度；scale 1-5；anchors 1=无过滤/3=部分/5=全量+测试；threshold >= 4；证据：services 审查 + 测试
