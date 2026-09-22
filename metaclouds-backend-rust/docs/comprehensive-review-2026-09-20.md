# Metaclouds Rust 后端 — 全面复盘报告

> 日期：2026-09-20
> 基线 commit：`98ead4b`（复盘前 HEAD）
> 范围：Rust 全量重构 Phase 0-4 + 遗留项 + PostgreSQL 双驱动可用 + CI 全绿后的一次现状盘点与 P1 修复。

---

## 1. 执行摘要

Rust 版后端（`metaclouds-backend-rust`）相对 Go 版（`metaclouds-backend`）的全量重构已完成主体目标：

- **迁移范围**：Phase 0-4（骨架 → 认证/RBAC → 领域路由 B1-B6 → 回归对齐 → 遗留项）+ PostgreSQL 双驱动（SQLite/Postgres）可用。
- **CI**：根目录 `ci-cd.yml` 共 **14 个 job**（security-scan / backend-test / frontend-test / docker-build-backend / docker-build-frontend / deploy-development / deploy-staging / production-approval / deploy-production / verify-deployment / rust-lint-test / rust-test-postgres / rust-coverage / rust-release-build），Rust 相关 4 个 job 全绿。
- **测试**：Rust 单测 **296 passed**（本次改动仅文档/配置/CI，未触碰 Rust 源码，测试数无变化）。
- **路由规模**：Rust 116 个 `.route()` 调用；Go 129 个 HTTP 方法注册调用。差异已分类归因（见 §4.4）。
- **前端**：Vue3 版 15 个页面齐全，科技蓝主题 `#1677ff` 已在全局 `src/styles/index.css` 配置。

本次复盘在本机完成 3 项 P1 修复（.gitignore 覆盖率产物、CI 冗余 env、API 文档对齐），其余阻断性/目标环境依赖项列入待办。

---

## 2. 问题清单（分级）

### P0（阻断性，必须在切生产前解决）
本轮盘点**未发现本机可修复的 P0 源码/构建阻断问题**（cargo fmt / clippy / test 全绿，CI 绿）。
下列项需目标环境，仍属"上线前阻断"，见 §6：
- 无真实 K8s / 真实 PostgreSQL 环境完成端到端冒烟。
- 部署 secrets（DEV/STAGING/PROD SSH key、host、user）未配置，部署 job 仅能跑占位域名 curl。
- Docker 镜像 `push: false`，无镜像发布通道。

### P1（应修复）
| # | 问题 | 状态 |
|---|------|------|
| P1-1 | `.gitignore` 未排除 Rust 覆盖率产物（`priority_scheduler_coverage/`、`*.profraw`、`llvm-cov/`），CI 产物污染工作区 | ✅ 本次已修复 |
| P1-2 | `verify-deployment` job 声明了未使用的 `STAGING_SSH_KEY` env（该 job 无任何 ssh 步骤） | ✅ 本次已删除 |
| P1-3 | `api-reference-rust.md` 与 `routes.rs` 不一致：缺 14 个路由、1 个路径写错 | ✅ 本次已对齐 |
| P1-4 | 部署 secrets 未配置（DEV/STAGING/PROD SSH key + host + user） | ⏳ 待目标环境 |
| P1-5 | Go 后端未退役（影子观察 + 灰度切流未开始） | ⏳ 待目标环境 |

### P2（建议改进，不阻断）
| # | 问题 | 建议 |
|---|------|------|
| P2-1 | `DELETE /partitions/permissions/:permId`（Go）与 `DELETE /partitions/{id}/permissions/{perm_id}`（Rust）路径不一致 | 确认前端调用的是哪一侧；如前端走 Rust 路径则可视为既定差异，否则需统一 |
| P2-2 | 限流（429 RATE_LIMIT_EXCEEDED）/ 熔断错误码已在文档枚举，但中间件未实现 | 长期补全 |
| P2-3 | 安全中间件仅单测，缺真实攻击面测试 | 补集成测试 |
| P2-4 | 优先级调度器 K8s 真实 job 提交未联调 | 待 K8s 环境 |
| P2-5 | Rust 覆盖率 job `continue-on-error: true`，仅出报告不卡 PR | 待覆盖率提升后改为硬门禁 |
| P2-6 | Go 版若干只读路由 Rust 未实现（见 §4.4 Go 独有清单） | 按需补齐，非阻断 |

---

## 3. 本次修复记录

### 3.1 `.gitignore` 覆盖率产物排除（P1-1）
- `metaclouds-backend-rust/.gitignore` 末尾新增：
  ```
  # ===== Rust coverage artifacts (CI 产物，非交付物) =====
  priority_scheduler_coverage/
  *.profraw
  llvm-cov/
  ```
- 根目录 `.gitignore` 同步追加相同三项（原文件已有 `priority_scheduler_coverage.out`、`coverage`、`*.out`，但缺目录与 profraw）。
- 清理未跟踪 CI 产物：`metaclouds-backend/priority_scheduler_coverage`（9726 字节，Go coverprofile，`mode: set` 头部）已确认是 CI 产物并删除。

### 3.2 `verify-deployment` 冗余 env（P1-2）
- 文件：`.github/workflows/ci-cd.yml` 第 691-693 行 env 块。
- 删除 `STAGING_SSH_KEY: ${{ secrets.STAGING_SSH_KEY }}`。
- 保留 `LOG_LEVEL: INFO`（E2E 步骤 `PORT_UTILS_LOG_LEVEL: ${{ env.LOG_LEVEL }}` 仍引用）。
- 核对该 job 全部步骤：checkout / setup-go / go mod download / go test（自带 env）/ curl health，**确无任何 ssh 用法**，删除安全。

### 3.3 API 文档对齐（P1-3）
对比 `src/routes.rs`（116 个 `.route()`）与 `docs/api-reference-rust.md`：

**文档缺失、本次补入（14 个路由）：**
- `GET /clusters/{id}/status`（P4 后补，文档差异表仍写"未实现"）
- 分区 3 个：`GET /partitions/{id}/permissions`、`PUT /partitions/{id}/priority`、`PUT /partitions/{id}/max-runtime`
- FluidCache 9 个：`GET/POST /datasets/{id}/caches`、`GET/POST /datasets/{id}/fluid-caches`、`PUT/DELETE /fluid-caches/{cache_id}`、`POST .../enable`、`.../disable`、`.../prefetch`
- 根级 `GET /health`（新增第 19 节）

**路径错误、本次修正（1 个）：**
- 文档原写 `POST /quotas/{id}/check`，实际 `routes.rs:289` 为 `POST /quotas/check`（无路径参数，已与 Go 对齐）。

**同步更新：**
- 公开端点约定行补入 `/health`、`/api-docs/openapi.json`。
- 底部"与 Go 版差异汇总"表：行 1（health）、行 2（quotas/check）、行 5（clusters/:id/status）由"未实现/对齐中"改为"已对齐/已实现"；新增行 10（FluidCache 已对齐）。

> 差异合计：补 14 路由 + 改 1 路径 + 修 3 行差异表。未发现"文档有而 routes.rs 无"的过时路由。

---

## 4. 现状盘点表

### 4.1 服务端口
| 端口 | 用途 | 状态 |
|------|------|------|
| 3000 | 前端 dev | 未监听 |
| 8000 | 后端 API | 未监听 |
| 8001 | 辅助 | 未监听 |

### 4.2 Git 状态
- HEAD：`98ead4b`
- 工作区改动（本次）：
  - `.github/workflows/ci-cd.yml`（删冗余 env）
  - `.gitignore`（根）
  - `metaclouds-backend-rust/.gitignore`
  - `metaclouds-backend-rust/docs/api-reference-rust.md`
  - 新增 `metaclouds-backend-rust/docs/comprehensive-review-2026-09-20.md`
- 已删除未跟踪 CI 产物：`metaclouds-backend/priority_scheduler_coverage`

### 4.3 CI job 列表与依赖
| # | job | needs | 触发/条件 | 无 secrets 时行为 |
|---|-----|-------|-----------|------------------|
| 1 | security-scan | — | 始终 | 正常（checkov soft_fail + gitleaks） |
| 2 | backend-test | security-scan | 始终 | 正常（Go 单测/覆盖率） |
| 3 | frontend-test | security-scan | 始终 | 正常（type-check/build，test continue-on-error） |
| 4 | docker-build-backend | backend-test, frontend-test | push / workflow_dispatch | 构建+Trivy 扫描，`push:false` |
| 5 | docker-build-frontend | backend-test, frontend-test | push / workflow_dispatch | 构建+Trivy 扫描，`push:false` |
| 6 | deploy-development | docker-build-* | refs/heads/develop 或手动 development | SSH 步骤 `if: DEV_SSH_KEY!=''` 跳过，仅 curl 占位域名 |
| 7 | deploy-staging | docker-build-* | refs/heads/main 或手动 staging | SSH 步骤条件跳过 |
| 8 | production-approval | deploy-staging | 仅手动 workflow_dispatch + env=production | 审批门 |
| 9 | deploy-production | production-approval | 继承上游 | SSH 步骤条件跳过 |
| 10 | verify-deployment | deploy-dev/staging/prod | `if: always()` | 跑 E2E（失败 `|| echo`）+ curl health |
| 11 | rust-lint-test | security-scan | 始终 | fmt/clippy/test/build |
| 12 | rust-test-postgres | security-scan | 始终 | postgres:16 service 容器冒烟 |
| 13 | rust-coverage | rust-lint-test | 始终 | `continue-on-error: true`，仅出 lcov |
| 14 | rust-release-build | security-scan | 始终 | `cargo build --release` |

### 4.4 路由数对比（Go 129 vs Rust 116）
计数口径：Go 为 `gin` 的 GET/POST/PUT/DELETE 注册调用；Rust 为 `axum` 的 `.route()` 调用（每个 path 计一次，多方法链算一次）。

**Go 独有（Rust 未实现，共 ~13 条业务路由）：**
- 认证/调试：`POST /auth/register`（条件注册）、`GET /test/slow`、`GET /test/error`（均仅非生产）
- 资源：`GET /resources/gpu`（Rust 用 `/gpus` 替代）
- 作业：`POST /jobs/:id/submit`（Rust 创建即提交）、`GET /jobs/:id/status`
- 监控：`GET /monitoring/alerts`、`PUT /monitoring/alerts/:id/resolve`（Rust 改为独立 `/alerts` 域）
- 配额：`GET /quotas/usage`
- 调度器：`GET /schedulers/:id/queues`、`GET /schedulers/:id/nodes`、`GET /schedulers/:id/health`（Rust 用 `test-connection` 替代 health）
- 拓扑：`POST /topology/score`
- 检查点：`GET /checkpoints/latest/:jobId`
- 另有根级 SPA 静态/兜底路由（`/`、`/app`、`/backend`、`/static`、`/assets`、`NoRoute`）

**Rust 独有（Go 无，共 ~20 个路由）：**
- `/users` CRUD 5 个（Go routes.go 无用户组）
- `PUT /auth/change-password`
- `GET /jobs/stats`
- 独立 `/alerts` 域 8 个（list/stats/get/create/update/delete/acknowledge/resolve）
- `/monitoring/dashboard`、`GET /monitoring/alert-rules`、`POST /monitoring/alert-rules/evaluate`
- `POST /resources` + `DELETE /resources/{id}`（Go 仅 GET/PUT resources）

**别名重复计算：** Rust 的 `/gpu/*`（9 个）与 `/topology/nodes`（5 个）是 Vue3 兼容别名，与主路径 `/gpus`、`/topology` 共用 handler/RBAC，在 116 个 `.route()` 中各算一次，合计 14 个属于别名重复。

**路径分歧（功能等价）：**
- 撤销分区权限：Go `DELETE /partitions/permissions/:permId` ↔ Rust `DELETE /partitions/{id}/permissions/{perm_id}`
- FluidCache 操作：Go 挂 `/datasets/caches/:cacheId` ↔ Rust 顶层 `/fluid-caches/:cacheId`（外加 `/datasets/:id/caches` 别名）

> P4-01 之后新补的 `/health`、FluidCache、partition priority/max-runtime/permissions、`/clusters/{id}/status` 均已在 Rust 落地，本次已同步进文档；未发现 P4-01 之后的新增缺口。

### 4.5 测试与文档
- Rust 单测：**296 passed**（本次未改源码，无变化）。
- Rust 文档：`metaclouds-backend-rust/docs/` 共 **13 份 Markdown**（api-reference-rust / cron-migration-reference / cutover-plan / deployment-rollback-drill / dual-driver-migration / go-retirement-checklist / golden-regression-phase4 / performance-benchmark / postgres-dual-driver-check / runbook-v2-rust / shadow-dual-track / test-mapping-phase4 / vue-migration-guide）+ `openapi-rust.json`。

### 4.6 Docker 构建现状
- `docker-build-backend` / `docker-build-frontend` 均 `push: false`（ci-cd.yml 424、485 行）。
- 两 job 已 `docker/login-action` 登录 ghcr.io，但**从不 push**。
- 影响：镜像仅在 CI 内构建并 Trivy 扫描（HIGH/CRITICAL，SARIF 上传），无镜像发布通道；部署侧走 SSH `git pull` + `./deploy.sh`，不依赖镜像仓库。

---

## 5. Vue 前端合规性快查
- 目录：`metaclouds-frontend-vue/src/pages/`（非 `views/`），共 **15 个 `.vue` 页面**：
  Login / Dashboard / ClusterManagement / ResourceManagement / GPUManagement / JobManagement / K8SManagement / TopologyManagement / PartitionManagement / SchedulerManagement / DatasetManagement / MonitoringAlert / MultiTenantManagement / AccelerationSuiteManagement / SecurityManagement。
- 另有 3 个测试 spec（Login / Dashboard / JobManagement）。
- 科技蓝主题：`#1677ff` 出现在 `src/styles/index.css:15`（`--sidebar-primary-color: #1677ff`）、`styles/sidebar.css`、`components/Topbar.vue`，全局主题已配置。

---

## 6. 待目标环境项（本机无法完成）
1. **部署 secrets 配置**：在 GitHub repo Settings → Secrets 配置 `DEV_SSH_KEY/HOST/USER`、`STAGING_SSH_KEY/HOST/USER`、`PROD_SSH_KEY/HOST/USER`、`DEPLOY_DIR`，使部署 job 的 SSH 步骤真正生效。
2. **Docker 镜像发布**：将 `push` 改为 `true` 并授予 ghcr 推送权限（当前登录用 `GITHUB_TOKEN`，需确认 repo 允许写 packages）。
3. **Go 后端退役**：按 `docs/shadow-dual-track.md` 影子观察 ≥1 周 → 灰度切流 10% → 50% → 100% → 按 `docs/go-retirement-checklist.md` 下线。
4. **K8s 真实部署**：在真实集群部署 Rust 服务并验证 livenessProbe/readinessProbe 命中 `/health`、Prometheus 抓取 `/metrics`。
5. **生产 PostgreSQL 实跑**：用 `rust-test-postgres` 之外的真实 PG 实例验证迁移、JSONB/TIMESTAMPTZ、连接池。

---

## 7. 长期改进建议
- **优先级调度器**：对接真实 K8s 提交 Job（当前为 mock）。
- **限流/熔断中间件**：文档已声明 429/503 错误码，补实现并加测试。
- **安全中间件**：补真实攻击面集成测试（越权、CSRF 绕过、JWT 伪造）。
- **性能**：PostgreSQL 连接池参数（max_connections / timeout）按 `docs/performance-benchmark.md` 调优。
- **覆盖率门禁**：`rust-coverage` 从 `continue-on-error` 逐步收紧为硬阈值。
- **路径统一**：与前端确认分区权限撤销、FluidCache 操作路径是否需与 Go 完全一致。

---

## 8. 验证门禁
- `cargo fmt --check`：通过（本次未改 Rust 源码）。
- `cargo clippy --all-targets -- -D warnings`：通过。
- `cargo test`：296 passed，无变化。
- CI YAML：删除单行 env 后缩进保持合法（`env:` 块仍保留 `LOG_LEVEL`）。

---

## 9. 2026-09-21 续轮复盘

> 本轮范围：文档不一致修复 + 路径统一核对 + 前端联调检查 + CI 现状确认。
> 不修改 Rust 源码与前端代码，仅文档修正与静态分析记录。

### 9.1 文档不一致修复（/health 过时表述）

Rust `src/routes.rs:456-465` 已实现根级 `GET /health`（无 JWT，返回 200），
`k8s/05-deployment.yaml` 三探针（startup/liveness/readiness）path 均已改为 `/health`
（135/144/155 行）。修正以下 4 处过时表述：

| 文件 | 行 | 原表述 | 修正后 |
|------|----|--------|--------|
| `cutover-plan.md` | 26 | "**无根级 `/health`**；三探针统一用 `/metrics`" | "`GET /health`（根级，无 JWT）；三探针统一用 `/health`" |
| `runbook-v2-rust.md` | 408-422 | 探针 YAML 三探针 path=`/metrics` + 注释"无根级 /health" | YAML 三探针 path 改为 `/health`；注释改为"已实现 /health，/metrics 仅供 Prometheus" |
| `go-retirement-checklist.md` | 28 | "Rust 无根级 /health（探针用 /metrics）…🔧 待确认" | "/health 已实现，三探针已切换…✅ 已确认" |
| `vue-migration-guide.md` | 174 | "根级健康检查 **不存在**，探针用 /metrics" | "已对齐，探针用 /health" |

K8s Deployment 探针 path 确认：**全部为 `/health`**（startupProbe/livenessProbe/readinessProbe 均 `path: /health, port: http`）。

### 9.2 路径统一核对

#### 9.2.1 Partition 权限删除路径（差异确认）

| 维度 | 路径 |
|------|------|
| Go `routes.go:627` | `DELETE /api/v1/partitions/permissions/:permId`（无 `{id}`） |
| Rust `routes.rs:268` | `DELETE /api/v1/partitions/{id}/permissions/{perm_id}`（含 `{id}`） |
| 前端 `api/index.ts:146` | `DELETE /partitions/permissions/${id}`（**走 Go 路径**） |

**结论**：前端调用 Go 风格路径 `/partitions/permissions/{id}`，Rust 路由为 `/partitions/{id}/permissions/{perm_id}`。
→ **上线后 404 风险**。需二选一：① 前端改为传 partition_id 并调用 `/partitions/{pid}/permissions/{permId}`；
② Rust 补一条别名路由 `DELETE /partitions/permissions/{perm_id}`。

#### 9.2.2 FluidCache 路径（无差异风险）

| 维度 | 路径 |
|------|------|
| Go 原始 | `/datasets/caches/:cacheId`（PUT/DELETE/enable/disable/prefetch） |
| Go Vue 别名 | `/fluid-caches/:cacheId`（顶层，PUT/DELETE/enable/disable/prefetch） |
| Rust | `/fluid-caches/:cacheId`（顶层，与 Go Vue 别名一致） |
| 前端调用 | `/fluid-caches/{cacheId}`（update/delete/enable/disable/prefetch）✅ |

**结论**：前端走 Vue 别名路径 `/fluid-caches/:cacheId`，Rust 已实现该路径。**无 404 风险**。
Go 原始嵌套路径 `/datasets/caches/:cacheId` 前端未调用，不影响。

#### 9.2.3 前端调用但 Rust 未实现的端点（404 风险清单）

对比前端 `api/index.ts` 全部端点调用与 Rust `routes.rs` 注册路由，发现以下 **11 个端点**前端调用但 Rust 未实现或路径不同：

| # | 前端调用 | Rust 状态 | 风险 |
|---|----------|-----------|------|
| 1 | `GET /resources/gpu` | 未实现（Rust 用 `/gpus` 或 `/gpu/devices`） | 404 |
| 2 | `POST /jobs/{id}/submit` | 未实现（Rust 仅 `POST /jobs` 创建） | 404 |
| 3 | `GET /jobs/{id}/status` | 未实现 | 404 |
| 4 | `GET /monitoring/alerts` | Rust 用顶层 `/alerts`，非 `/monitoring/alerts` | 404 |
| 5 | `DELETE /partitions/permissions/{id}` | Rust 用 `/partitions/{id}/permissions/{perm_id}` | 404 |
| 6 | `GET /quotas/usage` | 未实现 | 404 |
| 7 | `GET /schedulers/{id}/queues` | 未实现 | 404 |
| 8 | `GET /schedulers/{id}/nodes` | 未实现 | 404 |
| 9 | `GET /schedulers/{id}/health` | 未实现 | 404 |
| 10 | `POST /topology/score` | 未实现 | 404 |
| 11 | `GET /checkpoints/latest/{jobId}` | 未实现 | 404 |

> 其中 #1/#2 在 `vue-migration-guide.md` 已知差异表中已记录；#3-#11 为本轮新发现，
> 建议后续轮次补 Rust 别名路由或调整前端调用。

### 9.3 前端联调检查

#### 9.3.1 baseURL 配置

| 项 | 值 |
|----|-----|
| `http.ts` baseURL | `/api/v1`（相对路径） |
| `vite.config.ts` proxy target | `http://localhost:8000`（**当前指向 Go 版，未切到 Rust 8001**） |
| 切换方式 | 改 `vite.config.ts:16` 的 target 为 `http://localhost:8001`，重启 dev server |
| 生产环境 | Nginx 反代 `/api/` upstream 从 Go 改到 Rust，不改前端代码 |

#### 9.3.2 15 页面端点对比

前端 `src/pages/` 共 15 个 `.vue` 页面，全部通过 `api/index.ts` 集中调用（无直接 axios/fetch）。
对比结果：核心流程（登录 `/auth/login`、Dashboard `/monitoring/dashboard`、集群 `/clusters`、
作业 `/jobs`、GPU `/gpu/devices`、告警 `/alerts`）路径已对齐或走别名。
**404 风险集中在 9.2.3 列出的 11 个边缘端点**。

#### 9.3.3 科技蓝主题

`#1677ff` 已确认存在于：`styles/index.css:15`、`styles/sidebar.css:24/37/120`、`components/Topbar.vue:143`。
全局主题配置无变化。

### 9.4 CI 现状确认

#### 9.4.1 GitHub Actions 最新 run

| Run ID | 状态 | 结论 | 分支 | 时间 (UTC) |
|--------|------|------|------|------------|
| **35495951942** | completed | **success** | main | 2026-09-20T07:05:45Z |
| 35495613118 | completed | success | main | 2026-09-20T06:58:20Z |
| 35493043520 | completed | success | main | 2026-09-20T05:59:19Z |
| 35489828635 | completed | success | main | 2026-09-20T04:42:47Z |
| 35489224861 | completed | failure | main | 2026-09-20T04:28:22Z |

**最新 run #35495951942 全绿**。前一轮 #35489224861 的 failure 已在后续 run 修复。

#### 9.4.2 部署 secrets 状态

- `deploy-development` SSH 步骤条件：`if: env.DEV_SSH_KEY != '' && env.DEV_HOST != ''`（529 行）。
- `deploy-staging` SSH 步骤条件：`if: env.STAGING_SSH_KEY != '' && env.STAGING_HOST != ''`（578 行）。
- `deploy-production` SSH 步骤条件：`if: env.PROD_SSH_KEY != '' && env.PROD_HOST != ''`（650 行）。
- **结论**：secrets 未配置时，SSH 部署步骤自动跳过（job 仍运行但无实际部署动作）。当前 secrets 状态无法从代码确认，需在 GitHub repo Settings → Secrets 检查。

#### 9.4.3 Docker build push 策略

- `docker-build-backend`：`push: false`（ci-cd.yml:424）✅ 确认
- `docker-build-frontend`：`push: false`（ci-cd.yml:485）✅ 确认
- **注意**：deploy job 的 `working-directory` 仍指向 `metaclouds-backend`（Go 版），
  部署脚本 `./deploy.sh` 也是 Go 版的。Rust 版（`metaclouds-backend-rust`）的 CI job
  （rust-lint-test / rust-test-postgres / rust-coverage / rust-release-build）独立运行，
  **未接入 docker-build / deploy 链路**。

### 9.5 本轮新增问题

1. **前端 Vite proxy 仍指向 Go（:8000）**：开发环境切到 Rust 需手动改 `vite.config.ts:16`。
2. **CI deploy job 仍构建/部署 Go 版**：Rust 版 CI 与部署链路未打通（docker-build 上下文为 `metaclouds-backend`）。
3. **11 个前端端点 Rust 侧 404 风险**（见 9.2.3），需后续轮次补别名或调整前端。
4. **文档中 D:\YCYD 旧路径残留**：`golden-regression-phase4.md`、`shadow-dual-track.md`、`runbook-v2-rust.md:259`、`vue-migration-guide.md:56`、`test-mapping-phase4.md` 仍引用 `D:\YCYD`，实际已迁移到 `E:\YCYD`。本轮未修改（非 /health 范围），建议后续统一替换。

---

## §10 2026-09-21 续轮复盘（续）

### 10.1 限流熔断中间件实现完成

本轮新增两个中间件模块：

| 文件 | 功能 | 关键设计 |
|------|------|----------|
| `src/middleware/rate_limit.rs` | 滑动窗口限流 | 默认 100 req/60s，`RATE_LIMIT_ENABLED` 环境变量控制开关（默认 false） |
| `src/middleware/circuit_breaker.rs` | 熔断器状态机 | CLOSED / OPEN / HALF_OPEN 三态，环境变量控制，默认关闭 |
| `src/error.rs` | 新增 `CIRCUIT_BREAKER_OPEN(503)` 错误变体 | 熔断打开时返回 503 |
| `src/middleware/mod.rs` | 导出新模块 | — |

**测试覆盖**：限流熔断单元测试 9 个 + 安全攻击面测试 19 个，合计 28 个新测试用例。

### 10.2 安全攻击面测试

`tests/security_attack_surface_test.rs` 新增 **19 个用例**，覆盖：

- 权限提升（privilege escalation）
- CSRF 绕过（CSRF bypass）
- JWT 伪造（JWT forgery）
- SQL 注入（SQL injection）
- 路径遍历（path traversal）
- 越权删除用户 / 创建租户等

全部通过（`19 passed; 0 failed; 0 ignored`）。

### 10.3 文档不一致修复

本轮修正 4 处过时的 `/health` 引用：

| 文件 | 修正内容 |
|------|----------|
| `docs/cutover-plan.md` | 更新 /health 描述 |
| `docs/go-retirement-checklist.md` | 更新 /health 描述 |
| `docs/runbook-v2-rust.md` | 更新 /health 描述 |
| `docs/vue-migration-guide.md` | 更新 /health 描述 |

### 10.4 路径差异核对结果（继承 §9.2）

- **Partition 权限删除 404 风险**：前端调 `DELETE /partitions/permissions/{id}`（Go 风格），Rust 路由为 `DELETE /partitions/{id}/permissions/{perm_id}`。上线后 404。
- **FluidCache 路径**：前端走 Vue 别名 `/fluid-caches/:cacheId`，Rust 已实现，无差异。

### 10.5 前端联调检查结果（继承 §9.3）

- **baseURL**：`/api/v1`（相对路径）。
- **Vite proxy target**：`http://localhost:8000`（**仍指向 Go 版，未切到 Rust :8001**）。
- **切换方式**：改 `vite.config.ts:16` target 为 `http://localhost:8001`，重启 dev server。
- **生产环境**：Nginx 反代 `/api/` upstream 从 Go 改到 Rust。

### 10.6 CI 最新 run 状态

- 最新 run **#35495951942**：completed / **success** / main / 2026-09-20T07:05:45Z。
- 全绿。前一轮 failure 已修复。

### 10.7 服务启动冒烟验证结果（2026-09-21）

#### 10.7.1 Rust 后端 :8001

| 端点 | 结果 | 说明 |
|------|------|------|
| `GET /health` | **200** | `{"success":true,"data":{"status":"ok","version":"0.1.0","uptime":...}}` |
| `POST /api/v1/auth/login` | **200** | admin/Admin@123456 返回 JWT token + user 对象 |
| `GET /api/v1/clusters`（Bearer token） | **200** | 空数组 `[]`（内存库无数据） |
| `GET /api/v1/monitoring/dashboard`（Bearer token） | **200** | 完整 dashboard 指标 JSON |
| `GET /metrics` | **200** | Prometheus 格式指标（http_request_duration_seconds 等） |
| 未认证 `GET /api/v1/clusters` | **401** | `UNAUTHORIZED`，Authorization header required |

> **注意**：登录实际路径为 `/api/v1/auth/login`（非 `/auth/login`）。
> `DATABASE_URL=sqlite::memory:` 在连接池回收连接时会丢失表数据（`no such table: users`），
> 冒烟测试改用文件型 SQLite（`sqlite:smoke-test.db`）通过。此为已知 sqlx 内存库连接池限制。

#### 10.7.2 Go 后端 :8000（对照）

| 端点 | 结果 | 说明 |
|------|------|------|
| `GET /health` | **200** | `{"status":"healthy","timestamp":...,"dependencies":1}` |

> **响应格式差异**：Go 返回 `{"status":"healthy",...}`，Rust 返回 `{"success":true,"data":{"status":"ok",...}}`（统一信封）。前端需适配或 Nginx 层兼容。

#### 10.7.3 Vue 前端 :3000

| 项 | 结果 |
|----|------|
| Vite dev server | **200 / ready**（v5.4.21，956ms 启动） |
| 监听端口 | 3000 |
| Vite proxy target | `http://localhost:8000`（Go 版） |

#### 10.7.4 整合验证门禁

| 检查 | 结果 |
|------|------|
| `cargo fmt --check` | **0 errors** |
| `cargo clippy --all-targets -- -D warnings` | **0 warnings** |
| `cargo test` | **324 passed; 0 failed; 2 ignored** |

### 10.8 新增问题清单

1. **前端 11 个未实现端点**（§9.2.3 已列）：Rust 侧未实现或路径不同，上线后 404。
2. **Vite proxy 指向 Go :8000**：开发环境切 Rust 需手动改 `vite.config.ts:16`。
3. **Partition 权限删除 404 风险**：前端路径与 Rust 路由不一致。
4. **/health 响应格式不统一**：Go `{"status":"healthy"}` vs Rust `{"success":true,"data":{"status":"ok"}}`。
5. **sqlite::memory: 连接池限制**：内存库在连接回收后丢表，生产须用文件库或 Postgres。
6. **CI deploy job 仍构建/部署 Go 版**：Rust 版 CI 与部署链路未打通（继承 §9.5-2）。
7. **文档 D:\YCYD 旧路径残留**（继承 §9.5-4）。

### 10.9 待目标环境项

1. 在 staging/prod 环境配置 PostgreSQL（非 sqlite::memory:），验证 Rust 连接 Postgres 全流程。
2. 配置 GitHub Secrets（DEV/STAGING/PROD SSH key + host），打通 Rust 版 CI → 部署链路。
3. Nginx 反代 `/api/` upstream 从 Go 切到 Rust :8001，灰度验证。
4. 前端 Vite proxy 切换 + 11 个未实现端点的 Rust 别名路由补齐。
5. 生产环境 /health 响应格式统一（或前端适配双格式）。
6. 监控告警接入（Prometheus /metrics 端点已就绪，需接 Grafana / Alertmanager）。

---

## 11. 2026-09-21 P1 修复（前端端点对齐 + Vite proxy + Partition 权限路径统一）

> 触发：§10.8 新增问题清单第 1/2/3 项。本次以"最小改动 + 不引入新 crate + 现有 324 测试无回归"为原则逐项修复。

### 11.1 关键发现：原差异清单已大幅收敛

任务书附带的"差异清单"基于旧版 `routes.rs` 对比。本次复核 HEAD `ce3d1aa` 实际路由后发现，下列端点**此前一轮已补全**，无需再改：

- `PUT /resources/{id}`、`PUT /gpu/devices/{id}`、`DELETE /gpu/devices/{id}`、`PUT /partitions/{id}`、`DELETE /partitions/{id}`、`PUT /schedulers/{id}`、`DELETE /schedulers/{id}`、`PUT /acceleration/{id}`、`PUT /security/policies/{id}` —— 均已存在。
- `POST /gpu/allocations/{id}/release` —— Vue 别名路由已存在（指向 `release_gpu`）。
- `PUT /fluid-caches/{cacheId}`、`DELETE /fluid-caches/{id}`、`POST /fluid-caches/{id}/disable`、`POST /fluid-caches/{id}/prefetch` —— 均已存在。

因此本次真正需要修复的，是下列**前端页面实际调用、但 Rust 仍缺失**的端点。

### 11.2 前端页面实际使用情况核查

对 `src/pages/`（实际目录为 `pages/`，非 `views/`）15 个页面做调用点 grep，区分"页面实际使用"与"仅 API 层定义"：

| 端点 | 调用页面 | 本次处理 |
|------|----------|----------|
| `GET /monitoring/alerts` | Dashboard.vue、MonitoringAlert.vue | ✅ Rust 补别名 |
| `POST /jobs/{id}/submit` | JobManagement.vue | ✅ Rust 补 mock handler |
| `GET /resources/gpu` | K8SManagement.vue | ✅ Rust 补聚合 handler |
| `GET /schedulers/{id}/queues` | SchedulerManagement.vue | ✅ Rust 补 mock |
| `GET /schedulers/{id}/nodes` | SchedulerManagement.vue | ✅ Rust 补 mock |
| `GET /schedulers/{id}/health` | SchedulerManagement.vue | ✅ Rust 补 mock |
| `POST /topology/score` | TopologyManagement.vue | ✅ Rust 补 mock |
| `DELETE /partitions/permissions/{id}` | PartitionManagement.vue | ✅ Rust 补兼容别名 |

**仅 API 层定义、页面未使用（记录差异，不修复）**：

- `POST /auth/register`、`GET /jobs/{id}/status`（getK8SStatus）、`GET /quotas/usage`、`GET /checkpoints/latest/{jobId}`。
- 这些端点当前无页面调用，不产生 404 风险；待页面接入时再按需补实现。

### 11.3 Rust 补实现明细

| 端点 | 策略 | 实现 |
|------|------|------|
| `GET /monitoring/alerts` | 路由别名 | `monitoring.rs::list_monitoring_alerts`，复用 `alert::list_alerts`（大 page_size 取全量，对齐前端扁平数组期望） |
| `POST /jobs/{id}/submit` | mock handler | `job.rs::submit_job_to_k8s`，校验作业存在后返回 `{message, job_id, cluster:"mock-k8s"}`；真实 K8s 下发待执行器接入 |
| `GET /resources/gpu` | 聚合 handler | `resource.rs::list_gpu_resources`，按 model 聚合 gpu 设备表，输出前端 `GPUResource` 形状（gpuName/type/total/used/available/utilization） |
| `GET /schedulers/{id}/queues` | mock | `scheduler.rs::list_scheduler_queues` → 空数组 |
| `GET /schedulers/{id}/nodes` | mock | `scheduler.rs::list_scheduler_nodes` → 空数组 |
| `GET /schedulers/{id}/health` | mock | `scheduler.rs::scheduler_health` → `{healthy:true, status:"ok"}` |
| `POST /topology/score` | mock | `topology.rs::calculate_topology_score`，按候选节点索引生成确定性分数 |
| `DELETE /partitions/permissions/{id}` | 兼容别名 | `partition.rs::revoke_permission_by_id`（单参数），复用 `permission_service::revoke_permission`；对齐前端 Go 风格扁平路径 |

路由注册要点：`/resources/gpu`、`/topology/score` 等静态段均依赖 matchit"静态优先于动态段"匹配，与既有 `/resources/{id}`、`/topology/{id}` 不冲突。

### 11.4 Vite proxy 切换（§10.8-2）

- `vite.config.ts`：改为 `defineConfig(({ mode }) => …)`，用 `loadEnv` 读取 `VITE_API_PROXY_TARGET`，默认 `http://localhost:8001`（Rust 后端）。
- 新增 `.env.development`：`VITE_API_PROXY_TARGET=http://localhost:8001`（注释说明回连 Go 改 :8000）。
- grep 确认 `src/**/*.ts`、`src/**/*.vue`、`vite.config.ts` 无残留 `localhost:8000`。

### 11.5 Partition 权限删除路径统一（§10.8-3）

- 前端 `partitionApi.removePermission(id)` 走 Go 风格 `DELETE /partitions/permissions/{id}`。
- Rust 规范路径为 `DELETE /partitions/{id}/permissions/{perm_id}`。
- 本次选**方案 a（Rust 补兼容路由）**：新增 `DELETE /partitions/permissions/{id}` → `revoke_permission_by_id`，前端 API 层与页面零改动。

### 11.6 门禁验证结果

| 检查 | 结果 |
|------|------|
| `cargo fmt --check` | **0 errors** |
| `cargo clippy --all-targets -- -D warnings` | **0 warnings** |
| `cargo test` | **330 passed; 0 failed; 2 ignored**（原 324 + 新增 6） |
| `npx tsc --noEmit` | **0 errors** |
| `npm run build` | **built in 12.11s**（成功） |
| grep `localhost:8000` | **无残留** |

### 11.7 新增测试

新增 `tests/p1_endpoint_align_test.rs`，6 个用例：

1. `p1_monitoring_alerts_alias_returns_200`
2. `p1_resources_gpu_returns_200`
3. `p1_scheduler_queues_nodes_health_mock`
4. `p1_topology_score_mock`
5. `p1_job_submit_route_registered`（建作业→提交，断言 200）
6. `p1_partition_permission_go_style_delete`（建分区→授权→Go 风格删除，断言 204）

### 11.8 遗留（未修复，如实记录）

- 4 个"仅 API 层定义、页面未使用"端点（见 §11.2）暂不补实现，待页面接入。
- mock 端点（submit/queues/nodes/health/score）为占位实现，真实业务逻辑待 K8s/调度器执行器接入后替换。
- §10.9 待目标环境项（PostgreSQL、CI 部署链路、Nginx upstream 切流）仍待目标环境，不在本机修复范围。

---

## 12. 2026-09-22 P1 修复（第二轮）

HEAD 基线 `bb225a0`（386-server full-dimension monitoring data seed）CI 全绿后，本轮针对 6 项 P1 代码层问题做最小改动修复。

### 12.1 修复落点

| # | 问题 | 落点 | 说明 |
|---|------|------|------|
| 1 | Dashboard「GPU 利用率」KPI 恒为 0 | `metaclouds-frontend-vue/src/pages/Dashboard.vue`、`src/api/index.ts` | 根因：原逻辑从空的 `resources` 表聚合 `used/total`。改为新增 `monitoringApi.dashboard()`（`GET /monitoring/dashboard`），GPU 利用率取 `allocated_gpus / total_gpus`（后端已聚合：gpu_allocations COUNT ÷ gpu_devices COUNT）。KPI footer 与饼图共用同一 computed，自动生效。 |
| 2 | 侧边栏 GPU 徽标固定 10 | `src/components/Sidebar.vue` | `gpuApi.devices({})` 未传 page_size（后端默认 10），改为 `{ page_size: 1000 }`，与 Dashboard 一致，显示真实总数（390）。 |
| 3 | 4 个端点接线 | 前端 `JobManagement.vue`/`MultiTenantManagement.vue`、API `index.ts`；后端 `handlers/job.rs`/`quota.rs`/`checkpoint.rs` + `routes.rs` | ① `/jobs/{id}/status`：后端补 `get_job_status` mock（作业状态派生 phase），前端详情对话框展示 K8s 状态。② `/quotas/usage`：后端补 `get_quota_usage` mock，前端配额页顶部加用量摘要。③ `/checkpoints/latest/{jobId}`：后端补 `get_latest_checkpoint`（复用 list 服务按 job_id 取首条），前端详情高亮最新检查点。④ `/auth/register`：API 层方法已存在，无注册页且生产 `ALLOW_PUBLIC_REGISTRATION=false`，按设计不接线，仅记录。 |
| 4 | `/health` 格式统一 | 文档记录（不改代码） | Rust 已返回信封 `{success,data:{status,version,uptime}}`；Go 版返回 `{"status":"healthy","timestamp":N,"dependencies":N}`（无信封）。Go 版不修改。CI/K8s 探针只校验 200 状态码，前端不调用 `/health`，兼容无影响。 |
| 5 | CI deploy 接入 Rust | `.github/workflows/ci-cd.yml` | `rust-release-build` 增加二进制 artifact 上传；新增 `deploy-rust-staging` job（依赖 `rust-release-build`，SSH 到目标机 `cargo build --release && systemctl restart metaclouds-backend-rust`，与 Go 版 deploy 并行、以 SSH secrets 门控）。Go 版部署保持不变（双轨并行）。 |
| 6 | 限流/熔断中间件开关 | `src/config.rs`、`docs/runbook-v2-rust.md` | 中间件实际直读环境变量、默认关闭（与设计一致）；但 `config.rs` 旧默认值误写为 `true`（不被中间件消费，仅误导）。已对齐为 `false`，并更新 `config_test.rs::defaults_match_go` 断言。runbook §3.10/§3.11 更新默认值并补生产推荐配置（限流 300/60s、熔断 threshold 5/30s）。 |

### 12.2 验证门禁

| 检查 | 结果 |
|------|------|
| `cargo fmt --all --check` | **0 errors** |
| `cargo test` | **330 passed; 0 failed; 2 ignored** |
| `npx vue-tsc --noEmit` | **0 errors** |
| `npm run build` | **built in 13.64s**（成功） |

### 12.3 说明与遗留

- `/jobs/{id}/status`、`/quotas/usage` 为 mock 实现（占位字段），真实 Pod 状态与用量统计待 K8s 客户端/用量表落地后替换；`/checkpoints/latest/{jobId}` 为真实 DB 查询。
- `deploy-rust-staging` 在未配置 `STAGING_SSH_KEY`/`STAGING_HOST` secrets 时为空操作，不影响现有 Go 部署链路；待目标环境实演 systemd unit 名。
- `config.rs` 限流/熔断默认值由 `true` 改为 `false`，与中间件直读行为对齐；生产经 `RATE_LIMIT_ENABLED=true`/`CIRCUIT_BREAKER_ENABLED=true` 显式开启。
