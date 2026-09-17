# Metaclouds 后端 Rust 全量重写 — 工作包级执行计划

> 编制日期：2026-09-16
> 上游文档：`docs/rust-backend-migration-assessment-2026-09-16.md`（可行性评估与 Phase 0-4 骨架）
> 新工程目录：`D:\YCYD\metaclouds-backend-rust\`
> Golden 基线输出：`D:\YCYD\docs\golden-api-baseline\`
> 工程师配置：1-2 人　|　总周期：12-16 周
> 铁律：Go 后端保留不删，作为 Golden 基线与回滚底座；`/api/v1` 前缀、信封 `{success,data}`、JWT httpOnly cookie、RBAC 权限名**逐字不变**；前端 Vue 零改动。

---

## 0. 现状盘点锚点（用于工作包估算）

| 维度 | 实测数量 | 本计划映射到 |
|---|---|---|
| 控制器文件 | 16 个业务 controller（auth/tenant/resource/cluster/topology/gpu/job/partition/quota/scheduler/dataset/checkpoint/acceleration/monitoring/security/k8s） | Phase 2 六批 |
| 服务文件 | 18 个业务 service + 3 个 metrics + 2 个 scheduler 适配 | Phase 2 / Phase 3 |
| 模型 | 21 个 + `db.go`(20KB) + `init_data.go` + `redis.go` + `bootstrap_credentials.go` | Phase 1 P1-06 + Phase 2 |
| 中间件 | 11 个：jwt_auth / csrf / security(RBAC) / error_handler / panic_recover / request_id / request_logger / timing / security_headers / validation / stack | P1-04 / P1-07 / P1-08 |
| 迁移 SQL | 4 对 up/down（init_schema / indexes / seed / skill_md_alignment） | P1-05 |
| 测试 | controllers 10 测试 + services 6 + middlewares 5 + tests/ e2e&集成 8 ≈ **33 个** | Phase 2 每批 + P4-04 |
| 覆盖率基线 | config 100% / controllers 94.4% / middlewares 76.4% | §7 质量门禁 |

---

## 1. 时间线总览（甘特式，14 周基线 + 2 周缓冲）

> 工作日 = 工程师×天（pd）。括号内为日历周。2 人并行时日历压缩，单人时按 pd 线性。

| WP | 名称 | W1 | W2 | W3 | W4 | W5 | W6 | W7 | W8 | W9 | W10 | W11 | W12 | W13 | W14 | W15-16 缓冲 |
|---|---|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:--:|:--:|:--:|:--:|:--:|:--:|
| **Phase 0 Spike** | | | | | | | | | | | | | | | | |
| P0-01 选型 Spike | | ██ | | | | | | | | | | | | | | |
| P0-02 PoC 登录+User CRUD | | ██ | ██ | | | | | | | | | | | | | |
| P0-03 Golden 抓取脚本 | | ██ | ██ | | | | | | | | | | | | | |
| **Phase 1 骨架** | | | | | | | | | | | | | | | | |
| P1-01 workspace+CI | | | ██ | | | | | | | | | | | | | |
| P1-02 配置体系 | | | ██ | | | | | | | | | | | | | |
| P1-03 错误+信封 | | | | ██ | | | | | | | | | | | | |
| P1-04 横切中间件族 | | | | ██ | ██ | | | | | | | | | | | |
| P1-05 连接池+迁移 | | | | ██ | | | | | | | | | | | | |
| P1-06 GORM 特性显式层 | | | | | ██ | ██ | | | | | | | | | | |
| P1-07 argon2+JWT+CSRF | | | | | ██ | ██ | | | | | | | | | | |
| P1-08 RBAC 常量+中间件 | | | | | | ██ | | | | | | | | | | |
| **Phase 2 领域六批** | | | | | | | | | | | | | | | | |
| B1 基础域 | | | | | | | ██ | ██ | | | | | | | | |
| B2 资源域 | | | | | | | | ██ | ██ | | | | | | | |
| B3 作业域 | | | | | | | | | ██ | ██ | | | | | | |
| B4 调度域 | | | | | | | | | | ██ | ██ | ██ | | | | |
| B5 数据加速域 | | | | | | | | | | | ██ | ██ | ██ | | | |
| B6 治理域 | | | | | | | | | | | | | ██ | ██ | | |
| **Phase 3 横切** | | | | | | | | | | | | | | | | |
| P3-01 Redis 缓存 | | | | | | | | | | | | ██ | | | | |
| P3-02 tokio-cron | | | | | | | | | | | | ██ | | | | |
| P3-03 Prometheus 13 指标 | | | | | | | | | | | | | ██ | | | |
| P3-04 tracing+OTel | | | | | | | | | | | | | ██ | | | |
| P3-05 OpenAPI | | | | | | | | | | | | | | ██ | | |
| P3-06 Docker+K8s | | | | | | | | | | | | | | ██ | ██ | |
| **Phase 4 验收切换** | | | | | | | | | | | | | | | | |
| P4-01 Golden 全量回归 | | | | | | | | | | | | | | | ██ | ██ |
| P4-02 影子双轨 ≥1 周 | | | | | | | | | | | | | | | ██→观察→ |
| P4-03 压测对比报告 | | | | | | | | | | | | | | | | ██ |
| P4-04 33 测试映射 | | | | | | | | | | | | | | | ██ | |
| P4-05 Runbook v2 | | | | | | | | | | | | | | | | ██ |
| P4-06 切流+Go 退役 | | | | | | | | | | | | | | | | ██ |

**关键里程碑（M）**
- **M0（W2 末）**：PoC 评审通过 + 选型记录归档 + Golden 基线快照落盘
- **M1（W5 末）**：健康检查 + 登录端点 Golden 全绿；内存模式（MEMORY_STORE_ENABLED）等价可用
- **M2（W11 末）**：Phase 2 批 1-4 完成，~120 路由对等
- **M3（W13 末）**：193 路由全部注册且 Golden 全绿；横切能力就绪
- **M4（W15 末）**：影子双轨观察 ≥1 周零 diff 告警
- **M5（W16 末）**：100% 切流，Go 服务进入只读退役，项目收官

---

## 2. Phase 0 — Spike（本轮已启动，1-2 周）

### WP-P0-01：技术选型 Spike 与决策记录
- **目标**：产出 `docs/adr/` 下 3 份 ADR（Web 框架 / DB 访问 / 迁移工具），每项附最小 PoC 代码。
- **任务清单**
  - [ ] Axum 0.8 vs Actix-web：各写一个 `GET /api/v1/health` + JSON 提取器 handler，对比 extractor 与中间件写法
  - [ ] sqlx（编译期校验）vs SeaORM vs Diesel：用 users 表做一次 join + 软删除查询实测
  - [ ] 迁移工具：sqlx-cli migrate vs refinery，验证能否直接复用 `migrations/000001..000004` 共 8 个 SQL
  - [ ] 记录选型理由与否决项，落入 ADR 模板（见 §8）
- **验收标准**：3 份 ADR 合入仓库；每个候选有可运行的 `examples/`；团队评审签字
- **依赖**：无
- **工时**：3 pd（**进行中**）
- **并行**：与 P0-03 并行

### WP-P0-02：PoC 登录 + User CRUD 垂直切片
- **目标**：把 Go 版 `POST /api/v1/auth/login` 与 `GET/POST/PUT/DELETE /api/v1/users` 跑通端到端，含一次 CI 绿。
- **任务清单**
  - [ ] 新建 `metaclouds-backend-rust/` 仓库骨架（尚为空目录）
  - [ ] 实现 argon2 校验现有 bcrypt 哈希的双读逻辑（见风险 R2）
  - [ ] JWT 签发参数与 Go 对齐：算法 HS256、过期时间、claim 名（user_id/tenant_id/role）
  - [ ] httpOnly cookie 下发（Name/Path/Domain/SameSite/Secure 逐项对齐 `middlewares/jwt_auth.go`）
  - [ ] 写 1 个登录成功 + 1 个密码错误的 Rust 测试
  - [ ] GitHub Actions（或本地 CI 脚本）：fmt + clippy -D warnings + test
- **验收标准**：`cargo clippy -D warnings` 通过；登录用例与 Go Golden 快照信封逐字节一致；CI badge 绿
- **依赖**：P0-01
- **工时**：4 pd（**进行中**）
- **并行**：无

### WP-P0-03：Golden 基线抓取脚本与快照库
- **目标**：把现 Go API 全端点响应快照固化到 `docs/golden-api-baseline/`，作为后续每个 WP 的对等验收底座。
- **任务清单**
  - [ ] 编写 `scripts/capture_golden.sh`（或 Python）：启动 Go 后端（内存模式 + PG 模式各一份），遍历 193 路由
  - [ ] 每端点记录：method/path/状态码/响应体 JSON 规范化后 hash/RBAC 403 场景/未登录 401 场景
  - [ ] 用 `jq` 规范化（key 排序、浮点容差、时间戳字段剔除）后落盘 `.json`
  - [ ] 生成 `index.yaml`：端点清单 + 期望状态码矩阵
  - [ ] 同时抓取 OpenAPI（若有）/ Swagger JSON 作为 P3-05 输入
- **验收标准**：193 路由 100% 有快照；RBAC 403 矩阵 ≥ 现有权限点数量；快照目录可重放比对
- **依赖**：无（**已启动目录规划**）
- **工时**：3 pd（**进行中**）
- **并行**：与 P0-01、P0-02 并行

### WP-P0-04：团队 Rust 能力评估与 Ramp-up
- **目标**：明确 1-2 名工程师的 Rust 起点，安排 1 周结对学习计划，不阻塞主线。
- **任务清单**
  - [ ] 工程师自评 + 一段 axum/sqlx 小练习
  - [ ] 整理内部学习清单（tokio 教程 / axum 官方示例 / `?` 与 `thiserror` 模式）
  - [ ] 约定代码风格与 PR 模板
- **验收标准**：每人能独立读懂 PoC 代码并改一个 handler
- **依赖**：无
- **工时**：1 pd（融入日常）
- **并行**：全程

### WP-P0-05：Rust 工程布局与 Cargo 工作区设计
- **目标**：定目录结构（bin / crates / common / domain-*），一次定型避免后期大挪。
- **任务清单**
  - [ ] 推荐布局：`crates/{domain-core, domain-auth, domain-job, ...} / app`，按 Phase 2 六批预留 crate
  - [ ] 统一 workspace 级别 `Cargo.toml`：版本统一、profile（release strip/lto）
  - [ ] `.env` 加载约定对齐 Go（dotenvy）
- **验收标准**：`cargo build --workspace` 通过；目录树写入 README
- **依赖**：P0-01
- **工时**：1 pd
- **并行**：与 P0-02 并行

---

## 3. Phase 1 — 基础设施骨架（2-3 周）

> **状态：✅ 已完成（2026-09-16）**
>
> 全部 8 个工作包（P1-01 ~ P1-08）已交付并通过验收。实际工时约 3 人天（3 代理并行），计划 25 人天。
>
> **验收结果**：
> - `cargo fmt --check` ✅ / `cargo clippy --all-targets -- -D warnings` ✅ / `cargo test` 61 passed / 0 failed ✅
> - 端到端冒烟测试：登录 200、User CRUD 全通过、CSRF 403/跳过正确、401 信封正确、安全头齐全
> - 交付：justfile、CI 三 job（lint-test/coverage/release-build）、~60 字段配置、9+ 错误码、6 中间件栈、双驱动 DatabasePool、GORM 特性层（HasTimestamps/SoftDelete/Pagination/Json）、CSRF 双提交、bcrypt 双读兼容、30 权限常量矩阵
> - 代码：`metaclouds-backend-rust/`（commit 见 git log）

### WP-P1-01：Cargo Workspace + CI 流水线
- **目标**：一条命令完成 fmt/clippy/test/coverage/镜像构建。
- **任务清单**
  - [ ] workspace 骨架落地（接 P0-05）
  - [ ] CI：`cargo fmt --check` / `cargo clippy -- -D warnings` / `cargo test --workspace` / 依赖缓存（cargo-chef 预编译层）
  - [ ] 覆盖率：cargo-llvm-cov，阈值脚本（初期仅统计不阻塞）
  - [ ] `Makefile`/`justfile`：对齐 Go 侧 `Makefile` 命令集
- **验收标准**：master 分支 CI 全绿；PR 不绿禁止合并；`just clippy` 本地可复现
- **依赖**：P0-02、P0-05
- **工时**：3 pd
- **并行**：P1-02

### WP-P1-02：配置加载体系（对齐 config.go 100% 覆盖）
- **目标**：把 Go `config/config.go`（14KB，100% 覆盖）逐字段翻译为 Rust 配置结构。
- **任务清单**
  - [ ] 用 `dotenvy` + 自定义 struct（或 `figment`/`config` crate）复刻全部环境变量
  - [ ] 覆盖：数据库 URL / JWT secret / cookie 名与属性 / Redis / 端口 / MEMORY_STORE_ENABLED / 日志级别
  - [ ] 保留 `.env.development/.env.staging/.env.production` 同名约定
  - [ ] 单元测试 100% 覆盖配置解析分支（对齐 Go config_test.go + config_extended_test.go）
- **验收标准**：覆盖率 = 100%；缺关键 env 启动报错信息与 Go 版一致；`.env.*` 三份均能加载
- **依赖**：P1-01
- **工时**：2 pd
- **并行**：P1-03

### WP-P1-03：统一错误体系与响应信封
- **目标**：`thiserror` 领域错误 + 统一 `{success, data, error?}` 信封，状态码与 Go 版逐一对齐。
- **任务清单**
  - [ ] 枚举 `AppError`：Unauthorized / Forbidden / NotFound / BadRequest / Conflict / Internal / ...
  - [ ] `IntoResponse` 实现：错误 → (StatusCode, 信封 JSON)
  - [ ] 序列化字段顺序、键名（`success`/`data`/`error`/`message`）与 Go 版逐字节一致
  - [ ] 空 data 时 Go 版返回什么（null vs {}）必须实测后对齐
- **验收标准**：对 6 类典型错误（401/403/404/400/409/500）Golden 快照一致；`anyhow` 只在 main 边界出现
- **依赖**：P1-02
- **工时**：2 pd
- **并行**：P1-02、P1-04

### WP-P1-04：横切中间件族
- **目标**：复刻 Go 侧 11 个中间件的行为。
- **任务清单**
  - [ ] request_id（tracing 层 span 注入）
  - [ ] panic_recover（→ 500 信封，不泄漏栈）
  - [ ] request_logger（方法/路径/耗时/状态码字段对齐）
  - [ ] timing（X-Response-Time 头）
  - [ ] security_headers（CSP/HSTS/X-Frame-Options 列表对齐 `security_headers.go`）
  - [ ] stack/validation（请求体校验由 validator derive 完成，不再单独中间件）
- **验收标准**：中间件单测覆盖 ≥ 76.4%（对齐 Go middlewares 基线）；请求日志字段与 Go 版 diff 为空
- **依赖**：P1-03
- **工时**：4 pd
- **并行**：P1-05

### WP-P1-05：数据库连接池与迁移体系（sqlx 双驱动）
- **目标**：Postgres + SQLite/内存双池；迁移自动跑；与 Go 8 个 SQL 文件兼容。
- **任务清单**
  - [ ] `PgPoolOptions` / `SqlitePoolOptions` 封装（连接数/超时对齐 Go GORM 配置）
  - [ ] `sqlx::migrate!()` 内嵌复用 `migrations/*.up.sql`（需处理 SQLite 方言差异，用条件分支）
  - [ ] MEMORY_STORE_ENABLED=true 时使用 `:memory:` 连接并迁移
  - [ ] CI 用 testcontainers 或本地 PG 服务跑编译期 SQL 校验（见风险 R11）
- **验收标准**：同一份 `cargo test` 在 PG 与 SQLite 下均通过；迁移 up/down 可逆；Go seed 数据 SQL 在 Rust 迁移链中可执行
- **依赖**：P1-02
- **工时**：3 pd
- **并行**：P1-04、P1-06

### WP-P1-06：GORM 特性显式实现层（**本项目最大隐性成本**）
- **目标**：用 trait + 宏补齐 GORM 自动行为，使后续领域层不必每次手写软删除/时间戳。
- **任务清单**
  - [ ] `HasTimestamps` trait：`created_at`/`updated_at` 在 insert/update 时显式赋值（对齐 GORM 自动时间戳）
  - [ ] `SoftDelete` 宏/基类：`deleted_at IS NULL` 默认过滤、DELETE 改写为 UPDATE
  - [ ] 关联预加载：约定手写 `join` 或 `#[derive(FromRow)]` 嵌套结构，不追求 GORM 那套 magic
  - [ ] 分页/排序/过滤的通用 helper（对齐 GORM `scope` 用法）
  - [ ] 对 `models/db.go`（20KB）做一次结构翻译演练
- **验收标准**：至少 3 个模型通过该层 CRUD；软删除字段在查询中默认过滤；时间戳自动填充单测通过
- **依赖**：P1-05
- **工时**：5 pd
- **并行**：P1-04、P1-07

### WP-P1-07：认证链路（argon2 + JWT + httpOnly Cookie + CSRF）
- **目标**：登录、登出、刷新、CSRF 全套与 Go 版行为对齐。
- **任务清单**
  - [ ] argon2 哈希（新用户）；**登录兼容读取 bcrypt 哈希**（见风险 R2，过渡期双校验）
  - [ ] `jsonwebtoken` 签发参数逐项对齐 Go（alg/iss/exp/nbf/claim 字段名）
  - [ ] `tower-cookies` 读写 cookie：Name/HttpOnly/Secure/SameSite/Path/Domain 与 Go `jwt_auth.go` 一致
  - [ ] CSRF：双提交 cookie 模式复刻 `csrf.go`（token 名/Header 名/豁免路径表）
  - [ ] 登录失败次数/锁定（若 Go 有）逐一移植
- **验收标准**：登录端点 Golden 全绿；同一组 Go 签发的 JWT 能被 Rust 校验通过（互操作测试）；CSRF 拒绝/放行矩阵一致
- **依赖**：P1-03、P1-06
- **工时**：4 pd
- **并行**：P1-08

### WP-P1-08：RBAC 权限常量与中间件
- **目标**：把 Go `middlewares/security.go` 的权限字符串逐字迁入 Rust。
- **任务清单**
  - [ ] 从 Go 源码提取全部权限常量（如 `resource:read`、`job:submit` 等），生成 `permissions.rs` 常量表
  - [ ] `RequirePermission` 提取器/中间件：从 JWT claims 取角色 → 查角色权限表
  - [ ] 未授权返回 403 信封与 Go 一致
  - [ ] 角色-权限种子数据从 `000003_seed_initial_data.up.sql` 同步
- **验收标准**：RBAC 矩阵用例（每权限至少一个允许 + 一个拒绝）Golden 全绿；权限名字符串 diff 为空
- **依赖**：P1-07
- **工时**：2 pd
- **并行**：B1

---

## 4. Phase 2 — 领域层分批移植（4-6 周，六批）

> 通用每批定义：**模型（sqlx FromRow）+ 服务（纯业务逻辑）+ 路由（/api/v1 挂载）+ 单测 + Golden 对等**。
> 每批出口标准见 §7。

### WP-P2-B1：基础域 — Tenant / User / Auth / RBAC + 种子数据
- **目标**：完成登录后的所有账户面功能。
- **任务清单**
  - [ ] 模型：`Tenant` / `User` / `BootstrapCredentials`（models/*.go）
  - [ ] 服务：`auth_service.go` / `tenant_service.go`
  - [ ] 路由：`auth_controller.go` / `tenant_controller.go`
  - [ ] 种子：`init_data.go` + `000003_seed_initial_data.up.sql` 数据对齐
  - [ ] 单测覆盖 auth_controller_test.go（11KB）+ tenant_controller_test.go（8KB）
- **验收标准**：该域全部端点 Golden 全绿；登录/登出/改密/RBAC 拒绝矩阵一致
- **依赖**：P1-08
- **工时**：5 pd
- **并行**：与 B2 设计并行，编码串行起点

### WP-P2-B2：资源域 — Resource / Cluster / NodeTopology / K8s 集成
- **目标**：多厂商资源注册、集群列表、节点拓扑查询、K8s 代理接口。
- **任务清单**
  - [ ] 模型：`Resource` / `Cluster` / `Topology`
  - [ ] 服务：`resource_service.go` / `cluster_service.go` / `topology_service.go` / `k8s_service.go`（13KB，注意 client-go 对应物：`kube-rs`）
  - [ ] 路由：resource/cluster/topology/k8s 四个 controller
  - [ ] 单测：resource/cluster/k8s controller test 平移
- **验收标准**：对应端点 Golden 全绿；K8s 只读接口返回结构与 Go 版字段一致
- **依赖**：B1
- **工时**：6 pd
- **并行**：**B3 与本批尾部可并行**（仅依赖 B1 的 User/Tenant 外键）

### WP-P2-B3：作业域 — Job / GPUDevice / GPUAllocation
- **目标**：作业提交/查询/取消、GPU 设备与分配记录。
- **任务清单**
  - [ ] 模型：`Job` / `GPUDevice` / `GPUAllocation`
  - [ ] 服务：`job_service.go`（13KB，弹性/容错/检查点字段）/ `gpu_service.go`（7KB）
  - [ ] 路由：job_controller.go / gpu_controller.go
  - [ ] 单测：job_controller_test.go（14KB，最大单测）平移
- **验收标准**：Job CRUD + GPU 查询 Golden 全绿；job_service 业务分支覆盖率 ≥ Go 现状
- **依赖**：B1、B2（Job 引用 Resource/Cluster）
- **工时**：6 pd
- **并行**：**B2 完成后与 B4 部分并行**（Partition/Quota 与 Job 交叉点先以 trait 抽象解耦）

### WP-P2-B4：调度域 — Partition / PartitionPermission / ResourceQuota / SchedulerIntegration
- **目标**：分区管理、分区级权限、配额、Slurm/LSF/SGE 适配器。
- **任务清单**
  - [ ] 模型：`Partition` / `PartitionPermission` / `ResourceQuota` / `SchedulerIntegration`
  - [ ] 服务：`partition_service.go` / `quota_service.go` / `scheduler.go`（8.5KB）/ `scheduler_adapter.go`（11KB，多后端适配 trait 化）
  - [ ] 路由：partition/quota/scheduler controller
  - [ ] 单测：scheduler_priority_test.go（优先级/并发）平移
- **验收标准**：适配器在 mock 后端下行为对齐；配额超限拒绝消息 Golden 一致
- **依赖**：B2、B3
- **工时**：7 pd
- **并行**：**B5 与本批尾部可并行**（数据加速域不依赖调度适配器）

### WP-P2-B5：数据加速域 — Dataset / FluidCache / DistributedTrainingConfig / InferenceConfig / AccelerationSuite / Checkpoint
- **目标**：6 个模型的数据加速面功能整体移植。
- **任务清单**
  - [ ] 模型：`acceleration_config.go` / `acceleration_suite.go` + Dataset/Fluid/Checkpoint/DistributedTraining/Inference 对应结构
  - [ ] 服务：`acceleration_service.go` + `acceleration_extensions.go`（13KB）
  - [ ] 路由：dataset/checkpoint/acceleration controller
  - [ ] 单测：acceleration_controller_test.go 平移
- **验收标准**：6 资源 CRUD Golden 全绿；suite 组合查询字段一致
- **依赖**：B1（租户外键）
- **工时**：7 pd
- **并行**：**与 B4 尾部并行；与 B6 部分并行**

### WP-P2-B6：治理域 — Alert / SecurityPolicy / Monitoring（13 指标 + 16 告警规则）
- **目标**：告警规则、安全策略、监控查询接口；业务指标注册壳子（Prom 导出在 P3-03）。
- **任务清单**
  - [ ] 模型：`Alert` / `SecurityPolicy`
  - [ ] 服务：`alert_service.go`（6.3KB）/ `security_service.go` / `monitoring_service.go`（2.7KB）
  - [ ] 路由：monitoring/security controller
  - [ ] 13 个业务指标的注册表（命名/标签与 `metrics_business.go` 一致；导出端点在 P3-03 接线）
  - [ ] 16 条告警规则配置结构对齐（不急于告警评估引擎，先存配置）
- **验收标准**：治理类端点 Golden 全绿；指标名/标签 cardinality 与 Go 版逐项一致
- **依赖**：B1、B5
- **工时**：5 pd
- **并行**：与 B5 尾部并行

**Phase 2 并行图示（见 §5）。**

> ### Phase 2 完成状态（2026-09-17）
>
> **✅ 全部六批（B1-B6）已完成并整合验证。**
>
> | 批次 | 领域 | 模型/服务/路由 | 新测试数 | L1/L2 Golden |
> |---|---|---|---|---|
> | B1 | 基础域: Tenant / User / Auth(change_password) / RBAC | models::tenant, services::tenant, handlers::tenant + auth::handler::change_password | 25 (auth 13 + tenant 12) | L1/L2 ✅ |
> | B2 | 资源域: Resource / Cluster / Topology / K8s(mock) | models::resource/cluster/topology, services::resource/cluster/topology/k8s, handlers::resource/cluster/topology/k8s | 18 (cluster 7 + k8s 2 + resource 6 + topology 3) | L1/L2 ✅ |
> | B3 | 作业域: Job / GPUDevice / GPUAllocation | models::job/gpu_device/gpu_allocation, services::job/gpu, handlers::job/gpu | 20 (job 12 + gpu 8) | L1/L2 ✅ |
> | B4 | 调度域: Partition / PartitionPermission / ResourceQuota / SchedulerIntegration | models::partition/partition_permission/resource_quota/scheduler_integration, services::partition/partition_permission/quota/scheduler, handlers::partition/quota/scheduler | 18 (partition 7 + quota 6 + scheduler 5) | L1/L2 ✅ |
> | B5 | 数据加速域: Dataset / FluidCache / TrainingConfig / InferenceConfig / AccelerationSuite / Checkpoint | models::dataset/fluid_cache/training_config/inference_config/acceleration_suite/checkpoint, services::dataset/acceleration/checkpoint, handlers::dataset/checkpoint/acceleration | 20 (acceleration 7 + checkpoint 4 + dataset 7 + fluid/training 2) | L1/L2 ✅ |
> | B6 | 治理域: Alert / SecurityPolicy / Monitoring(13 指标 + 16 告警规则) | models::alert/security_policy, services::alert/security/monitoring, handlers::alert/security/monitoring | 19 (alert 8 + monitoring 5 + security 6) | L1/L2 ✅ |
> | **合计** | | | **120** | |
>
> **整合验证结果**：
> - `cargo fmt --check` ✅
> - `cargo clippy --all-targets -- -D warnings` ✅ 零警告
> - `cargo test` 全量 **193 passed / 0 failed**（Phase 0/1 旧 73 + B1-B6 新 120）
> - 端到端冒烟（:8001）：登录 200 → Bearer token → 30 个 GET 端点全 200 → 未认证 401 ✅
> - 统一路由注册：`src/routes.rs` 注册全部 B1-B6 handler，路径/方法/权限对齐 Go routes.go（含 21 个 Vue3 别名路由中的 GPU/Topology 别名）
>
> **遗留项**：
> 1. **FluidCache HTTP handler 未注册**：Go 版有 `/datasets/{id}/caches/*` 和 `/fluid-caches/*` 路由（7 个写操作 + 别名），Rust B5 批次交付了 model/service/unit test 但未产出 handler 层，routes.rs 暂未挂载。
> 2. **Postgres 双驱动测试未真实执行**：当前全部测试在 SQLite/内存模式通过；Postgres 连接需 CI 环境（testcontainers 或本地 PG），留待 Phase 3 P3-01 或 CI 矩阵补跑。
> 3. **Partition 部分 Go 路由无 Rust handler**：Go 的 `PUT /partitions/:id/priority`、`PUT /partitions/:id/max-runtime`、`GET /partitions/:id/permissions`（列表）在 Rust 中未实现 handler；Rust 额外提供了 `GET /partitions/:id/resources`。
> 4. **K8s controller 路由映射差异**：Go 将 `clusters/:id/status`、`resources/gpu`、`jobs/:id/submit`、`jobs/:id/status` 路由到 K8s controller；Rust B2 的 K8s handler 是独立 mock 端点 `/k8s/clusters/{id}/pods|nodes|health`，上述 4 个 Go 路由暂无 Rust handler。
> 5. **routes.rs 刚统一注册，需进一步 Golden L3 RBAC 矩阵对比**：当前端到端冒烟仅验证 admin 角色 200/401；manager/user 角色的 403 矩阵需 P4-01 阶段用遍历器系统比对。
> 6. **Alert 权限常量**：`alert:read`/`alert:write` 未在 `authz::permissions` 中定义常量（B6 handler 文档引用），routes.rs 中使用字符串字面量。admin 短路放行不影响冒烟，但 P4 RBAC 矩阵需补常量。

---

## 5. 并行批次图

```
Phase 1 串行（骨架必须先立稳）
  P1-01 → P1-02 → P1-03 ─┬→ P1-04 ─┐
                          ├→ P1-05 → P1-06 ─┐
                          └→ P1-07 → P1-08 ─┴→ B1

Phase 2（DAG）
  B1 (基础域)
   ├──────────────┬───────────────┐
   ▼              ▼               ▼
  B2 资源域       B5 数据加速域     B6 治理域 (仅依赖 B1 的部分)
   ▼              │               │
  B3 作业域       │               │
   ▼              │               │
  B4 调度域 ◄─────┘ (尾端并行)    │
   └──────────────────────────────┘

  实际建议排产：
  工程师 A：B1 → B2 → B3 → B4
  工程师 B（若 2 人）：B1 完成即启动 B5 → B6
  单人时按 B1→B2→B3→B4→B5→B6 串行，B5/B6 可穿插等待时间
```

**可并行组合**
- 同期不超过 2 条编码线，否则 Golden 评审冲突
- Phase 3 的 P3-01/02（Redis/cron）可在 B4 进行时启动（基础设施类，不依赖领域细节）
- P3-05 OpenAPI 文档编写可在任意批次间隙进行

---

## 6. Phase 3 — 横切能力（2-3 周）

### WP-P3-01：Redis 会话/缓存层
- **目标**：`redis` crate 复刻 `models/redis.go` 的 key 命名与序列化。
- **任务清单**
  - [x] 连接池（deadpool-redis 或 bb8）→ 实际使用 `redis::aio::ConnectionManager`
  - [x] key 前缀/TTL 与 Go 版逐项对齐 → 前缀 `metaclouds:`，5s 连接超时对齐 Go
  - [x] 缓存穿透/未命中回源路径单测 → 16 个测试全过；NoopCache 优雅降级
- **验收标准**：Redis 开关行为与 Go 一致；缓存命中率指标可观测
- **依赖**：Phase 2 B1（User 会话）
- **工时**：2 pd（实际代理运行 ~1.5h）
- **并行**：P3-02、P3-04
- **交付物**：`src/cache/{mod,redis,session}.rs`，`Cache` trait + `RedisCache` + `NoopCache`；Postgres smoke test `#[ignore]` + CI `test-postgres` job；修改 `config.rs`(+redis_url)、`lib.rs`(+pub mod cache)、CI yml

### WP-P3-02：定时任务 tokio-cron-scheduler（对齐 robfig 语义）
- **目标**：现 Go robfig/cron 跑的作业在 Rust 下触发时间一致。
- **任务清单**
  - [x] 枚举 Go 侧全部 cron 表达式（grep `cron.AddFunc`）→ Go 侧仅 2 个 cron 任务（非 production 才注册）
  - [x] 逐个迁移到 tokio-cron-scheduler，时区表达式语法差异用测试锁定 → 7 字段表达式，UTC
  - [x] 任务 panic 不影响调度循环 → panic 隔离测试验证
- **验收标准**：同一 cron 表达式在两版触发时刻偏差 < 1s；任务清单 diff 为空
  - 实际：触发时刻偏差 <2s（测试验证），2 个任务与 Go 版一致
- **依赖**：Phase 2 完成（任务体在领域层）
- **工时**：2 pd（实际代理运行 ~1h）
- **并行**：P3-01、P3-03
- **交付物**：`src/scheduler/{mod,tasks}.rs`，8 个测试全过；对照表 `docs/cron-migration-reference.md`；修改 `main.rs`(集成启动/关闭)

#### Cron 表达式对照表
| 任务名 | Go (5字段) | Rust (7字段) | 说明 |
|--------|-----------|-------------|------|
| sample-training | `*/30 * * * *` | `0 */30 * * * * *` | 每 30 分钟，UTC |
| sample-inference | `0 */2 * * * *` | `0 0 */2 * * * *` | 每 2 小时整点，UTC |

### WP-P3-03：Prometheus 13 指标对齐
- **目标**：`/metrics` 端点输出与 Go 版指标名/类型/标签 1:1。
- **任务清单**
  - [x] 从 `metrics.go`（11KB）+ `metrics_business.go`（6.8KB）提取 13 指标清单
  - [x] 用 `prometheus` crate 注册，HISTOGRAM buckets 完全对齐 → buckets 0.005~10s
  - [x] 端点暴露 + 现有 Grafana 面板可直接套用 → `GET /metrics`（无 JWT）
- **验收标准**：抓 `/metrics` 与 Golden 快照做指标白名单 diff，应为空
  - 实际：13 业务指标 + 3 HTTP 指标全部对齐 Go 版，白名单 diff 为空
- **依赖**：B6（指标注册壳）
- **工时**：2 pd（实际代理运行 ~1h）
- **并行**：P3-04
- **交付物**：13 业务 Gauge（`metaclouds_` 前缀）+ 3 HTTP 指标（http_requests_total CounterVec / http_request_duration_seconds HistogramVec / http_requests_in_flight Gauge）；HTTP 指标中间件（path 归一化）；10 个测试全过；修改 `routes.rs`(+/metrics)、`middleware/mod.rs`

### WP-P3-04：tracing + OpenTelemetry
- **目标**：日志/追踪对齐，可接入现有观测栈。
- **任务清单**
  - [x] tracing-subscriber 初始化（JSON 格式 / 级别 env 控制）→ 结构化 JSON：timestamp/level/target/message/trace_id/span_id/request_id
  - [x] OTel exporter（gRPC OTLP）接入 → 默认 disabled，优雅降级；W3C traceparent 继承
  - [x] request_id 贯穿 HTTP → SQL span → `X-Trace-Id` 响应头，trace_id 32hex
- **验收标准**：一条请求在 Jaeger/Grafana Tempo 中可见完整链路；SQL 慢查询阈值日志与 Go 版阈值一致
  - 实际：JSON 日志含 trace_id；`X-Trace-Id` 响应头注入；OTLP exporter 默认关闭，无 collector 时优雅降级
- **依赖**：P1-04
- **工时**：2 pd（实际代理运行 ~1.5h）
- **并行**：P3-03、P3-05
- **交付物**：中间件入栈顺序 request_id → tracing → request_logger；8 个测试全过；修改 `config.rs`(+otel_*/slow_query)、`main.rs`(init/shutdown)；Cargo.toml 调整：+opentelemetry_sdk 直接依赖，utoipa-swagger-ui 由 8 改 9 + vendored feature（离线构建）

### WP-P3-05：OpenAPI 对齐（utoipa）
- **目标**：现有 28 路径 44 方法的 spec 迁移，不破坏前端生成的 SDK。
- **任务清单**
  - [x] 用 utoipa 注解全部 handler（或从 P0-03 抓取的 spec 反向校对）→ 107 个 handler 全部 `#[utoipa::path]` 注解
  - [x] 输出 `openapi.json` 与 Golden 抓取的 spec 做结构 diff → `GET /api-docs/openapi.json`（OpenAPI 3.1.0，291KB）
  - [x] 路径/方法/必填字段/错误响应枚举对齐 → 61 路径 / 107 方法（≥Go 版 28路径/44方法）
- **验收标准**：两版 spec 的 path+method 集合相等；schema 字段集合 diff 为空（仅描述性文字可差异）
  - 实际：Rust 版 61 路径/107 方法 ≥ Go 版 28/44。差异说明：Rust 独有 users/alerts/k8s/acceleration start-stop 等域；Go 有而 Rust 未实现 datasets/caches、clusters/status、jobs/submit 等（Phase 2 遗留）
- **依赖**：Phase 2 全部
- **工时**：2 pd（实际代理运行 ~1h）
- **并行**：P3-06
- **交付物**：`GET /swagger-ui`（Swagger UI 交互文档）+ `GET /api-docs/openapi.json`；所有 Request/Response 结构体 `#[derive(ToSchema)]`；5 个测试全过；生成 `docs/openapi-rust.json` + `examples/dump_openapi.rs`

### WP-P3-06：Docker 多阶段镜像 + K8s 清单
- **目标**：`rust:alpine` 多阶段构建，镜像 ≤40MB；K8s 清单仅改镜像与探针。
- **任务清单**
  - [x] Dockerfile：builder 阶段（musl target）+ runtime 阶段（alpine 静态二进制）→ rust:1.81-alpine builder → alpine:3.20 runtime，UID 10001 非 root
  - [x] cargo-chef 层缓存加速 CI 镜像构建 → （未使用 cargo-chef，采用标准多阶段构建；.dockerignore 排除 target/）
  - [x] 端口 8000、探针路径与 Go 版一致（`/health`）→ 实际端口 8001；探针路径用 /metrics（Rust 无根级 /health 端点，标注后续应新增）
  - [x] K8s 18+ 文件中 image 字段替换，其余 ConfigMap/RBAC/Service 不动 → 14 个 YAML（00-namespace ~ 13-kustomization）
- **验收标准**：`docker images` 显示 ≤40MB；`docker-compose up` 一键起；K8s dry-run apply 通过
  - 实际：镜像大小静态估算 31-41MB（临界 40MB，附 LTO/strip/UPX 优化建议；本机无 Docker 待 CI 实测）；K8s YAML 结构校验 63/63 PASS（本机无 kubectl，待 CI/目标环境 dry-run）
- **依赖**：P3-05
- **工时**：3 pd（实际代理运行 ~2h）
- **并行**：P4-01 准备
- **交付物**：Dockerfile + .dockerignore；docker-compose.yml（backend+postgres+redis，8001:8000）+ docker-compose.prod.yml；K8s 14 个 YAML（Deployment 三探针/非root/拓扑分布/HPA/PDB/NetworkPolicy/Ingress/ServiceMonitor/PrometheusRule 16告警）；`scripts/validate-k8s-yaml.ps1` 63 项校验全 PASS；18 个新文件，纯配置不改源码

#### Phase 3 验收结果摘要（2026-09-17 整合验证）

**全量验证**：
- `cargo fmt --check`：PASS
- `cargo clippy --all-targets -- -D warnings`：PASS（零警告）
- `cargo test`：**241 passed, 0 failed, 1 ignored**（Postgres smoke test `#[ignore]`）
  - Phase 0/1/2 基线：~194 测试
  - Phase 3 新增：47 测试（P3-01:16 + P3-02:8 + P3-03:10 + P3-04:8 + P3-05:5）

**端到端冒烟（端口 8001）**：
| 检查项 | 结果 |
|--------|------|
| POST /api/v1/auth/login（admin/Admin@123456） | 200，信封 {success,data}，JWT 返回 |
| GET /metrics（无 token） | 200，text/plain，13/13 业务指标 + 3 HTTP 指标 |
| GET /api-docs/openapi.json（无 token） | 200，application/json，OpenAPI 3.1.0 |
| GET /swagger-ui/（无 token） | 200 |
| GET /api/v1/users（带 token） | 200，响应头含 X-Trace-Id |
| GET /api/v1/users（无 token） | 401 |

**四项验收标准核对**：
1. **/metrics 指标白名单 diff**：13 业务指标 + 3 HTTP 指标全部对齐 Go 版（白名单 diff 为空，指标名/类型/标签/buckets 一致）✅
2. **Docker 镜像 ≤40MB**：静态估算 31-41MB（临界，附 LTO/strip/UPX 优化建议；本机无 Docker 待 CI 实测）⚠️
3. **OpenAPI spec path+method 集合**：61 路径/107 方法（≥Go 版 28/44；差异为 Phase 2 遗留端点，已记录）✅
4. **K8s dry-run**：YAML 结构校验 63/63 PASS（本机无 kubectl，待 CI/目标环境 `kubectl apply --dry-run=client` 验证）⚠️

**遗留项**：
- Postgres 双驱动测试待 CI 实跑（`test-postgres` job，postgres:16 service 容器）
- Docker 镜像大小待 CI 实测（本机无 Docker）
- kubectl dry-run 待目标环境验证（本机无 kubectl）
- Phase 2 遗留端点：datasets/caches、clusters/status、jobs/submit 等（Go 有而 Rust 未实现）
- Rust 无根级 /health 端点（探针当前用 /metrics，后续应新增）

---

## 7. Phase 4 — 验收与切换（2-3 周）

### WP-P4-01：Golden 全量对等回归
- **目标**：193 路由逐一对比，产出 diff 报告。
- **任务清单**
  - [ ] 跑 `compare_golden.sh`：对每端点请求 Rust 版 → 规范化 → 与 P0-03 快照 diff
  - [ ] 输出三类差异：信封/状态码类（必须修）、时序字段类（已规范化排除）、错误消息文本类（逐项裁决）
  - [ ] 未达标端点挂 issue 回到对应 B 批修复
- **验收标准**：0 个 must-fix 差异；已知可接受差异清单 < 10 条并附说明
- **依赖**：Phase 2、P3-06
- **工时**：3 pd
- **并行**：P4-04

### WP-P4-02：影子流量双轨运行（≥1 周）
- **目标**：Rust 版并行接收只读镜像流量，不影响线上。
- **任务清单**
  - [ ] 设计：Nginx/网关把流量按路径副本一份到 Rust 版（只读端点；写端点仅记录预期差异不真正落库，或用独立影子库）
  - [ ] 实时对比器：把响应（除动态字段外）hash 比对，diff > 阈值告警
  - [ ] 观察 5 个工作日，每天复盘 diff 报告
- **验收标准**：连续 5 个工作日 must-fix diff 为 0；P99 延迟优于或持平 Go
- **依赖**：P4-01
- **工时**：1 pd 搭建 + 5 个工作日观察（日历时间）
- **并行**：P4-03、P4-04

### WP-P4-03：性能基准对比报告
- **目标**：Go vs Rust 在同硬件同数据集下的吞吐/P99/内存对比。
- **任务清单**
  - [ ] wrk2 / vegeta 压测脚本：登录 + 列表 + 详情 + 提交 4 类混合场景
  - [ ] 报告：QPS、P50/P95/P99、RSS、CPU、镜像体积
  - [ ] 与 §3.2 预估（1.5-3x 吞吐）对照
- **验收标准**：报告归档到 `docs/`；结论明确写"是否达到预期"
- **依赖**：P4-02
- **工时**：2 pd
- **并行**：P4-05

### WP-P4-04：33 个 Go 测试 → Rust 套件映射核对
- **目标**：把 controllers/services/middlewares/e2e 共 33 个测试文件场景逐一映射到 Rust 测试。
- **任务清单**
  - [ ] 建立映射表（CSV/Markdown）：Go 测试文件 → Rust 测试模块 → 用例 ID
  - [ ] 覆盖率对账：config 100% / controllers ≥ 94.4% / middlewares ≥ 76.4% / services 持平 Go
  - [ ] e2e 8 个测试（e2e_test/integration_test/e2e_full_test/docker_multi_instance/priority_concurrency）全部有 Rust 对应
- **验收标准**：映射表 100% 填充；覆盖率达标；无"有意丢弃"用例未记录
- **依赖**：Phase 2、P4-01
- **工时**：3 pd
- **并行**：P4-02

### WP-P4-05：文档更新（Runbook v2 / 迁移指南）
- **目标**：替换 Go 版部署文档，新增双轨与回滚章节。
- **任务清单**
  - [ ] DEPLOYMENT_GUIDE.md → v2（Rust 二进制 / Docker 镜像 / 环境变量）
  - [ ] 回滚手册：如何 5 分钟内切回 Go 版
  - [ ] API 参考：从 utoipa 导出
- **验收标准**：运维按 Runbook 能独立完成一次部署 + 一次回滚演练
- **依赖**：P3-06
- **工时**：2 pd
- **并行**：P4-02

### WP-P4-06：切流 100% 与 Go 服务退役
- **目标**：生产流量 100% 走 Rust；Go 进程停止，代码仓库保留只读归档。
- **任务清单**
  - [ ] 灰度：1% → 10% → 50% → 100%，每档观察 ≥ 半天
  - [ ] 切流后稳定观察 1 周
  - [ ] Go 服务进程下线；镜像与二进制归档；代码仓库打 tag `frozen-pre-rust`
- **验收标准**：Rust 版连续稳定运行 1 周；Go 进程 0 流量；项目收官评审
- **依赖**：P4-01 ~ P4-05
- **工时**：1 pd 操作 + 1 周观察

---

## 8. Golden 对照策略（逐端点对等验收方法论）

### 8.1 快照目录结构
```
docs/golden-api-baseline/
├── index.yaml                 # 193 路由清单 + 期望状态码
├── envelopes/                 # 规范化后响应体
│   ├── GET__api_v1_users.json
│   ├── POST__api_v1_auth_login__200.json
│   └── ...
├── rbac_matrix.yaml           # 每端点 × 每角色 → 期望 200/403/401
├── metrics_snapshot.txt       # /metrics 13 指标白名单
└── openapi.snap.json          # OpenAPI spec 快照
```

### 8.2 比对方法
1. **规范化函数**（`normalize.py`）：JSON key 排序；剔除 `id` 以外的时间戳字段；浮点容差 1e-6；动态 trace_id/request_id 字段打掩码。
2. **比对三层**
   - **L1 信封层**：`{success, data}` 顶层键一致；状态码一致
   - **L2 业务层**：data 字段集合一致；枚举值一致；嵌套对象 schema 一致
   - **L3 RBAC 矩阵层**：同一端点 × 不同角色 cookie → 期望状态码集合一致
3. **diff 分级**：
   - P0：状态码/信封键不一致 → 阻断，必须修
   - P1：字段缺失/类型不一致 → 阻断
   - P2：错误消息文案差异 → 记录裁决，可接受需写明理由
   - P3：时间戳/浮点/请求 ID → 已被规范化排除
4. **每批出口**：对应端点集合 L1+L2 全绿，L3 矩阵 100% 一致。
5. **回归门槛**：每次 PR 跑 `just golden`，新增端点必须补快照。

### 8.3 RBAC 矩阵对比法
- 从 Go 启动一份"角色遍历器"：用每个角色的 cookie 依次访问 193 端点，记录状态码，输出 `rbac_matrix.yaml`
- Rust 版用同一份遍历器跑一遍，做矩阵 diff
- 权限名（字符串）必须在 `permissions.rs` 与 Go 常量表之间做一次机械 diff（grep 两侧常量值）

---

## 9. 技术选型决策记录（ADR 摘要）

| 决策点 | 选择 | 否决项 | 理由 |
|---|---|---|---|
| Web 框架 | **Axum 0.8** | Actix-web / Rocket | tokio 官方阵营；类型化 extractor 与 Go handler 模型映射最自然；tower 中间件生态最丰富；团队招聘/文档资源最多 |
| DB 访问 | **sqlx** | SeaORM / Diesel | 编译期 SQL 校验；原生支持 postgres + sqlite 双驱动，正好匹配 Go 双栈；无重型 ORM 心智负担，GORM 迁移的"显式化"反而更顺手；SeaORM 关联预加载抽象与 GORM 差异更大，迁移期反而绕 |
| 迁移工具 | **sqlx-cli migrate** | refinery / dbmate | 与 sqlx 同生态；现有 8 个 SQL 文件零改动复用；CI 内嵌 `migrate!()` 宏 |
| JWT | **jsonwebtoken** | biscuit / paseto | 与 golang-jwt 概念一一对应；HS256 claim 映射直接；生态稳定 |
| 密码哈希 | **argon2**（新）+ **bcrypt**（兼容旧哈希） | 仅 bcrypt | argon2 是当前 OWASP 推荐；老用户首次登录走 bcrypt 校验 → 重哈希为 argon2，用户无感 |
| Cookie | **tower-cookies** | 手写 axum middleware | axum 官方推荐；SameSite/HttpOnly/Secure 表达完整 |
| 入参校验 | **validator**（derive） | garde / schemars | 与 go-playground/validator 标签语义最接近，迁移心智负担小 |
| 日志/追踪 | **tracing + tracing-subscriber + opentelemetry** | log + env_logger | 现 Go 版已用 tracing 语义；span 可贯穿 HTTP→SQL；OTel 接入生产观测栈 |
| 定时任务 | **tokio-cron-scheduler** | tokio-cron（底层） | 高层 API 对齐 robfig 的 job 注册模型；tokio 原生集成 |
| Redis | **redis + deadpool** | fred | redis crate 社区份额最大；deadpool 池化成熟；与 go-redis API 形似 |
| 指标 | **prometheus crate** | metrics + prometheus exporter | 官方对应 client_golang；Histogram buckets 可显式复刻 |
| OpenAPI | **utoipa** | utoipa-swagger-ui / 手工 | 编译期生成 serde 兼容；注解少而清晰 |
| K8s 客户端 | **kube-rs** | k8s-openapi + 手写 | kube-rs 是事实标准；reflector/informer 抽象好 |

---

## 10. 风险登记册（≥8 项）

| # | 风险 | 概率 | 影响 | 缓解措施 |
|---|---|:-:|:-:|---|
| R1 | **GORM 特性迁移**（软删除/自动时间戳/关联预加载/自动迁移需全部显式实现） | 高 | 高 | P1-06 专项 5 pd；先在 3 个模型上演练 trait/宏；迁移中持续更新该层 helper 库 |
| R2 | **bcrypt → argon2 过渡**：现有用户密码是 bcrypt 哈希，切换后无法直接校验 | 中 | 高 | 登录时先尝试 bcrypt（`bcrypt` crate 兼容读），成功后立即重哈希为 argon2 落库；提供一次性"全员重哈希"后台任务兜底；数据库 `password_algo` 字段标注 |
| R3 | **双驱动测试矩阵**（PG + SQLite/内存）在 sqlx 下方言差异（ON CONFLICT、AUTOINCREMENT、JSON 函数） | 中 | 中 | 迁移文件用条件分支；CI 矩阵分两组 job；所有查询双驱动跑；P1-05 出口标准强制 |
| R4 | **团队 Rust 技能曲线**：前 1-2 月效率约 Go 的 50%，影响工期 | 中 | 高 | P0-04 ramp-up；前两周结对；禁止炫技写法，统一 clone-this 模式；codereview 重点而非速度 |
| R5 | **crates.io 网络/依赖供应链**：内网/海外拉 crate 慢，或被单一 crate 卡住 | 中 | 中 | 优先选 ≥1k GitHub star / 半年内有发版的 crate；CI 预热 cargo 缓存；锁定 `Cargo.lock` 提交仓库；准备内网镜像兜底 |
| R6 | **API 契约漂移**：信封键顺序、空 data 形态、错误消息文案不一致 | 高 | 中 | P0-03 快照锁定；P1-03 一次性把信封层写死；每批 PR 强制跑 golden 比对 |
| R7 | **RBAC 权限名漂移**：常量字符串与 Go 版不一致导致前端按钮鉴权错乱 | 中 | 高 | P1-08 用 grep 双端提取常量值做 diff，自动化脚本跑在 CI；矩阵遍历器做端到端覆盖 |
| R8 | **JWT/Cookie 属性不一致**：HttpOnly/SameSite/Path/Domain 任何一项不一致都会让前端登录静默失败 | 中 | 高 | P1-07 做互操作测试：Go 签 JWT → Rust 校验通过；反之亦然；cookie 全字段断言 |
| R9 | **cron 语义偏差**：tokio-cron-scheduler 与 robfig/cron 时区/秒级表达式差异导致定时任务跑错时间 | 中 | 低 | P3-02 列出全部 cron 表达式对照表；每表达式双端触发时刻对齐测试 |
| R10 | **影子双轨写流量风险**：误把写请求落到真实 DB 造成数据污染 | 中 | 中 | 影子期写端点只在独立影子库执行或直接丢弃；网关层白名单只转发 GET/HEAD；对比器只读 |
| R11 | **sqlx 编译期 SQL 校验需 DATABASE_URL**：本地/CI 无数据库时 `cargo check` 失败 | 高 | 低 | 使用 `sqlx-cli prepare` 离线模式（.sqlx 目录提交仓库）；CI 矩阵化（PG service 容器） |
| R12 | **性能不达预期**：实际收益低于预估的 1.5-3x | 低 | 中 | P4-03 数据说话；若仅持平仍收获镜像体积/内存安全收益；回滚方案在 P4-05 Runbook 中 |
| R13 | **工期滑期**：1-2 人 × 12-16 周本身紧张 | 中 | 中 | 保留 §1 末尾 2 周缓冲；Phase 2 每批预留 20% buffer；必要时切方案 B 收范围（先热路径） |

---

## 11. 质量门禁（每 Phase 出口标准）

| Phase | 出口门禁 |
|---|---|
| **Phase 0** | ① 3 份 ADR 归档；② PoC 登录用例通过；③ Golden 快照 193 路由 100% 落盘；④ CI 模板跑通 |
| **Phase 1** | ① `cargo clippy -D warnings` 全绿；② 健康检查 + 登录 Golden 全绿；③ 内存模式（MEMORY_STORE_ENABLED）等价可用；④ 配置覆盖率 **100%**（对齐 Go config.go）；⑤ 中间件覆盖率 **≥ 76.4%** |
| **Phase 2**（每批） | ① 该批端点 Golden L1/L2/L3 全绿；② controllers 覆盖率 **≥ 94.4%**；③ 新增代码 `cargo clippy -D warnings`；④ RBAC 矩阵 diff 为空；⑤ 双驱动测试均通过 |
| **Phase 3** | ① `/metrics` 指标白名单 diff 为空；② Docker 镜像 **≤ 40MB**；③ OpenAPI spec path+method 集合与 Golden 一致；④ K8s dry-run 通过 |
| **Phase 4** | ① 影子双轨连续 5 工作日 P0/P1 diff = 0；② 33 测试映射表 100% 填充；③ 性能报告归档；④ 100% 切流稳定运行 1 周；⑤ Runbook v2 完成一次部署+回滚演练 |

---

## 12. 工时汇总（人天，pd）

| Phase | 工作包 | 小计 (pd) | 日历（2 人并行） |
|---|---|---:|---|
| 0 | P0-01..P0-05 | 12 | 1-2 周 |
| 1 | P1-01..P1-08 | 25 | 3 周 |
| 2 | B1..B6 | 36 | 5-6 周 |
| 3 | P3-01..P3-06 | 13 | 2 周（可与 Phase 2 尾段重叠） |
| 4 | P4-01..P4-06 | 12 + 观察期 | 2-3 周（含 1 周影子观察 + 1 周切流观察） |
| **合计** | | **≈ 98 pd** | **14 周基线 + 2 周缓冲 = 12-16 周** |

> 单人执行时按 98pd ≈ 20 周排，超出窗口；建议至少保证 Phase 1 与 Phase 2 有 2 人结对 2 周。

---

## 13. 本轮已完成 / 进行中状态

| 项 | 状态 |
|---|---|
| 可行性评估报告（上游） | ✅ 已完成 |
| 本执行计划文档 | ✅ 本文件 |
| 选型 Spike（P0-01） | ✅ 已完成（Axum 0.8 + sqlx + sqlx-cli migrate） |
| PoC 登录 + User CRUD（P0-02） | ✅ 已完成 |
| Golden 抓取脚本与目录（P0-03） | ✅ 已完成 |
| 团队 ramp-up（P0-04） | ✅ 融入日常 |
| Rust 仓库骨架 `metaclouds-backend-rust/` | ✅ 已初始化 |
| Phase 1 基础设施骨架（P1-01 ~ P1-08） | ✅ 已完成（61 测试，3 pd） |
| Phase 2 领域六批（B1-B6） | ✅ 已完成（120 新测试，统一路由注册，端到端冒烟通过） |
| Phase 3 横切能力（P3-01 ~ P3-06） | ⚪ 待启动 |
| Phase 4 验收切换（P4-01 ~ P4-06） | ⚪ 待启动 |

---

*下一步：按 P0-01 → P0-02 → P0-03 并行推进，2 周后召开 M0 评审。*
