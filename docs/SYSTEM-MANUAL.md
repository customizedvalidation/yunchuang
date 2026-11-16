# Metaclouds 系统说明书

> **文档版本**：v1.0 · **编制日期**：2026-11-16 · **适用版本**：前后端 `2.0.1`
> **事实口径**：本文所有版本号、端点、菜单、状态均来自源码/配置实测，而非 README 的历史描述
> （后端 `README.md` 部分字段已过时，以本文「§3 技术架构」标注的实际版本为准）。
>
> **一句话定位**：Metaclouds 是一套「GPU/算力集群调度管理平台」的前后端 monorepo，
> 提供统一的 Web 控制台管理作业、GPU、分区、集群、多租户与监控告警。
> **当前处于演示/联调形态**：管理闭环真实可用，但算力执行体、K8s 编排、监控时序为模拟实现，
> 系统通过常驻横幅对此诚实声明（详见「§11 演示模式与能力边界」）。

---

## 目录

1. [系统概述](#1-系统概述)
2. [快速开始](#2-快速开始)
3. [技术架构](#3-技术架构)
4. [功能模块全览](#4-功能模块全览)
5. [认证与权限](#5-认证与权限)
6. [API 参考](#6-api-参考)
7. [数据存储与数据模型](#7-数据存储与数据模型)
8. [部署与运维](#8-部署与运维)
9. [CI/CD 与安全门禁](#9-cicd-与安全门禁)
10. [可观测性](#10-可观测性)
11. [演示模式与能力边界（重要）](#11-演示模式与能力边界重要)
12. [已知限制与未决台账（R1–R11）](#12-已知限制与未决台账r1r11)
13. [验证与质量状态](#13-验证与质量状态)
14. [常见问题与故障排查](#14-常见问题与故障排查)
15. [附录：文档索引与术语表](#15-附录文档索引与术语表)

---

## 1. 系统概述

### 1.1 它是什么

Metaclouds（元云）是一套面向 GPU/算力场景的**集群调度管理平台**。用户通过一个 Web 控制台完成：

- 作业的创建、排队、取消与提交；
- GPU 设备的细粒度分配（MIG/vGPU）与利用率查看；
- 计算分区、集群、调度器集成与拓扑感知配置；
- 数据集与 Fluid 缓存加速、加速套件管理；
- 多租户配额、监控告警与安全策略治理。

### 1.2 仓库形态

单一 monorepo，根目录 `D:/YCYD`，远端 `github.com/customizedvalidation/yunchuang`，主分支 `main`。

```
D:/YCYD
├── metaclouds-frontend-vue/   # Vue3 前端（MDS v3.1 设计系统）
├── metaclouds-backend-rust/   # Rust/axum 后端（唯一主后端）
├── scripts/                   # 部署、密钥生成、前置校验、digest 钉死脚本
├── .github/workflows/ci-cd.yml# CI/CD 流水线（10 实跑 job + 6 部署 job）
├── docs/                      # 复盘/设计/运维/API 文档
├── docker-compose.yml         # 全栈本地/联调编排（9 个服务）
├── prometheus.yml / alerts.yml / alertmanager.yml
└── .env.example / .env.deploy.example   # 配置模板（真实值不入库）
```

> 历史上曾规划拆分前端仓库，**已决策保持 monorepo 统一**：`docker-compose.yml`、Prometheus 配置、
> `init.sql` 横跨前后端，拆分会破坏编排完整性。

### 1.3 当前成熟度（诚实摘要）

| 维度 | 状态 | 说明 |
|---|---|---|
| 管理后台 UI | 🟢 完善 | MDS v3.1，浅/深/跟随系统三主题，真实浏览器核验 |
| 菜单/路由结构 | 🟢 完善 | 20 个菜单 key 与路由一一对应，无死链 |
| 核心业务 CRUD | 🟢 基本完善 | 14 个模块中 8 个实现真实增删改查闭环 |
| 认证与 RBAC | 🟢 完善 | JWT + CSRF + 三层权限口径一致 |
| 算力执行体 | 🔴 模拟 | `sleep(2s)` 后置完成，实耗合成 |
| K8s 编排 | 🔴 模拟 | `MockK8sClient` 固定节点/Pod |
| 监控时序 | 🔴 模拟 | `Math.sin` 合成趋势 |
| 生产部署 | 🟡 就绪未启用 | `DEPLOY_ENABLED` 关闭，缺真实凭据与 kubeconfig |

---

## 2. 快速开始

### 2.1 本地启动（开发模式）

**后端**（默认监听 `8000`；本机预编译二进制实测为 `8001`）：

```bash
cd metaclouds-backend-rust
# 方式 A：源码运行（首次会从 rsproxy.cn 镜像拉依赖）
cargo run
# 方式 B：使用预编译二进制（需在后端目录内启动，否则 migrations 路径解析失败）
./target/release/metaclouds-backend-rust.exe
```

> ⚠️ 坑：预编译二进制必须在 `metaclouds-backend-rust/` 目录内启动。从仓库根启动会因
> `migrations/` 相对路径找不到而退出（实测 `exit 1`）。

**前端**（Vite dev server，端口 `3000`，`/api` 代理到后端）：

```bash
cd metaclouds-frontend-vue
# Windows 平台需关闭 safe-delete 拦截（否则 Vite 重优化缓存清理被拦截）
CODEBUDDY_SAFE_DELETE_ENABLED=0 \
VITE_API_PROXY_TARGET=http://127.0.0.1:8001 \
npm run dev -- --host 127.0.0.1 --port 3000
```

打开 `http://127.0.0.1:3000/login`。

### 2.2 登录

- 管理员账号：`admin`
- 初始口令由 `DEFAULT_ADMIN_PASSWORD` 决定；**口令不写入本文/版本库**，请从环境配置
  （`.env` / 部署 Secret，均已 gitignore）获取。非生产未设置时回退默认值，生产未设置则
  fail-secure 拒绝播种并退出。
- 登录页右上角可切换外观：浅色 / 深色 / 跟随系统。

### 2.3 常用开发命令

| 命令 | 作用 |
|---|---|
| `npm run type-check` | 前端 TypeScript 类型检查 |
| `npm run test` | 前端 Vitest 回归（当前 74 用例） |
| `npm run build` | 前端生产构建（含 `vue-tsc`） |
| `cargo fmt --check` | 后端格式检查 |
| `cargo clippy --all-targets -- -D warnings` | 后端静态检查（零告警门禁） |
| `cargo test` | 后端测试（本地实测 48 套件 / 200 passed） |
| `just <cmd>` | 后端常用命令封装（需安装 just） |

---

## 3. 技术架构

### 3.1 技术栈一览（实际版本）

**前端**（`metaclouds-frontend-vue/package.json`，v2.0.1）：

| 类别 | 技术 / 版本 |
|---|---|
| 框架 | Vue 3.5 + TypeScript 5.6 |
| UI 库 | Element Plus 2.8（全量皮肤）+ @element-plus/icons-vue 2.3 |
| 状态 | Pinia 2.2 |
| 路由 | Vue Router 4.4（fail-closed 守卫） |
| 图表 | ECharts 5.5（统一 `mc-light` / `mc-dark` 双主题） |
| 构建 | Vite 5.4 + vue-tsc 2.1 |
| 测试 | Vitest 3.2.6 + @vue/test-utils 2.5 + jsdom 30 |
| 设计系统 | MDS v3.1（`tokens.ts` ↔ `index.css` 双向同步） |

**后端**（`metaclouds-backend-rust/Cargo.toml` + `Cargo.lock`，v2.0.1）：

| 类别 | 技术 / 版本 |
|---|---|
| HTTP | axum 0.8 + tokio 1 |
| 存储 | SQLx 0.8.6（SQLite + PostgreSQL **双驱动**，运行时查询） |
| 认证 | jsonwebtoken 9（HS256）+ argon2 0.5 + bcrypt 0.15（迁移双读） |
| Cookie | tower-cookies 0.11 |
| 校验 | validator 0.20 |
| 缓存 | redis 0.27（ConnectionManager） |
| 定时任务 | tokio-cron-scheduler 0.10 |
| 指标 | prometheus 0.14 |
| 链路追踪 | tracing + opentelemetry 0.27 + otlp + tracing-opentelemetry 0.28 |
| API 文档 | utoipa 6 + utoipa-swagger-ui 10（OpenAPI **3.1.0**，MSRV 1.88 / CI 固定 rustc 1.96） |
| 错误/工具 | thiserror 2 + anyhow 1 + chrono 0.4 + uuid 1 |

> 注：utoipa 6 连带将 build-dependency `zip` 提升至 9.5.1，从而真实消除了 4 条 RustSec 豁免；
> `validator 0.20` 带来 `idna 1.1.0`、`prometheus 0.14` 带来 `protobuf 3.7.2`，又各消除 1 条。
> 详见「§12 R11」。

### 3.2 后端分层

```
src/
├── main.rs / lib.rs       入口（配置加载、服务器、调度器、tracing）
├── config.rs              环境变量加载与校验（约 65 字段，与 Go 版 config.go 1:1）
├── db.rs                  DatabasePool（Sqlite/Postgres）、migrations、admin 播种
├── routes.rs              全部领域路由装配 + /metrics + /swagger-ui
├── middleware/            横切链：request_id → tracing → request_logger → metrics
│                          → timing → security_headers → error_handler → panic_recover
├── auth/                  password / jwt / csrf / middleware / handler
├── authz/mod.rs           30 个权限常量 + 角色-权限矩阵
├── cache/                 Redis 缓存层 + session jti 撤销
├── scheduler/             tokio-cron-scheduler 定时任务
├── models/                领域模型（sqlx FromRow + 响应 DTO）
├── services/              业务逻辑（纯逻辑，不含 HTTP）：含 dispatch（调度闭环）、
│                          k8s（MockK8sClient）、monitoring、priority_scheduler 等
├── handlers/              HTTP 提取器 + 统一响应封装
└── orm/mod.rs             GORM 兼容层（时间戳/软删除/分页/JSON 列）
```

### 3.3 数据流与中间件链

```
浏览器 ──HTTPS──> frontend(nginx) ──/api 反代──> backend(axum)
                                                    │
   request_id → tracing → request_logger → metrics → timing → security_headers
   → error_handler → panic_recover → JWT 鉴权 → 权限校验(RBAC) → handler → services → DB/Redis
```

- CSRF 在 `jwt_auth` 之后对受保护路由叠加，仅校验写方法（POST/PUT/DELETE/PATCH），
  Bearer 客户端豁免。
- 时区：API 时间统一 ISO 8601 / Unix 秒；定时任务使用 UTC。

### 3.4 部署拓扑

`docker-compose.yml` 编排 9 个服务，全部只绑定回环地址（数据库/缓存/中间件不暴露到宿主机网卡）：

| 服务 | 镜像 | 端口（宿主） | 角色 |
|---|---|---|---|
| backend | GHCR `backend-rust:sha-<commit>` | 127.0.0.1:8000 | 主后端 |
| frontend | GHCR `frontend:sha-<commit>` | 127.0.0.1:8080 | Nginx 反代 + SPA |
| postgres | postgres:16-alpine | 127.0.0.1:5432 | 关系库（切换 PG 后启用） |
| redis | redis:7-alpine | 127.0.0.1:6379 | 缓存/会话 |
| prometheus | prom/prometheus:v2.48.0 | 127.0.0.1:9090 | 指标采集 |
| alertmanager | prom/alertmanager:v0.26.0 | 127.0.0.1:9093 | 告警投递 |
| grafana | grafana/grafana:10.2.3 | 127.0.0.1:3001 | 可视化 |
| etcd | quay.io/coreos/etcd:v3.5.9 | 127.0.0.1:2379/2380 | 分布式键值 |
| jaeger | jaegertracing/all-in-one:1.53 | 127.0.0.1:16686 等 | 链路追踪 |

K8s 部署另有 19 个 YAML 清单（00-namespace → 18-redis），含 Deployment/Service/HPA/PDB/
NetworkPolicy/Ingress/ServiceMonitor/PrometheusRule，见「§8.2」。

---

## 4. 功能模块全览

菜单单一真源：`src/nav.ts`（14 一级项 + 6 子项 = 20 key），分组为 6 组。
Sidebar 菜单、Topbar 面包屑、router 守卫全部从该文件派生。

| 分组 | 菜单项 | 路径 | 核心能力 | 完成度 |
|---|---|---|---|---|
| 总览 | 仪表盘 | `/dashboard` | 9 个 GET 聚合、KPI、跳转、自动刷新 | 🟢 |
| 作业调度 | 作业管理（列表/队列/历史） | `/job/*` | 建/取消/提交 K8s/详情/checkpoint | 🟢 |
| 基础资源 | 资源管理 | `/resource` | 列表、编辑、厂商概览、筛选 | 🟢 |
| 基础资源 | GPU 设备 | `/gpus` | 增改删、分配/释放、维护、记录 | 🟡 隔离卡片为展示 |
| 基础资源 | 分区管理 | `/partitions` | CRUD、优先级、运行时长、权限增删 | 🟢 |
| 集群管理 | 集群管理 | `/cluster` | CRUD、重建、扩容、详情、搜索、分页 | 🟢 |
| 集群管理 | K8S 管理（节点/Pod/服务） | `/k8s/*` | 三 Tab 数据/搜索/分页 | 🔴 语义借用（已标注） |
| 集群管理 | 调度器 | `/schedulers` | CRUD、队列/节点、同步、健康检查 | 🟢 |
| 集群管理 | 拓扑感知 | `/topology` | CRUD、评分、搜索、筛选 | 🟡 字段口径待核 |
| 加速套件 | 加速套件 | `/acceleration` | 列表、编辑、启停、详情 | 🟡 无新增/删除 |
| 加速套件 | 数据集 | `/datasets` | 数据集 + Fluid 缓存 CRUD、启停/预取 | 🟢 |
| 系统治理 | 监控告警 | `/monitoring` | 指标、告警解决/忽略（真实写库） | 🔴 趋势合成 |
| 系统治理 | 多租户管理 | `/tenant` | 租户 + 多维配额 CRUD、用量 | 🟢（仅 admin/manager） |
| 系统治理 | 安全管理 | `/security` | 策略列表、编辑、启停、规则详情 | 🟡 无新增/删除 |

**统计**：核心 CRUD 闭环 8 项 🟢；部分受限 5 项 🟡；能力缺口 2 项 🔴。

> 详细的逐页功能穿透、真实缺陷与修复记录见
> [`docs/MENU-FUNCTIONAL-AUDIT-2026-11-16.md`](MENU-FUNCTIONAL-AUDIT-2026-11-16.md)。

---

## 5. 认证与权限

### 5.1 认证方式

| 方式 | 适用 | 说明 |
|---|---|---|
| JWT Bearer | 非浏览器客户端 | `Authorization: Bearer <token>`，CSRF 豁免 |
| HttpOnly Cookie | 浏览器客户端 | `access_token`（HttpOnly + Secure(生产) + SameSite）+ `csrf_token`（非 HttpOnly） |

- 令牌经 `POST /api/v1/auth/login` 获取，有效期 `JWT_EXPIRATION_HOURS`（默认 24h），
  刷新令牌默认 168h。
- 生产 Cookie 标记 `Secure`；`COOKIE_SAME_SITE` 默认 `lax`。

### 5.2 CSRF 双提交

- Cookie `csrf_token`（非 HttpOnly，同源 JS 可读）+ 请求头 `X-CSRF-Token`。
- 仅当存在 `access_token` Cookie 时，对写方法做常量时间比对；Bearer 客户端豁免。

### 5.3 RBAC 三层口径

角色：`admin` / `manager` / `user`；`admin` 短路所有权限。30 个权限常量（如
`cluster:read/write`、`job:write/submit`、`gpu:read/write`、`alert:write` 等）。

**授权边界（关键）**：

- 前端 `localStorage['user']`（含 `role`）**仅用于菜单/按钮展示过滤与路由守卫，不是凭证、不可信**。
  用户自行篡改只改变本地界面，不会获得任何服务端权限。
- 真正的授权边界在服务端：受保护路由经 `jwt_auth` + `RequirePermission` 校验（fail-closed），
  并对非 admin 强制租户隔离。
- 菜单/守卫/权限三处从 `nav.ts` 同源取值；`/tenant` 仅 admin/manager 可见。

> 因此新增后端端点必须自带权限校验，不得依赖前端隐藏入口充当授权。

---

## 6. API 参考

完整规范由后端运行时导出：`GET /api-docs/openapi.json`（OpenAPI **3.1.0**，约 61 paths / 107 methods）
与 `GET /swagger-ui/`。下文为摘要，权威以 OpenAPI 为准。

### 6.1 约定

- 基础路径：`/api/v1`（开发 `http://localhost:8000/api/v1`）。
- 响应封装：成功 `{ "success": true, "data": {...}, "timestamp": <unix秒> }`；
  失败 `{ "success": false, "message": "...", "code": "...", "timestamp": ... }`。
- 错误码→HTTP：`BAD_REQUEST/VALIDATION_ERROR`=400、`UNAUTHORIZED`=401、`FORBIDDEN`=403、
  `NOT_FOUND`=404、`CONFLICT`=409、`RATE_LIMIT_EXCEEDED`=429、
  `INTERNAL_SERVER_ERROR`=500、`SERVICE_UNAVAILABLE`=503。
- 列表分页参数 `page`（默认 1）/ `page_size`（默认 10，最大 100）。

### 6.2 端点分组摘要

| 分组 | 代表端点 | 写权限 |
|---|---|---|
| auth | `/auth/login` `/auth/logout` `/auth/refresh` `/auth/profile` `/auth/csrf` `/auth/change-password` | JWT |
| users | `/users`（CRUD，软删除） | admin |
| tenants | `/tenants` | `tenant:write` |
| clusters | `/clusters`、`/clusters/{id}/status` | `cluster:write` |
| resources | `/resources` | `resource:write` |
| topology | `/topology`、`/topology/nodes`（Vue3 别名） | `topology:write` |
| k8s | `/k8s/clusters/{id}/{pods,nodes,health}`（mock） | JWT（演示） |
| jobs | `/jobs`、`/jobs/stats`、`/jobs/{id}/cancel` | `job:write` |
| gpus | `/gpus`、`/gpus/allocations`、`/gpus/utilization` + `/gpu/*` 别名 | `gpu:write`/`job:write` |
| partitions | `/partitions`、`/partitions/{id}/{priority,max-runtime,permissions}` | `partition:write` |
| quotas | `/quotas`、`/quotas/{id}/check` | `quota:write` |
| schedulers | `/schedulers`、`/{id}/{sync,test-connection}` | `scheduler:write` |
| datasets | `/datasets`、`/datasets/{id}/caches` + Fluid 启停/预取 | `dataset:write` |
| checkpoints | `/checkpoints` | `checkpoint:write` |
| acceleration | `/acceleration`、`/{id}/{start,stop}` | `acceleration:write` |
| alerts | `/alerts`、`/{id}/{acknowledge,resolve}` | `alert:write`（仅 admin） |
| security | `/security/policies`、`/{id}/{enable,disable}` | `security:write` |
| monitoring | `/monitoring/dashboard`（39 指标族）、`/metrics`（16 规则）、`/alert-rules/evaluate` | `monitoring:write` |
| 可观测 | `/health`（公开）、`/metrics`（公开）、`/swagger-ui/`（公开） | — |

> 部分后端能力前端无菜单入口（如 `/users` CRUD、改密、告警规则），属产品决策，非 bug。

---

## 7. 数据存储与数据模型

### 7.1 双驱动

- 默认 SQLite 文件库（`sqlite:metaclouds.db?mode=rwc`），测试用 `sqlite::memory:`。
- 生产 `USE_SQLITE=false` 走 PostgreSQL（`DATABASE_URL=postgresql://...`）。
- 迁移双份：`migrations/*.sql`（SQLite，15 个）与 `migrations/postgres/`（语义等价）。
- `DatabasePool` 枚举统一两种驱动；建表由后端启动时跑 migrations 完成（不再挂载 `init.sql`，
  避免 password vs password_hash 双 schema 冲突）。

### 7.2 核心领域模型

用户/租户、集群、资源、拓扑节点、作业、GPU 设备/分配、分区/分区权限、配额、调度器集成、
数据集/Fluid 缓存、checkpoint、训练/推理配置、加速套件、告警、安全策略、K8s 对象。

ORM 兼容层提供：`HasTimestamps`（自动时间戳）、`SoftDelete`（`deleted_at` 过滤）、
`Json<T>`（SQLite TEXT / PG JSONB）、分页结构。

### 7.3 关键迁移

`014_dispatch_lease.sql`（作业租约选主）、`015_usage_source.sql`（标记实耗来源）为纯增量、
双驱动各一份，部署前需确认方言目录。

---

## 8. 部署与运维

### 8.1 CI/CD 流水线

`.github/workflows/ci-cd.yml`：push/PR 到 `main` 触发。实跑 10 个 job（含前后端 lint/test/build、
Trivy 镜像扫描、K8s 清单校验、cargo audit、gitleaks、checkov、部署总闸），6 个部署 job 在开关关闭时 skipped。

- 部署总闸三态：`DEPLOY_ENABLED≠true` → skipped（正常）；声明启用但缺配置 → `deploy-prereqs`
  exit 1（一次报全）；配齐 → 只部署 `DEPLOY_TARGETS` 列出的目标。
- 当前 `main == origin/main`，最近一次 UI 提交 `869ddd1` 的 CI run `43382670733` = **success**
  （10 实跑 job 全绿 / 6 部署 skipped）。

### 8.2 部署方式

**compose（联调/单节点）**：`docker compose up -d`，CI/生产用 GHCR 镜像 `sha-<commit>` 钉死；
保留 build 段供本地离线 `--build`。目标机不编译，只 `git pull --ff-only` 取编排后 `docker compose pull/up`。

**K8s（多副本）**：19 个 YAML 清单 + `kustomization.yaml`（规范文件名）。部署前用
`scripts/render-k8s-secret.sh` 从 CI secrets 渲染真实 Secret 工作副本，避免占位值覆盖。

### 8.3 部署前置配置

唯一真源：`scripts/check-deploy-prereqs.sh`（`--list` 打印完整变量清单）。

- 总闸：`DEPLOY_ENABLED`、`DEPLOY_TARGETS`（含 production 必须同时含 staging）、`REQUIRE_PG_HA`。
- compose 部署：三环境 `*_BASE_URL` / `*_SSH_KEY` / `*_HOST` / `*_USER` + `DEPLOY_DIR`。
- K8s 部署：`KUBE_CONFIG`（base64）、6 个 K8s Secret（JWT/DATABASE_URL/Redis/Admin/PG/Protected token），
  PG 口令必须与 `DATABASE_URL` 同源。

四步落地顺序：本地 strict 预检 → `--all-gh-commands` 全量注入 → `--verify-gh` 只列名称核验 → 人工开开关。

### 8.4 上线前必复核四件事

1. 生产 ConfigMap 当前以**演示集群**口径显式承认模拟，作为真实平台交付前必须改 `false`
   并停 `DISPATCH_ENABLED`（或接入真实执行器后移除模拟路径）。
2. `GET /api/v1/system/mode` 应返回 `simulated=false`，前端 ModeBanner 消失。
3. `jobs.usage_source='simulated'` 的合成实耗禁止进入计费/showback。
4. 实际镜像 tag 必须是 `sha-<commit>`；`verify-deployment.sh` 返回后端 JSON 信封而非 SPA HTML。

---

## 9. CI/CD 与安全门禁

- **Secret 扫描**：gitleaks（提交前扫描，凭据不入库）。
- **基础设施扫描**：checkov（`CKV_DOCKER_7` 要求字面镜像 tag，已据此回退 ARG 参数化）。
- **镜像扫描**：Trivy image scan + SARIF；因 GHA `cache-from` 会冻结安全更新层，两个 Dockerfile
  引入 `ARG SECURITY_UPDATES_REVISION=${{ github.run_id }}` 强制该层每次失效，避免「代码没改昨天绿今天红」。
- **依赖审计**：npm audit（vitest 已升至 3.2.6 消除 critical）、cargo audit（RustSec advisory-db，
  当前仅剩 2 条豁免：libsqlite3-sys 待 sqlx 升级、rsa 待 sqlx 0.8→0.9 跨大版本）。
- **生产 fail-closed**：`JWT_SECRET` ≥48 且非弱值黑名单；compose 强制注入
  `POSTGRES_PASSWORD`/`REDIS_PASSWORD`/`IMAGE_TAG`；生产未承认演示模式则拒绝启动。

---

## 10. 可观测性

- **`/health`**：根级存活探针（公开），返回 `{status, version, uptime}`，K8s 三探针均走它。
- **`/metrics`**（公开）：39 个指标族，均为 `metaclouds_*` 前缀——16 个业务指标 + 3 个 HTTP 指标 + 20 个调度指标（以源码 `register_*!` 宏调用数为准）
  （`http_requests_total` / `http_request_duration_seconds` / `http_requests_in_flight`），
  动态路径段归一化为模板，避免标签基数爆炸。
- **日志**：JSON 结构化，含 `timestamp/level/target/message/trace_id/span_id/request_id`；
  慢查询阈值可配 `DB_SLOW_QUERY_THRESHOLD_MS`。
- **追踪**：W3C `traceparent` 透传，响应头 `X-Trace-Id`；OTLP gRPC 导出默认关闭（未设 endpoint 时 no-op），
  Jaeger 在 compose 中可用。
- **缓存**：Redis 键前缀 `metaclouds:`，不可用时 `NoopCache` 优雅降级；会话 jti 撤销
  默认 fail-open（可用性优先），高安全环境可设 `REVOCATION_FAIL_CLOSED=true`。
- **定时任务**：`tokio-cron-scheduler`，7 段 cron（秒在前），默认两个 sample job 仅非生产注册。

---

## 11. 演示模式与能力边界（重要）

> **这是使用本系统最需要理解的一点。** 系统的管理闭环是真实的，但「算力」本身目前是模拟的。

| 能力 | 现状 | 代码/配置证据 |
|---|---|---|
| 作业执行体 | `sleep(sim_ms)` 后直接置 completed | `services/dispatch.rs` |
| 作业实耗 | `申请 × 0.75` 合成，标记 `usage_source='simulated'` | `services/dispatch.rs` + `015_usage_source.sql` |
| K8s 节点/Pod | 固定 mock 数据，非 API Server | `services/k8s.rs::MockK8sClient` |
| 监控趋势 | `Math.sin` 合成，吞吐 `i%3` 规律 | `MonitoringAlert.vue::buildTrend` |
| 前端提示 | 常驻 ModeBanner 横幅明示演示模式 | `GET /api/v1/system/mode` 驱动 |
| K8S 页面 | 三个 Tab 加显著「非真实 Kubernetes 对象」标注 | `K8SManagement.vue` |
| 生产防护 | 生产未设 `ALLOW_SIMULATED_EXECUTION` 则拒绝启动 | `config.rs::validate()` |

**这意味着**：

- 你可以真实地创建作业、分配 GPU、配置租户配额、处理告警——这些操作会持久化到数据库；
- 但作业"运行完成"和"用了多少 GPU 时"是模拟的，不能用于计费或 showback；
- K8S 管理页展示的是语义借用的数据（节点=GPU 资源池、Pod=运行中作业、服务=资源池定义），
  已在页面顶部明示；
- 监控系统有真实的告警规则与 UI 交互，但趋势图是合成曲线。

系统选择**诚实标注**而非伪造，这是刻意的设计决策。

---

## 12. 已知限制与未决台账（R1–R11）

图例：🔴 需外部条件 / 🟡 设计已定待环境 / 🟢 闭环 / ⚪ 部分完成。
权威真源：`docs/system-fixes-2026-11-09.md` 第五节 + 8.x。

| 项 | 内容 | 状态 | 解除条件 |
|---|---|---|---|
| R1 | 真实执行器（kube-rs Job/CRD）、DCGM 实耗 | 🟡 | 契约已成文（`real-executor-contract.md`），**未写实现**；需真实集群+GPU/DCGM+系统定位拍板 |
| R2 | 执行任务 DB 持久化 + worker lease/心跳 | 🟡 | 契约已定 `exec_tasks`，未建表未改代码；与 R1 同批 |
| R3 | reconcile owner 运行期幂等 | ⚪ | 启动期已用租约选主，运行期未做 |
| R4 | P2-3 抢占分支回归与合并 | 🟡 | 回归清单已成文，建议与 R1 同批 |
| R5 | PostgreSQL HA | 🔴 | 单点已标注；需托管 PG / CloudNativePG |
| R6 | 基础镜像 digest 钉死 | 🟡 | 沙箱无 Docker/crane 无法 resolve；联网环境跑 `pin-base-images.sh --resolve` |
| R7 | CI 首个完整 run 全绿 | 🟢 | run `42769493670` 闭环 |
| R8 | 真实凭据注入 + K8s 部署可用 | 🔴 | 缺真实主机/SSH/kubeconfig；`DEPLOY_ENABLED` 仍关闭 |
| R9 | K8s 客户端仍是 MockK8sClient | 🔴 | `K8S_SIMULATION_MODE=false` 时读端点已 fail-closed；待真实 kube-rs |
| R10 | 生产仍在跑模拟执行 | 🔴 | 已显式化 fail-closed；待 R1 完成后关闭开关 |
| R11 | RustSec 豁免收敛 | ⚪ | 8→4→**2**（zip/idna/protobuf 已真实升级消除）；余 libsqlite3-sys 待 sqlx 升级、rsa 待 sqlx 0.9 |

**其他未覆盖项**（见各专项文档）：Playwright 视觉回归基线（依赖未安装且 CDN 502）、
其余 12 个宽表的列优先级、i18n、组件文档站。

---

## 13. 验证与质量状态

| 检查 | 结果 |
|---|---|
| 前端 type-check | 0 错误 |
| 前端 Vitest | **74/74 通过**（含权限口径、诚实性标注、双主题、轮询等加固测试） |
| 前端 build | 成功 |
| 后端 fmt / clippy | PASS（零告警） |
| 后端 cargo test | 本地实测 48 套件 / 200 passed / 0 failed（doc-tests 0） |
| CI run `43382670733` | success（10 实跑 job 全绿 / 6 部署 skipped） |
| 真实浏览器 | 登录/仪表盘/集群/监控 × 浅色+深色 DOM 与截图核验 |

> 质量纪律：所有新增测试均为**加固而非放宽门禁**；安全升级走真实版本升级 + 对照实验，
> 从未用 `--ignore` 或放宽断言伪装通过。

---

## 14. 常见问题与故障排查

| 现象 | 原因 | 处理 |
|---|---|---|
| 预编译后端启动 `exit 1` | 工作目录不对，migrations 路径加载失败 | 在 `metaclouds-backend-rust/` 目录内启动 |
| 登录页 Vite 反复重优化 | safe-delete 拦截缓存清理 | 加 `CODEBUDDY_SAFE_DELETE_ENABLED=0` |
| manager 点"解决/忽略" 403 | 后端 `ALERT_WRITE` 仅 admin | 前端已收窄按钮到 admin，属预期 |
| 作业"完成"但无真实耗时 | 执行体为模拟 | 见「§11」，非 bug |
| K8S 页数据像 GPU 列表 | Tab 语义借用 | 页面已标注，非真实 K8s 对象 |
| 部署 job 全 skipped | `DEPLOY_ENABLED≠true` | 正常状态；配置完成后再开 |
| `Deployment Preflight` 红 | 声明启用但配置不全 | 按日志变量名补齐，用 `--strict` 本地复现 |
| 后端启动即退出（生产） | 未承认演示模式 / 缺 `DATABASE_URL`/`DEFAULT_ADMIN_PASSWORD` | fail-closed 设计；见 `deployment-prerequisites.md` |
| `verify-deployment.sh` 红 | `/health/ready` 拿到 SPA HTML | 确认新版 nginx 镜像 + Ingress `/health` 生效 |

---

## 15. 附录：文档索引与术语表

### 15.1 相关文档

| 文档 | 内容 |
|---|---|
| `docs/MENU-FUNCTIONAL-AUDIT-2026-11-16.md` | 全菜单功能完成度穿透 |
| `docs/UI-DESIGN-REVIEW-2026-11-13.md` | MDS v3.0/v3.1 设计复盘、双主题、P2 收敛 |
| `docs/system-fixes-2026-11-09.md` | 修复总报告、R1–R11 台账、8.1–8.15 |
| `docs/metaclouds-optimization-roadmap.md` | 优化路线图与各轮横幅 |
| `docs/deployment-prerequisites.md` | 部署前置条件真源（变量、状态机、注入核验） |
| `docs/real-executor-contract.md` | 真实执行器接入契约（设计稿，未实现） |
| `docs/api-reference.md` / `metaclouds-backend-rust/docs/api-reference-rust.md` | API 参考 |
| `metaclouds-backend-rust/README.md` | 后端架构/README（**部分版本字段过时**，以本文为准） |
| `docs/rbac-permission-convergence.md` | RBAC 权限收敛 |
| `docs/p2-3-preemption-backlog.md` | 抢占分支回归清单 |

### 15.2 术语表

| 术语 | 含义 |
|---|---|
| MDS | Metaclouds Design System，本系统设计系统（当前 v3.1） |
| 双驱动 | 同一套代码同时支持 SQLite 与 PostgreSQL |
| fail-closed | 条件不满足时拒绝服务而非降级放行 |
| fail-secure | 生产环境配置缺失时拒绝播种/启动，而非用弱默认值 |
| 语义借用 | 页面 Tab 名与实际数据来源不一致（K8S 三 Tab），已明示 |
| 合成实耗 | 非真实采集、按公式估算的用量（`usage_source='simulated'`） |
| MockK8sClient | 后端 K8s 只读 mock 实现，返回固定节点/Pod |
| GHCR | GitHub Container Registry，存放前后端镜像 |

---

> **免责声明**：本文描述的是系统**当前实际状态**，其中标注为"模拟/未实现/部分完成"的部分
> 会随后续 R1–R11 推进而变化。使用前请结合「§11 能力边界」与最新 CI 状态判断。
> 本文不含任何明文口令与真实凭据。
