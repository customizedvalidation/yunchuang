# Metaclouds 后端 Rust 版 — 切流方案（Cutover Plan）

> 工作包：Phase 4 / P4-06
> 编制日期：2026-09-18
> 上游：`docs/rust-migration-execution-plan-2026-09-16.md`（执行计划 §7 WP-P4-06）
> 配套文档：`docs/go-retirement-checklist.md`（Go 退役检查清单）、P4-05 Runbook v2（回滚手册）
>
> **本方案性质**：纯方案/文档。切流与观察的实际执行在**目标生产环境**进行，本机仅交付方案与检查清单。
> 文中标注「🔧 待目标环境执行」的步骤，均需在生产/预发环境由运维按 Runbook 操作。

---

## 0. 架构基线（本方案所依据的实际事实）

> 以下事实来自 Rust 仓库源码与 K8s 清单，非臆造。切流操作以这些事实为准。

| 维度 | Rust 版实际值 | 来源 |
|---|---|---|
| 路由前缀 | `/api/v1/...`（前端 baseURL 为相对路径 `/api/v1`） | `src/routes.rs`、`frontend/src/api/http.ts` |
| 裸机/同机端口 | Go 版 `:8000`；Rust 版 `:8001`（docker-compose 映射 `8001:8000`） | 任务背景、`docker-compose.yml` |
| 容器内监听端口 | `8000`（`SERVER_PORT=8000`） | `k8s/01-configmap.yaml`、`k8s/05-deployment.yaml` |
| K8s Service | `metaclouds-backend:8000`（ClusterIP） | `k8s/06-service.yaml` |
| K8s Ingress | `app.metaclouds.example.com` → `/api` → `metaclouds-backend:8000` | `k8s/10-ingress.yaml` |
| 指标端点 | `GET /metrics`（Prometheus 文本，无 JWT） | `src/routes.rs` |
| 交互式文档 | `GET /swagger-ui` + `GET /api-docs/openapi.json`（无 JWT） | `src/routes.rs` |
| 健康/存活探测 | **无根级 `/health`**；三探针统一用 `/metrics` | `k8s/05-deployment.yaml` 注释 |
| 业务指标 | 13 业务 Gauge（`metaclouds_` 前缀）+ 3 HTTP 指标 | P3-03 交付 |
| 告警规则 | 16 条（GPU/CPU/内存/磁盘/Job/集群/配额/安全/登录/API/DB/Redis 等） | `k8s/12-prometheusrule.yaml` |
| 日志 | JSON 格式，含 `trace_id` / `span_id` / `request_id`；响应头 `X-Trace-Id` | P3-04 交付 |
| 数据库迁移 | 8 个文件：`001_initial` ~ `008_b6_governance`（sqlx `migrate!()` 内嵌） | `migrations/*.sql` |
| 种子数据 | `003_b1_seed.sql`（default 租户 + admin 用户） | `migrations/003_b1_seed.sql` |
| 数据库 | PostgreSQL（`DATABASE_URL` 从 Secret 注入），生产 `DATABASE_SSL_MODE=require` | `k8s/01-configmap.yaml`、`02-secret.yaml` |
| Redis | 生产 `REDIS_ENABLED=true`，key 前缀 `metaclouds:` | `k8s/01-configmap.yaml`、P3-01 |
| JWT | HS256，`JWT_SECRET` 启动期强校验 **≥32 字符**（否则 fail-fast） | `src/config.rs::validate` |
| 生产强校验 | `USE_SQLITE=false`、`MEMORY_STORE_ENABLED=false`、`ALLOW_PUBLIC_REGISTRATION=false`、`ALLOWED_ORIGINS` 非空且不含 `*` | `src/config.rs::validate` |
| Cookie | `COOKIE_SAME_SITE=lax`、`COOKIE_SECURE=true`（生产 HTTPS） | `k8s/01-configmap.yaml` |
| 镜像 | `ghcr.io/customizedvalidation/yunchuang/backend-rust:<git-sha>`（禁止 latest） | `k8s/05-deployment.yaml` |
| 副本/滚动 | 3 副本，`maxSurge=1` / `maxUnavailable=0`（零宕机滚动） | `k8s/05-deployment.yaml` |

> **端口说明**：任务给定的 Nginx 灰度示例使用 `localhost:8000`(Go) / `localhost:8001`(Rust)，对应**裸机/同机双进程**部署。在 K8s 中两版是独立 Deployment + Service，Rust 容器内仍监听 8000（见上表），灰度通过 Ingress/Istio/Argo Rollouts 实现，而非改容器端口。两种部署形态下文分别给出配置示例。

---

## 1. 切流前准备（Pre-cutover）

> 目标：D-7 前全部就绪。任一项未达标不得进入灰度。

### 1.1 功能验收（依赖 P4-01 / P4-02 / P4-03）

- [ ] **Golden 回归（P4-01）**：P0/P1 级 diff = 0（状态码/信封键/字段缺失/类型不一致均为 0）。可接受差异清单（P2 文案类）< 10 条并附裁决说明。🔧 待目标环境执行
- [ ] **影子双轨（P4-02）**：连续 **5 个工作日**观察无 must-fix diff；P99 延迟不劣于 Go 版。🔧 待目标环境执行
- [ ] **性能基准（P4-03）**：压测报告归档 `docs/`，结论明确"是否达到预期"；Rust 版 P99 ≤ Go 版 120%。🔧 待目标环境执行
- [ ] **33 测试映射（P4-04）**：映射表 100% 填充，覆盖率达标。
- [ ] **Runbook v2（P4-05）**：已完成一次部署演练 + 一次回滚演练，运维可独立操作。

### 1.2 数据迁移

- [ ] **Rust 版迁移已执行**：8 个迁移文件 `001_initial` → `008_b6_governance` 在生产 PostgreSQL 上全部 up 成功（sqlx `migrate!()` 启动期自动执行，或 `sqlx migrate run` 手动确认）。🔧 待目标环境执行
- [ ] **种子数据就绪**：`003_b1_seed.sql` 已落库 —— default 租户 + admin 用户存在；RBAC 角色权限种子完整。🔧 待目标环境执行
- [ ] **历史数据迁移（如需要）**：
  - 若 Rust 版与 Go 版**共用同一 PostgreSQL 数据库**（表结构兼容），无需额外迁移，直接切连接即可。
  - 若 Rust 版使用**独立新库**，需从 Go 版数据库导入历史数据（pg_dump/psql 或 DTS），并在影子双轨期对核心表行数做对账。🔧 待目标环境执行
  - 迁移后对账：`users` / `tenants` / `clusters` / `jobs` 等核心表行数与 Go 版源库一致。🔧 待目标环境执行
- [ ] **密码哈希兼容**：Go 版老用户为 bcrypt 哈希，Rust 版登录时先尝试 bcrypt 校验、成功后自动重哈希为 argon2（P1-07 / 风险 R2）。切流后观察登录成功率，确认无大面积登录失败。🔧 待目标环境执行

### 1.3 配置核对（生产环境）

> 依据 `src/config.rs::validate` 的启动期强校验，以下任一项缺失/错误会导致 Rust 进程启动失败（fail-fast）。

- [ ] `JWT_SECRET`：**长度 ≥ 32 字符**，且与 Go 版**同一密钥**（否则切流瞬间全量用户 JWT 失效、被迫重新登录）。🔧 待目标环境执行
- [ ] `DATABASE_URL`：指向正确的 PostgreSQL，`DATABASE_SSL_MODE=require`（生产禁止 `disable`）。🔧 待目标环境执行
- [ ] `REDIS_ENABLED=true`，`REDIS_HOST` / `REDIS_PORT`（或 `REDIS_URL`）可达；Redis 中会话/缓存 key 前缀 `metaclouds:`。🔧 待目标环境执行
- [ ] `OTEL_EXPORTER_OTLP_ENDPOINT`：如启用链路追踪，指向 OTLP collector（gRPC 4317）；默认 `OTEL_ENABLED=false`，确认是否在生产开启。🔧 待目标环境执行
- [ ] `ALLOWED_ORIGINS`：精确列出前端域名（如 `https://app.metaclouds.example.com`），**不得为 `*`**（带凭据跨域时校验拒绝）。🔧 待目标环境执行
- [ ] `COOKIE_SAME_SITE`：`lax`（默认）或按前端跨域需要；若用 `none` 必须 `SERVER_ENV=production` 且 HTTPS。🔧 待目标环境执行
- [ ] `SERVER_ENV=production`：确保生产强校验全部生效（禁止内存模式/SQLite/公开注册）。🔧 待目标环境执行
- [ ] `TRUSTED_PROXIES`：填写 Nginx/Ingress 网段（如 `10.0.0.0/8`），确保真实客户端 IP 与限流/CSP 正确。🔧 待目标环境执行

### 1.4 监控就绪

- [ ] **Prometheus 抓取**：ServiceMonitor（`k8s/11-servicemonitor.yaml`）已 apply，15s 间隔抓 `/metrics`；裸机场景在 Prometheus 静态配置加 `job=metaclouds-backend-rust` 指向 `:8001/metrics`。🔧 待目标环境执行
- [ ] **Grafana 面板**：Rust 版面板已导入（13 业务指标 + 3 HTTP 指标，指标名与 Go 版 `metaclouds_` 前缀一致，现有面板可直接套用）。🔧 待目标环境执行
- [ ] **16 告警规则**：`k8s/12-prometheusrule.yaml`（`metaclouds-alerts`）已生效，确认 `HighErrorRate`(5xx>5%)、`HighLatency`(P99>2s)、`PodCrashLooping` 等关键告警可触发。🔧 待目标环境执行
- [ ] **日志收集**：JSON 日志采集管道已接入，确认含 `trace_id` / `span_id` / `request_id` 字段，可按 `X-Trace-Id` 串联日志。🔧 待目标环境执行

### 1.5 回滚预案（依赖 P4-05 Runbook v2）

- [ ] **回滚步骤已演练**：按 P4-05 Runbook v2 完成至少一次回滚演练，运维可独立操作。
- [ ] **回滚触发条件已定义**：见 §3.4。
- [ ] **回滚时间估算**：
  - 应用层（Nginx 权重切回 / 反代改 upstream）：**< 1 分钟**生效。
  - K8s 层（Ingress 改回 Service / 恢复 Deployment 流量）：**< 5 分钟**（含 Pod 就绪探测）。
- [ ] **回滚期间数据一致性**：灰度期 Go 与 Rust 共用同一数据库（或影子库已对账），回滚不会丢失已提交写操作。🔧 待目标环境执行

---

## 2. 切流策略（灰度）

> 原则：逐步放量，每档观察达标再进入下一档；Go 版全程保持运行作为热备。

### 2.1 三阶段灰度

| 阶段 | 流量比例 | 分流方式 | 观察时长 | 达标门槛 |
|---|---|---|---|---|
| **阶段 1** | 10% | Nginx/网关按权重或用户组（`split_clients` 按 `remote_addr`）分流 10% 到 Rust 版 `:8001` | 24 小时 | 错误率 <0.1%、P99 < Go 版 120%、无 P0 业务异常 |
| **阶段 2** | 50% | 权重提到 50%（或用户组扩大） | 24 小时 | 同上；额外确认数据库连接池/Redis 无瓶颈 |
| **阶段 3** | 100% | 全量切到 Rust 版；Go 版保留运行 **1 周**作为热备（不接流量） | 1 周稳定期 | 见 §4 |

### 2.2 每阶段观察指标

- **错误率**：5xx 占比 **< 0.1%**（对应 PrometheusRule `HighErrorRate` 阈值 5% 为告警红线，切流门槛更严）。🔧 待目标环境执行
- **P99 延迟**：**< Go 版同期 120%**（对应 `HighLatency` 告警阈值 P99>2s）。🔧 待目标环境执行
- **资源使用率**：CPU / Memory 在 limits 内（requests CPU 250m / Mem 256Mi；limits 500m / 512Mi），无 `OOMKilled` / Pod 频繁重启。🔧 待目标环境执行
- **业务功能**：登录、核心 CRUD、Dashboard、作业提交/GPU 分配等主流程正常；无数据不一致工单。🔧 待目标环境执行
- **JWT/Cookie**：切流用户登录态不丢失；CSRF 双提交（`X-CSRF-Token`）写操作正常（切到 Rust 的 10% 用户首次登录后 Cookie 由 Rust 下发）。🔧 待目标环境执行

### 2.3 回滚触发条件（任一满足立即回滚）

- 错误率 **> 1%**（持续 5 分钟）。
- P99 延迟 **> Go 版 200%**（持续 5 分钟）。
- 核心业务功能不可用（登录失败、作业无法提交、Dashboard 数据空等）。
- 数据不一致（对账发现核心表行数/关键字段与 Go 版分叉）。
- 16 告警中 `critical` 级（`GPUUnavailable` / `StorageLow` / `HighErrorRate` / `DatabaseConnectionPoolExhausted` / `RedisConnectionDown`）触发。

> 回滚操作严格按 **P4-05 Runbook v2** 执行（应用层 <1 分钟 / K8s <5 分钟）。

---

## 3. 切流操作步骤

### 3.1 前端 API base URL

- **开发环境**：前端 `src/api/http.ts` 的 `baseURL` 已是相对路径 `/api/v1`，无需改源码；通过 Vite dev server proxy 指向目标后端。🔧 待目标环境执行
- **生产环境**：前端构建产物部署后，`/api/v1` 由 Nginx 反向代理转发到后端，**不修改前端代码**，只改 Nginx 反代/网关路由目标。🔧 待目标环境执行

### 3.2 裸机/同机部署：Nginx 灰度分流

> 适用：Go 版 `localhost:8000` 与 Rust 版 `localhost:8001` 同机运行。

```nginx
upstream backend_go  { server localhost:8000; }
upstream backend_rust { server localhost:8001; }

# 按客户端 IP 哈希分流：10% 流量走 Rust，其余走 Go
split_clients "${remote_addr}AAA" $backend {
    10%     backend_rust;
    *       backend_go;
}

server {
    listen 443 ssl;
    server_name app.metaclouds.example.com;

    # TLS 证书配置（略，对齐现有）

    location /api/ {
        proxy_pass http://$backend;
        proxy_set_header Host              $host;
        proxy_set_header X-Real-IP         $remote_addr;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto  $scheme;
        # WebSocket / SSE（作业日志流式）
        proxy_read_timeout 300s;
        proxy_send_timeout 300s;
        client_max_body_size 100m;
    }

    # 前端静态资源（略）
}
```

**灰度调整**（🔧 待目标环境执行）：
- 阶段 1：`10% backend_rust;`
- 阶段 2：`50% backend_rust;`
- 阶段 3：`* backend_rust;`（或直接把 `$backend` 固定为 `backend_rust`，Go 版保留进程不接流量）
- 回滚：把 `backend_rust` 权重改回 `* backend_go;`，`nginx -s reload`，<1 分钟生效。

### 3.3 K8s 部署：Ingress / 服务网格灰度

> 适用：Go 版与 Rust 版为独立 Deployment + Service。Rust 侧已就绪：Ingress `/api` → `metaclouds-backend:8000`。

- **方式 A — Ingress 权重切换（最简）**：
  - 现状 Go 版 Ingress 后端为 `go-backend:8000`。
  - 新建/修改 Ingress，用 nginx ingress 注解按权重分流到 `rust-backend:8000`（即 `metaclouds-backend` Service）。🔧 待目标环境执行
  - 全量时把 Ingress 后端 Service 从 `go-backend` 改为 `metaclouds-backend`。🔧 待目标环境执行
- **方式 B — Istio / Argo Rollouts 灰度（推荐）**：
  - 用 VirtualService 按 weight 切流（10% → 50% → 100%），或 Argo Rollouts `Canary` 发布策略自动逐步放量。🔧 待目标环境执行
- **方式 C — API 网关（Kong/APISIX）**：如使用网关，修改路由 upstream 目标从 Go 服务指向 Rust 服务，按插件做权重分流。🔧 待目标环境执行

> K8s 回滚：把 Ingress/VirtualService 后端改回 `go-backend`，<5 分钟（含 Pod readiness）。Rust 版 Deployment 滚动更新策略 `maxSurge=1/maxUnavailable=0`，本身零宕机。

### 3.4 切流后立即验证（每档放量后必做）🔧 待目标环境执行

1. **登录**：`POST /api/v1/auth/login`（admin）→ 200，信封 `{success,data}`，JWT Cookie 下发。
2. **CSRF**：`GET /api/v1/auth/csrf` 取 token，写操作带 `X-CSRF-Token` 正常。
3. **核心 CRUD**：`GET /api/v1/users`、`/tenants`、`/clusters`、`/resources`、`/jobs`、`/gpus` 等带 token 访问 200。
4. **Dashboard**：`GET /api/v1/monitoring/dashboard` 数据正常。
5. **指标**：`GET /metrics` 返回 200，含 13 业务指标 + 3 HTTP 指标。
6. **文档**：`GET /swagger-ui`、`GET /api-docs/openapi.json` 可访问。
7. **未鉴权**：`GET /api/v1/users`（无 token）→ 401 信封正确。
8. **链路**：响应头含 `X-Trace-Id`，日志可按 trace_id 串联。

---

## 4. 切流后观察（1 周稳定期，D2 ~ D9）

> 阶段 3（100%）完成后进入。Go 版保持启动状态作为热备，**不接流量**。

- [ ] **每日检查**（🔧 待目标环境执行）：
  - 错误率、P99 延迟、CPU/Memory 使用率、16 告警规则触发情况。
  - 数据库连接池水位（`metaclouds_db_connections_in_use / max < 90%`）。
  - Redis 连接正常（`up{job="redis"} == 1`）。
- [ ] **业务验证**（🔧 待目标环境执行）：
  - 核心业务流程（作业提交/取消、GPU 分配/释放、告警确认/解决、调度器同步）连续正常。
  - 无数据不一致工单；核心表行数对账无漂移。
- [ ] **Go 版热备**：保持进程启动、健康检查通过、但 Nginx/Ingress 权重为 0；确保随时可在 <1 分钟切回。🔧 待目标环境执行
- [ ] **1 周后无异常**：进入 Go 退役流程（见 `go-retirement-checklist.md`）。

---

## 5. 切流 / 退役时间线

| 时间点 | 事件 | 说明 |
|---|---|---|
| **D-7** | 切流前准备完成 | 功能验收（P4-01/02/03）、数据迁移、配置核对、监控就绪、回滚演练（P4-05）全部绿灯 |
| **D0** | 10% 灰度切流 | Nginx/网关权重 10% → Rust；观察 24 小时 |
| **D1** | 50% 灰度切流 | 权重提到 50%；观察 24 小时 |
| **D2** | 100% 全量切流 | 全量切到 Rust；Go 版保留热备（不接流量） |
| **D2 ~ D9** | 1 周稳定观察期 | 每日检查指标/告警/业务；Go 版热备待命 |
| **D10** | Go 退役检查清单执行 | 按 `go-retirement-checklist.md` 逐项确认 |
| **D10+** | Go 版停止服务 | Deployment 缩容到 0 / 进程停止；进入退役后观察（24h / 7d / 30d / 90d） |

> 每档放量前确认上一档达标；任一档触发 §2.3 回滚条件，立即按 P4-05 Runbook v2 回滚，不强行推进。

---

## 6. 与 P4-05 Runbook v2 的一致性

- 本方案 §2.3 回滚触发条件、§1.5 回滚时间估算（应用层 <1 分钟 / K8s <5 分钟）与 **P4-05 Runbook v2** 回滚手册一致。
- 实际回滚操作步骤（Nginx 改权重 / Ingress 改后端 / 进程拉起）以 P4-05 Runbook v2 为准，本文不重复。
- 退役后回滚触发条件见 `go-retirement-checklist.md`。
