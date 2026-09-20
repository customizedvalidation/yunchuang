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
