# Metaclouds 系统复盘与修复报告（2026-10-01）

> 触发：对齐 GitHub 远端 + 全系统复盘与修复。
> 方法：先确认云端对齐（fetch/分叉检查），再以三路并行**只读审计**（Rust 后端 / Vue 前端 / CI·K8s·compose 部署链路）产出带 `file:line` 证据的缺陷清单，随后对可本机验证的缺陷实施修复并过门禁。
> 基线：上一轮 `docs/gap-review-2026-09-30.md`（遗留 P0×1、P1×4、P2×4）。

---

## 0. 摘要

- **云端对齐**：`git fetch --all --prune` 后 `git rev-list --left-right --count origin/main...HEAD` = **0/0**，工作区干净无脏文件；分支 `workbuddy/main-08c88e90` 与 `origin/main` 同处 `52d1519`。
- **最关键发现**：系统**当前起不来**。`docker-compose.yml` 下发 `USE_SQLITE=false` → `wants_postgres=true` → 迁移成功后 `main.rs:48-53` 的 `as_sqlite()` 返回 `None` → **显式报错退出**。即 compose 与 K8s 两条部署路径都会启动即失败（不是静默降级）。
- **本轮修复 29 项**（部署配置 13 / 后端 6 / 前端 10），全部过本机门禁：后端 `cargo fmt --check`、`clippy --all-targets -D warnings`、`cargo test` 全绿；前端 `vue-tsc --noEmit && vite build` 通过；22 份 YAML 全量解析通过。
- **仍遗留**：P0 请求层 PG 方言移植（独立里程碑）、P1 生产加固批次（限流/CORS/Redis/cron/Swagger 鉴权）、多租户隔离仅 job 域落地等，**均未冒充完成**，见 §4。

---

## 1. 云端对齐状态

| 项目 | 状态 | 证据 |
|---|---|---|
| 本地 HEAD | `52d1519` | `git log --oneline -1` |
| 远端 origin/main | `52d1519` | `git fetch --all --prune` 后比对 |
| 分叉数 | 0 / 0（无分叉） | `git rev-list --left-right --count origin/main...HEAD` |
| 工作区 | 干净（修复前） | `git status --short` 无输出 |
| 遗留分支 | `feat/compose-rust-backend-ci-image` 已合并进主线 | `git branch -vv` |

---

## 2. 审计发现（三路并行，带证据）

### 2.1 P0 — 阻断上线

| 编号 | 问题 | 证据 | 影响 | 本轮处理 |
|---|---|---|---|---|
| A-1 | **部署路径启动即退出** | `docker-compose.yml:157` `USE_SQLITE=false` + `src/main.rs:48-53` fail-closed | compose/K8s 后端容器起不来 | ✅ 已修：compose 切 SQLite 文件库并注释说明前置条件 |
| A-2 | 请求层 SQL 仍是 SQLite 方言（`?N`、`last_insert_rowid`） | `src/services/cluster.rs:129`、`src/services/gpu.rs:251`；统计 152 处 `?N` vs 11 处 `$N` | PG 路径不可用，独立里程碑 | ⏸ 遗留（见 §4） |
| A-3 | 多租户隔离仅 job 域实现 | 正确范例 `src/services/job.rs:108-111`；反面 `src/handlers/quota.rs:236`、`src/handlers/resource.rs:92`、`src/handlers/gpu.rs:129` | 可跨租户读取配额/资源/GPU/数据集/告警 | ⏸ 遗留（改动面大） |
| A-4 | CI 镜像 `push:false` 但 deploy 拉取 `:GIT_SHA` | `ci-cd.yml:195/256` vs `:788-791`；且 `permissions` 缺 `packages: write` | deploy 必 ImagePullBackOff | ⏸ 遗留（待 GHCR 权限，上轮已标注） |
| A-5 | K8s 未部署 Postgres/Redis，且 DB 主机名自相矛盾 | `01-configmap.yaml:34` `postgresql.metaclouds.svc...` vs `02-secret.yaml:42` `postgres:5432`（代码优先读 DATABASE_URL） | 后端 CrashLoop | 🔶 部分修：主机名已统一为集群内地址；PG/Redis 清单仍需环境决策 |
| A-6 | `init.sql` 与 sqlx 迁移双轨建表且字段冲突 | 根 `init.sql:5-13`（`id SERIAL`、`password`）vs `migrations/postgres/001_initial.sql:11-20`（`BIGSERIAL`、`password_hash`） | compose 首库由 init.sql 建表，后端写入报 `column password_hash does not exist` | ⏸ 遗留（需二选一，见 §4） |

### 2.2 P1 — 上线前应修

| 编号 | 问题 | 证据 | 本轮处理 |
|---|---|---|---|
| B-1 | CI 把 kubeconfig 明文打进日志 | `ci-cd.yml:771` `kubectl config view --minify -o yaml`（含 CA 与 token） | ✅ 已修 |
| B-2 | `/metrics`、Swagger 无鉴权且默认开启 | `src/routes.rs:478/486`；`src/config.rs:174` | ⏸ 遗留（需先于真实部署完成） |
| B-3 | 限流无条件信任 XFF，`trusted_proxies` 零消费 | `src/middleware/rate_limit.rs:136-145`；`src/config.rs:332` | ⏸ 遗留 |
| B-4 | CORS / body-limit 完全未实现 | 全仓无 `Access-Control-*`；`Cargo.toml` 无 tower-http（注：`tower-http` 经依赖树已存在，但未装配 `CorsLayer`） | ⏸ 遗留 |
| B-5 | Redis 缓存/会话、cron 调度未接线 | `src/cache/redis.rs:251` `build_cache` 仅测试引用；`src/scheduler/mod.rs:126` | ⏸ 遗留（configmap 却已开启） |
| B-6 | 登录路径 `Mutex::lock().unwrap()` | `src/services/auth.rs:48/63/74` | ✅ 已修（poison 恢复） |
| B-7 | `/metrics` 编码 `expect` panic | `src/metrics/mod.rs:47-48` | ✅ 已修（改 Result） |
| B-8 | 告警指标名在后端**一个都不存在** | `alerts.yml:63/132/159/174`、`12-prometheusrule.yaml`（8 条）；真实指标见 `src/metrics/business.rs`、`http.rs` | ✅ 已修（改为真实指标名） |
| B-9 | Prometheus 抓不到后端 | `prometheus.yml` target `host.docker.internal`（无 extra_hosts）+ 抓 `/monitoring/metrics`（需 JWT 必 401）；K8s `09-networkpolicy` 未放行 monitoring ns | ✅ 已修 |
| B-10 | compose 缺 alertmanager，全端口裸奔 | `prometheus.yml:24` 指向 `alertmanager:9093`；compose 5432/6379/2379/jaeger 全部 `0.0.0.0` | ✅ 已修（补服务 + 回环绑定） |
| B-11 | 前端 Pod 只读根文件系统下 nginx 起不来 | `14-frontend-deployment.yaml:99` + 未挂 `/var/log/nginx` | ✅ 已修 |
| B-12 | 告警「解决/忽略」是**假功能**（纯前端覆盖） | `MonitoringAlert.vue:243-246`；后端已有 `POST /alerts/{id}/resolve`、`/acknowledge`（`src/routes.rs:404-405`） | ✅ 已修（接线真实 API） |
| B-13 | 401 无刷新重试、错误未映射中文、无 5xx 重试 | `src/api/http.ts:66-84`、`src/utils/useFetch.ts:18` | 🔶 部分修（错误映射已做；refresh 重试仍遗留） |
| B-14 | 前端权限与后端矩阵不一致 / fail-open | `MultiTenantManagement.vue` 6 处 `['admin','manager']`（后端 manager 无 `TENANT_WRITE`，`authz/mod.rs:274`）；`router/index.ts:54` 与 `utils/auth.ts:31` fail-open | ✅ 已修 |
| B-15 | 登出三处重复且未清 store | `Sidebar.vue:259`、`Topbar.vue:93`、`http.ts:74` | ✅ 已修 |

### 2.3 P2 — 打磨

| 编号 | 问题 | 证据 | 本轮处理 |
|---|---|---|---|
| C-1 | Dockerfile HEALTHCHECK 打 `/metrics`（`PROMETHEUS_ENABLED=false` 时恒 404） | `metaclouds-backend-rust/Dockerfile:71` | ✅ 已修（改 `/health`） |
| C-2 | 健康检查不区分 liveness/readiness，DB 抖动会重启全部 Pod | `src/handlers/health.rs` 单端点带 DB 探测；`05-deployment.yaml` 三探针全指 `/health` | ✅ 已修（拆分 `/health/live`、`/health/ready` 并联动探针） |
| C-3 | 口令哈希错误回显内部细节 | `src/error.rs:194` `format!("password hashing error: {e}")` | ✅ 已修（细节只进日志） |
| C-4 | JWT 默认 60s leeway | `src/auth/jwt.rs:107` `Validation::default()` | ✅ 已修（leeway=0） |
| C-5 | nginx 无 `/api` 反代、缺安全响应头 | `metaclouds-frontend-vue/nginx.conf` | ✅ 已修 |
| C-6 | vite dev 代理默认 8001 与 compose 8000 不一致 | `vite.config.ts:9` | ✅ 已修 |
| C-7 | 响应拦截器无条件解包，吞掉 `success:false` | `src/api/http.ts:60-63` | ✅ 已修 |
| C-8 | kustomization 使用已弃用的 `metadata.name` | `13-kustomization.yaml:11-12` | ✅ 已修 |
| C-9 | gitleaks 白名单未覆盖新增 Secret 占位 | `gitleaks.toml:34-35` 仅 2 个，实际 4 个 | ✅ 已修 |

---

## 3. 本轮修复清单

### 3.1 部署与配置（13 项）

| 文件 | 改动 |
|---|---|
| `docker-compose.yml` | ①后端切 SQLite 文件库（`USE_SQLITE=true` + `DATABASE_URL` + `backend_data` 卷），使联调栈真正可启动；②全部中间件/后端端口改 `127.0.0.1` 绑定；③新增 `alertmanager` 服务与配置挂载；④补 `backend_data`/`alertmanager_data` 卷 |
| `alertmanager.yml`（新增） | 最小可用告警路由（group_by / inhibit_rules + 生产接收器替换说明） |
| `prometheus.yml` | target 改 `backend:8000`（原 `host.docker.internal` 缺映射）；删除必 401 的 `metaclouds-jobs` job |
| `alerts.yml` | 5 条失效表达式改用真实指标：`up{job=...}`、`metaclouds_running_jobs`、`metaclouds_active_alerts`、`metaclouds_allocated_gpus/total_gpus`、`http_requests_total{status="401"}` |
| `k8s/12-prometheusrule.yaml` | 6 处指标名校正 + 移除依赖未暴露指标（`metaclouds_db_connections_*`）的规则并留说明 |
| `k8s/09-networkpolicy.yaml` | 放行 `monitoring` ns 入站 8000（否则 ServiceMonitor 抓不到、告警全哑） |
| `k8s/02-secret.yaml` | `database-url` 主机名统一为 `postgresql.metaclouds.svc.cluster.local`（与 configmap 一致），含 create-secret 命令与 base64 占位重算 |
| `k8s/05-deployment.yaml` | 三探针联动新端点：startup/liveness → `/health/live`，readiness → `/health/ready` |
| `k8s/13-kustomization.yaml` | 移除 kustomize v5 已弃用的 `metadata.name` |
| `k8s/14-frontend-deployment.yaml` | 补 `/var/log/nginx` emptyDir（只读根文件系统下 nginx 无法写日志会 CrashLoop） |
| `metaclouds-frontend-vue/nginx.conf` | 新增 `/api/` 反代（compose 场景）+ 全套安全响应头（含 CSP，并在各 location 重复以规避 nginx 子块覆盖父块行为） |
| `metaclouds-backend-rust/Dockerfile` | HEALTHCHECK `/metrics` → `/health` |
| `.github/workflows/ci-cd.yml` | 删除 `kubectl config view --minify -o yaml`（会泄露集群 CA/token），改为打印 current-context |
| `gitleaks.toml` | 补齐 4 个 K8s Secret base64 占位白名单 |

### 3.2 后端 Rust（6 项）

| 文件 | 改动 |
|---|---|
| `Cargo.toml` | 新增 `indexmap = { version = "1.9", features = ["std"] }`：修复 `opentelemetry-otlp(grpc-tonic) → tonic 0.12 → tower 0.4.13 → indexmap 1.9.3` 因未启用 std 导致 `IndexMap` 第三泛型无默认值、tower::ready_cache 编译失败（E0107）。**此依赖为修复依赖树所加，勿删** |
| `src/services/auth.rs` | 3 处 `Mutex::lock().unwrap()` → `unwrap_or_else(\|e\| e.into_inner())`，避免一次 panic 污染锁导致后续所有登录 500 |
| `src/metrics/mod.rs` | `encode_metrics()` 去 `expect`，改返回 `AppResult<String>`；handler 同步改 `AppResult<impl IntoResponse>` |
| `src/error.rs` | 口令哈希错误不再把 argon2 内部细节写进响应体，细节仅 `tracing::error!` 记录 |
| `src/auth/jwt.rs` | 显式 `validation.leeway = 0` |
| `src/handlers/health.rs` + `src/routes.rs` | 新增 `GET /health/live`（不查库）与 `GET /health/ready`（查库），`/health` 保留旧语义兼容 Dockerfile/CI |

### 3.3 前端 Vue（10 项）

| 文件 | 改动 |
|---|---|
| `src/utils/error.ts`（新增） | `toUserMessage()`：错误码→中文文案映射，网络异常统一提示，5xx 不回显后端原文 |
| `src/utils/useFetch.ts` | 加载/变更错误改用 `toUserMessage`，不再暴露 axios 英文原文 |
| `src/api/index.ts` | 新增 `resolveAlert` / `acknowledgeAlert`（后端 `routes.rs:404-405`） |
| `src/pages/MonitoringAlert.vue` | 「解决/忽略」接线真实后端接口并刷新列表；「重新打开」因后端无接口显式提示未变更 |
| `src/pages/JobManagement.vue` | 「取消」按钮纳入 `<Can>`（破坏性写操作，此前裸奔收到 403） |
| `src/pages/MultiTenantManagement.vue` | 6 处写操作权限收敛为 `['admin']`（与后端 `TENANT_WRITE` 矩阵一致） |
| `src/router/index.ts` | 角色守卫 fail-closed（role 缺失不再放行） |
| `src/utils/auth.ts` | `isRoleAllowed` 改 fail-closed |
| `src/components/Sidebar.vue`、`Topbar.vue` | 登出统一走 `authStore.logout()`（清 httpOnly Cookie + 清空内存 user），消除三处重复硬编码 |
| `src/api/http.ts` | HTTP 200 + `success:false` 显式 reject，不再被静默解包 |
| `vite.config.ts` | dev 代理默认端口 8001 → 8000（对齐 compose） |

---

## 4. 门禁结果

| 门禁 | 命令 | 结果 |
|---|---|---|
| 后端格式 | `cargo fmt --check` | exit 0 |
| 后端静态检查 | `cargo clippy --all-targets -- -D warnings` | 通过，0 warning |
| 后端编译 | `cargo check --all-targets` | Finished dev profile |
| 后端测试 | `cargo test` | **336 passed / 0 failed**（`CARGO_INCREMENTAL=0 -j 4`） |
| 前端类型检查 + 构建 | `npm run build`（`vue-tsc --noEmit && vite build`） | 通过，✓ built in 31.22s |
| YAML 全量解析 | 22 份（k8s 17 + compose/prometheus/alerts/alertmanager/ci-cd） | 22 ok / 0 failed |

> 本机注意点：Rust 工具链为 **1.96-x86_64-pc-windows-msvc**（非此前记录的 gnu）；`cargo` 不在默认 PATH，需
> `export PATH="$HOME/.rustup/toolchains/1.96-x86_64-pc-windows-msvc/bin:$PATH"`；
> 本项目需 `CARGO_INCREMENTAL=0`，否则 Windows 上移动 `dep-graph.part.bin` 会触发 os error 5。

---

## 5. 仍遗留与下一步建议

### 5.1 必须解决才能真正上生产（按优先级）

> **2026-10-02 状态更新**：本轮已落地 2/3/4/5/6/7，仅第 1 项（请求层 PG 方言移植）
> 仍是硬阻塞。下方按项标注当前状态。

| # | 事项 | 状态 | 落地位置 |
|---|---|---|---|
| 1 | 请求层 PG 方言移植 | ❌ **仍未完成（唯一硬阻塞）** | 见 §5.1.1 |
| 2 | `init.sql` 与 sqlx 迁移二选一 | ✅ 已完成 | `init.sql` 已 `git rm`；compose 去掉挂载；runbook §3.1 只保留建库建账号 |
| 3 | 多租户隔离补全 | ✅ 已完成（quota/dataset/acceleration/alert/checkpoint/gpu 六个域） | `src/authz/mod.rs` 新增 `tenant_filter` / `tenant_for_write` / `user_for_write`；新增 `tests/tenant_isolation_test.rs` |
| 4 | 生产加固批次 | ✅ 已完成 | 见 §5.1.2 |
| 5 | GHCR 推送链路 | ✅ 已完成 | `.github/workflows/ci-cd.yml`：`permissions.packages: write` + 两处 `push: true` |
| 6 | K8s 内 Postgres/Redis 清单 | ✅ 已完成 | `k8s/17-postgresql.yaml`、`k8s/18-redis.yaml`、`k8s/19-backend-data-pvc.yaml`，已入 `13-kustomization.yaml` |
| 7 | Secret 占位值 | ✅ 已完成（门禁 + 生成命令） | CI 新增 "Guard - reject placeholder secrets"；Secret 新增 `postgres-password`、`protected-endpoints-token` |

#### 5.1.1 仍未完成：请求层 PG 方言移植（阻塞 K8s 切回 Postgres）

工作量实测（不是估算，是 `grep` 出来的数）：

- `?N` 占位符：**253 处**，分布在 `src/services/*`（quota 24、acceleration 19、partition 17、
  cluster 15、scheduler 14…）与 `src/models/*`；
- `SqlitePool` 类型标注：**276 处**（`AppState.pool` 也是 `SqlitePool`，不是 `DatabasePool`）；
- `last_insert_rowid()`：**23 处**（PG 需改为 `INSERT ... RETURNING id`）。

`main.rs` 对此是 fail-closed 的（连上 PG、跑完迁移后立刻报错退出），因此**不完成这一步
就不能把 compose/K8s 切回 `USE_SQLITE=false`**。本轮据此把 K8s 固定为
SQLite + PVC + 单副本（`01-configmap.yaml` `USE_SQLITE=true`，`05-deployment.yaml`
`replicas: 1` + `Recreate`，`07-hpa.yaml` 1/1，`08-pdb.yaml` `maxUnavailable: 1`），
PostgreSQL StatefulSet 已就绪待切，切换步骤见 `production-deployment-runbook.md` §1.2.1。

#### 5.1.2 生产加固批次（已完成的具体内容）

| 加固项 | 实现 |
|---|---|
| `/metrics` + Swagger 生产鉴权 | 新增 `src/middleware/protected.rs`：`SERVER_ENV=production` 时对 `/metrics`、`/swagger-ui`、`/api-docs` 生效；放行条件为「直连对端在 `METRICS_TRUSTED_CIDRS`（缺省回退 `TRUSTED_PROXIES`）」或「Bearer / `X-Api-Token` 常数时间等于 `PROTECTED_ENDPOINTS_TOKEN`」；两者都没有 → 503 失败关闭 |
| CORS | `routes.rs::cors_layer()` 按 `ALLOWED_ORIGINS` 组装 `CorsLayer`；生产未配置即不回任何 CORS 头（不用 `permissive()`） |
| 请求体上限 | `RequestBodyLimitLayer::new(body_limit(config.max_request_body_size))`，≤0 回落 10 MiB 默认 |
| XFF 仅可信代理采信 | 新增 `src/middleware/client_ip.rs`：`ConnectInfo` 对端在 `TRUSTED_PROXIES`（支持 CIDR）内才采信 XFF；限流与访问日志统一走它（此前审计日志无条件取 XFF 首段，可被伪造污染） |
| 直连地址注入 | `main.rs` 改用 `into_make_service_with_connect_info::<SocketAddr>()`，否则上面拿不到 peer IP |
| Redis 会话撤销 | `cache::install_cache(build_cache(&config))`；登出写 `revoked:<jti>`（TTL = 剩余寿命），`jwt_auth` 校验；Redis 不可用自动降级 |
| cron 接线 | `main.rs` 启动 `scheduler::Scheduler`（注册默认任务 + `start()`），优雅关闭时 `shutdown()`；此前只在单测里被构造过 |

### 5.2 打磨项

- 401 单飞 refresh + 5xx 指数退避重试（4xx 不重试）
- dashboard 串行 COUNT 聚合化、`SELECT *` 下推聚合
- OpenAPI 由 utoipa 生成并加 CI 契约测试（当前 `docs/openapi-rust.json` 滞后约 20 条）
- 前端 `/auth/register` 死端点清理、`tsconfig` 开启 `noUnusedLocals`

---

## 6. 说明

- 本机无 `gh` 凭据，云端 CI 由推送后自动触发；本轮全部改动已过本机门禁。
- 未修改项均已明确标注，未以「已完成」表述。
