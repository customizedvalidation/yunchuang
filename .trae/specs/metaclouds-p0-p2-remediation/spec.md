# Metaclouds P0-P2 差距修复 - 产品需求文档

## Overview
- **Summary**: 修复架构复盘报告 `metaclouds-architecture-review-2026-09-30.md` 中识别的全部 P0、P1、P2 级差距，使系统达到国际一流生产级水准。
- **Purpose**: 解锁生产数据库运行能力，补齐分布式协调、韧性、安全审计与前端质量门禁。
- **Target Users**: 平台运维、安全审计、SRE、前端开发者。

## Goals
- P0: 生产环境可在 PostgreSQL 上完整运行；多副本下限流与调度行为正确。
- P1: 具备异步削峰、操作审计、默认凭据安全、防爆破、前端质量门禁。
- P2: 链路追踪默认可用、金丝雀部署骨架、密钥管理升级、备份策略、多租户隔离显式化。

## Non-Goals
- 不重写前端页面逻辑，仅补质量门禁与强制改密交互。
- 不引入完整服务网格；金丝雀仅提供 Argo Rollouts 清单骨架。
- 不替换现有 K8s Secret 为外部 Vault（仅提供 Sealed Secrets 迁移清单与文档）。

## Background & Context
- 现状证据见 `metaclouds-architecture-review-2026-09-30.md` 与改造方案 `metaclouds-p0-p1-remediation-plan.md`。
- 后端为 Rust (axum + sqlx)，前端 Vue 3 + TS，部署于 K8s。
- 当前 `AppState.pool` 为 `SqlitePool`，Postgres 路径在 `main.rs` 中被阻断。

## Functional Requirements

### P0
- **FR-1**: `USE_SQLITE=false` 配置 PostgreSQL DSN 时，服务正常启动并处理全部 B1-B6 业务请求。
- **FR-2**: 限流基于 Redis 分布式滑动窗口，多副本下阈值一致。
- **FR-3**: 调度器通过 Redis 租约选主，任意时刻仅一个副本执行 cron 任务。

### P1
- **FR-4**: 作业提交走异步消息总线，失败进入死信队列，不丢失。
- **FR-5**: 所有写操作（POST/PUT/DELETE）产生审计日志记录。
- **FR-6**: 默认管理员首次登录必须改密。
- **FR-7**: 登录端点具备 IP 级限流与账户锁定防爆破。
- **FR-8**: 前端 ESLint + 阻塞式测试 + 覆盖率门槛纳入 CI。

### P2
- **FR-9**: 生产环境默认启用 OpenTelemetry 链路追踪。
- **FR-10**: 提供 Argo Rollouts 金丝雀部署清单骨架。
- **FR-11**: 提供 Sealed Secrets 加密清单与使用说明。
- **FR-12**: 提供 Postgres 备份 CronJob 清单。
- **FR-13**: 多租户数据行级隔离在代码与迁移中显式保证。

## Non-Functional Requirements
- **NFR-1**: 所有 P0/P1 改动不得破坏现有 SQLite 测试通道（双驱动兼容）。
- **NFR-2**: 安全相关改动（限流、审计、防爆破）在故障时降级而非阻断主流程。
- **NFR-3**: 新增代码通过 `cargo clippy --all-targets -- -D warnings` 与 `cargo fmt --check`。
- **NFR-4**: 前端新增配置通过 `npm run lint` 与 `npm run test`。

## Constraints
- **Technical**: sqlx AnyPool 需支持 SQLite 与 Postgres 双驱动；Redis 已在依赖中（redis 0.27）。
- **Dependencies**: NATS 作为消息总线（可选 Redis Streams 降级）；Argo Rollouts CRD 需集群预装。
- **Business**: 默认管理员强制改密不得影响存量非默认用户。

## Assumptions
- Redis 在生产环境高可用部署（主从 + Sentinels）。
- NATS 或等价消息中间件在生产环境可用。
- K8s 集群已安装 metrics-server、支持 NetworkPolicy 的 CNI。

## Acceptance Criteria

### AC-1: Postgres 生产路径贯通
- **Type**: `rule`
- **Given**: 配置 `USE_SQLITE=false` 且 `DATABASE_URL=postgres://...`
- **When**: 启动服务并调用任意 B1-B6 CRUD 接口
- **Then**: 服务不退出，接口返回正确数据，SQL 方言适配 Postgres
- **Pass Condition**: `cargo test --features postgres` 全绿 + 手动 smoke 通过
- **Evidence**: 测试输出 + 运行日志

### AC-2: 分布式限流多副本一致
- **Type**: `rule`
- **Given**: 3 副本部署，Redis 可用
- **When**: 同一 IP 60 秒内发送 101 次请求
- **Then**: 第 101 次返回 429，无论请求落到哪个副本
- **Pass Condition**: 分布式限流测试通过
- **Evidence**: 限流器单测 + 集成测试

### AC-3: 调度器单实例执行
- **Type**: `rule`
- **Given**: 3 副本部署，Redis 可用
- **When**: 到达 cron 触发时刻
- **Then**: 仅 leader 副本执行一次任务，其余副本不执行
- **Pass Condition**: 选主逻辑单测通过 + 日志验证
- **Evidence**: 选主测试 + 日志

### AC-4: 作业异步提交不丢失
- **Type**: `rule`
- **Given**: 消息总线可用，K8s API 暂时不可达
- **When**: 提交 10 个作业
- **Then**: 10 个作业全部进入队列，K8s 恢复后被消费，失败的进入 DLQ
- **Pass Condition**: DLQ 测试 + 重试逻辑验证
- **Evidence**: worker 测试

### AC-5: 写操作审计日志
- **Type**: `rule`
- **Given**: 已认证用户发起 POST 请求
- **When**: 请求完成
- **Then**: `audit_logs` 表新增一条记录，含 user_id/tenant_id/resource/action/status/ip
- **Pass Condition**: 审计中间件测试通过
- **Evidence**: 审计测试

### AC-6: 默认管理员强制改密
- **Type**: `rule`
- **Given**: 默认 admin 账户 `must_change_password=true`
- **When**: admin 登录
- **Then**: 返回错误码 `MUST_CHANGE_PASSWORD`，改密后可正常登录
- **Pass Condition**: 登录测试 + 改密测试
- **Evidence**: auth 测试

### AC-7: 登录防爆破
- **Type**: `rule`
- **Given**: 同一 IP 在 60 秒内对 /auth/login 发送 6 次请求
- **Then**: 第 6 次返回 429；同一账户连续 5 次密码错误后锁定 15 分钟
- **Pass Condition**: 限流 + 锁定测试通过
- **Evidence**: auth 测试

### AC-8: 前端质量门禁
- **Type**: `rule`
- **Given**: PR 触发 CI
- **When**: 运行 `npm run lint` 与 `npm run test:coverage`
- **Then**: ESLint 无 error，测试通过，语句覆盖率 ≥ 70%，否则阻断
- **Pass Condition**: CI 配置检查 + 本地运行通过
- **Evidence**: CI yml + 本地输出

### AC-9: OTel 生产默认开启
- **Type**: `rule`
- **Given**: `SERVER_ENV=production` 且未显式设置 `OTEL_ENABLED=false`
- **When**: 服务启动
- **Then**: OTel exporter 初始化并向 `OTEL_EXPORTER_OTLP_ENDPOINT` 导出 span
- **Pass Condition**: tracing 配置生产默认 true
- **Evidence**: config.rs 默认值

### AC-10: 金丝雀部署骨架
- **Type**: `rule`
- **Given**: 集群已安装 Argo Rollouts
- **When**: 应用 Rollout 清单
- **Then**: 后端以金丝雀策略（20% → 50% → 100%）发布
- **Pass Condition**: Rollout CRD 清单存在且语法正确
- **Evidence**: k8s/rollout.yaml

### AC-11: Sealed Secrets 清单
- **Type**: `rule`
- **Given**: 集群已安装 Sealed Secrets controller
- **When**: 应用 SealedSecret 清单
- **Then**: 敏感凭据被加密存储于 Git，运行时解密为 Secret
- **Pass Condition**: SealedSecret 清单存在
- **Evidence**: k8s/12-sealed-secrets.yaml 或文档

### AC-12: 备份策略
- **Type**: `rule`
- **Given**: K8s 集群
- **When**: 应用备份 CronJob
- **Then**: 每日定时执行 `pg_dump` 并上传至 PVC
- **Pass Condition**: CronJob 清单存在
- **Evidence**: k8s/backup-cronjob.yaml

### AC-13: 多租户行级隔离
- **Type**: `rubric`
- **Dimension**: 租户数据隔离显式度
- **Scale**: 1-5
- **Anchors**: 1 = 无 tenant_id 过滤；3 = 部分查询带 tenant_id；5 = 所有租户表查询强制带 tenant_id 且有测试
- **Pass Threshold**: >= 4
- **Evidence**: services 层查询审查

## Open Questions
- [ ] P2 的 Argo Rollouts / Sealed Secrets 是否需要在本仓库实际部署验证，还是仅提供清单？
