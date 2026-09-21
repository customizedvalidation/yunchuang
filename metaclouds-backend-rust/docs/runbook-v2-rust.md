# Metaclouds Backend (Rust) 运维 Runbook v2

> 本文档基于 Rust 版实际代码（`src/config.rs`、`src/routes.rs`、`src/db.rs`、`src/metrics/`、`src/authz/`、`src/middleware/`）编写，不臆造。
> 对照源：Go 版 `metaclouds-backend`（`config/config.go`、`api/routes.go`、`pkg/metrics/`）。
> Go 版原始 `docs/production-deployment-runbook.md` 不存在，本文档按 Rust 版实际情况从零编写。

---

## 1. 架构概述

### 1.1 技术栈

| 组件 | 版本 / 库 | 用途 |
|------|-----------|------|
| 运行时 | Rust edition 2021 | 编译型后端 |
| Web 框架 | Axum 0.8 | HTTP 路由、中间件、状态管理 |
| 异步运行时 | Tokio 1.x（full features） | 事件循环 |
| 数据库驱动 | sqlx 0.8（sqlite + postgres 双驱动） | ORM / 连接池 / 迁移 |
| 认证 | jsonwebtoken 9.x | JWT HS256 签发与校验 |
| 密码哈希 | argon2 0.5 + bcrypt 0.15 | argon2id 新密码哈希；bcrypt 双读兼容 |
| Cookie | tower-cookies 0.11 | HttpOnly 认证 Cookie 管理 |
| 缓存 | redis 0.27 + 内置 NoopCache 降级 | Redis 可选；未启用时自动降级 |
| 指标 | prometheus 0.13 | Prometheus 文本格式 /metrics 端点 |
| 链路追踪 | tracing 0.1 + opentelemetry 0.27 + OTLP gRPC | 结构化日志 + 可选 OTel 导出 |
| 定时任务 | tokio-cron-scheduler 0.10 | 指标采集 / 调度循环 |
| API 文档 | utoipa 5.x + utoipa-swagger-ui 9.x | OpenAPI 3.0 + Swagger UI |
| 配置 | dotenvy 0.15 | .env 文件加载（生产直接读环境变量） |
| 校验 | validator 0.18 | 请求体字段校验 |

### 1.2 模块结构

```
src/
├── main.rs              # 入口：config → db → router → serve
├── lib.rs               # 库导出
├── config.rs            # 全局配置（~60 字段，环境变量驱动）
├── db.rs                # 双驱动连接池 + sqlx 迁移 + 种子数据
├── error.rs             # 统一错误类型 + 9 类错误码
├── response.rs          # 统一 JSON 信封 {success,data,timestamp}
├── routes.rs            # 路由组装（61 路径 / 107 方法）
├── auth/                # 认证（login/jwt/csrf/password/middleware）
├── authz/               # RBAC 30 权限 + 角色矩阵
├── cache/               # Redis 缓存 + NoopCache 降级
├── handlers/            # 各业务域 HTTP handler（18 个子模块）
├── metrics/             # 13 业务 Gauge + 3 HTTP 指标
├── middleware/          # request_id / logger / timing / security_headers / error_handler / panic_recover
├── models/              # 数据模型
├── orm/                 # 数据库查询封装
├── scheduler/           # 调度器
├── services/            # 业务服务层
└── tracing/              # 日志/追踪初始化
```

### 1.3 端口与监听

| 环境 | 监听地址 | 端口 | 说明 |
|------|----------|------|------|
| 开发 | `0.0.0.0:8001` | 8001 | `cargo run` 默认；与 Go 版 8000 并存 |
| 生产 | `0.0.0.0:8000` | 8000 | 由 `SERVER_PORT` 环境变量控制 |
| Prometheus 抓取 | 同端口 `/metrics` | — | 横切端点，无 JWT |
| Swagger UI | 同端口 `/swagger-ui` | — | 横切端点，无 JWT |

> **注意**：Rust 版没有独立的 Prometheus 端口。`/metrics` 挂在应用同一监听端口上。`PROMETHEUS_PORT` 配置项保留供 Go 版对齐，但 Rust 版不使用。

---

## 2. 环境要求

### 2.1 运行时依赖

| 依赖 | 最低版本 | 必需性 | 说明 |
|------|----------|--------|------|
| Rust 工具链 | 1.81+ | 编译时必需 | edition 2021；推荐 stable |
| PostgreSQL | 14+ | 生产必需 | `USE_SQLITE=false` 时连接 |
| SQLite | 内置 | 开发/测试 | `sqlite::memory:` 零依赖 |
| Redis | 6+ | 可选 | `REDIS_ENABLED=true` 时必需；否则 NoopCache 降级 |
| OpenTelemetry Collector | 任意 | 可选 | `OTEL_ENABLED=true` 时必需（gRPC 4317） |

### 2.2 系统资源建议

| 资源 | 最低 | 推荐 |
|------|------|------|
| CPU | 1 核 | 2 核+ |
| 内存 | 256 MB | 512 MB+ |
| 磁盘 | 100 MB（二进制） | 500 MB+（含日志/数据） |

### 2.3 生产环境硬性约束（启动期 fail-fast）

以下条件在 `SERVER_ENV=production` 时违反任一则**拒绝启动**：

- `MEMORY_STORE_ENABLED` 必须为 `false`
- `ALLOW_PUBLIC_REGISTRATION` 必须为 `false`
- `USE_SQLITE` 必须为 `false`
- `DATABASE_SSL_MODE` 不能为 `disable`
- `ALLOWED_ORIGINS` 必须设置且不能包含 `*`
- `JWT_SECRET` 必须非空且 ≥32 字符

---

## 3. 配置说明

全部配置通过环境变量加载（优先 `.env` 文件）。以下按功能分组，字段名与 Go 版逐一对齐。

### 3.1 服务

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `SERVER_HOST` | `0.0.0.0` | 监听地址 |
| `SERVER_PORT` | `8000` | 监听端口（开发用 8001） |
| `SERVER_ENV` | `development` | 运行环境；`production` 触发安全约束 |
| `ALLOWED_ORIGINS` | （空） | CORS 允许的源，逗号分隔；生产必填且不能为 `*` |

### 3.2 数据库

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `USE_SQLITE` | `true` | 使用 SQLite 驱动 |
| `MEMORY_STORE_ENABLED` | `true` | SQLite 内存模式（生产必须 false） |
| `DATABASE_HOST` | `localhost` | PostgreSQL 主机 |
| `DATABASE_PORT` | `5432` | PostgreSQL 端口 |
| `DATABASE_USER` | `metaclouds` | PostgreSQL 用户 |
| `DATABASE_PASSWORD` | （空） | PostgreSQL 密码 |
| `DATABASE_NAME` | `metaclouds` | 数据库名 |
| `DATABASE_SSL_MODE` | `disable` | SSL 模式（生产不能为 disable） |
| `DATABASE_URL` | `sqlite::memory:` | 直连连接串（优先于拼装 DSN） |

### 3.3 Redis

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `REDIS_ENABLED` | `false` | 启用 Redis 缓存 |
| `REDIS_HOST` | `localhost` | Redis 主机 |
| `REDIS_PORT` | `6379` | Redis 端口 |
| `REDIS_PASSWORD` | （空） | Redis 密码 |
| `REDIS_DB` | `0` | 数据库编号 |
| `REDIS_URL` | （空） | 完整连接串（优先于分项拼装） |

### 3.4 JWT

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `JWT_SECRET` | （空，必填） | HS256 密钥；必须 ≥32 字符 |
| `JWT_EXPIRATION_HOURS` | `24` | 访问令牌有效期（小时） |
| `JWT_REFRESH_EXPIRATION_HOURS` | `168` | 刷新令牌有效期（小时，7 天） |
| `JWT_EXPIRES_SECONDS` | `86400` | Phase 0 兼容字段（秒） |

### 3.5 监控

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `PROMETHEUS_ENABLED` | `true` | 启用 /metrics 端点 |
| `PROMETHEUS_PORT` | `9090` | 保留字段（Rust 版不单独监听） |
| `MONITORING_ENABLED` | `true` | 启用监控模块 |
| `ALERT_ENABLED` | `true` | 启用告警 |
| `METRICS_COLLECTION_INTERVAL_SECONDS` | `15` | 指标采集间隔 |

### 3.6 K8S

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `K8S_ENABLED` | `true` | 启用 K8s 集成 |
| `K8S_NAMESPACE` | `metaclouds` | 命名空间 |
| `K8S_CONFIG_PATH` | `~/.kube/config` | kubeconfig 路径 |
| `K8S_SIMULATION_MODE` | `true` | 模拟模式（不直连集群） |

### 3.7 调度

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `SCHEDULER_ENABLED` | `true` | 启用调度器 |
| `SCHEDULER_INTERVAL_SECONDS` | `10` | 调度循环间隔 |

### 3.8 链路追踪

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `TRACING_ENABLED` | `false` | 启用 Jaeger 兼容追踪 |
| `TRACING_SERVICE_NAME` | `metaclouds-backend` | 服务名 |
| `JAEGER_ENDPOINT` | `http://localhost:14268/api/traces` | Jaeger 上报端点 |
| `OTEL_ENABLED` | `false` | 启用 OpenTelemetry OTLP gRPC 导出 |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4317` | OTLP gRPC endpoint |
| `OTEL_SAMPLE_RATE` | `1.0` | 采样率（0.0~1.0） |
| `SLOW_QUERY_THRESHOLD_MS` | `500` | SQL 慢查询阈值（毫秒） |

### 3.9 配置中心

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `CONFIG_CENTER_ENABLED` | `false` | 启用配置中心 |
| `CONFIG_CENTER_ENDPOINTS` | `localhost:2379` | etcd 端点 |
| `CONFIG_CENTER_PREFIX` | `/metaclouds/config/` | 配置前缀 |

### 3.10 限流

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `RATE_LIMIT_ENABLED` | `true` | 启用限流 |
| `RATE_LIMIT_REQUESTS` | `100` | 时间窗口内最大请求数 |
| `RATE_LIMIT_DURATION_SECONDS` | `60` | 限流窗口（秒） |

### 3.11 熔断

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `CIRCUIT_BREAKER_ENABLED` | `true` | 启用熔断 |
| `CIRCUIT_BREAKER_THRESHOLD` | `10` | 熔断触发失败次数 |
| `CIRCUIT_BREAKER_TIMEOUT_SECONDS` | `30` | 熔断恢复等待时间 |

### 3.12 日志

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `LOG_LEVEL` | `info` | 日志级别（trace/debug/info/warn/error） |
| `LOG_FORMAT` | `json` | 日志格式（json / pretty） |
| `LOG_OUTPUT` | `console` | 输出目标（console / file） |
| `LOG_PATH` | `/var/log/metaclouds/backend.log` | 日志文件路径 |

### 3.13 HTTP 服务

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `MAX_REQUEST_BODY_SIZE` | `10485760` | 请求体上限（字节，10MB） |
| `READ_TIMEOUT_SECONDS` | `30` | 读超时 |
| `WRITE_TIMEOUT_SECONDS` | `30` | 写超时 |
| `IDLE_TIMEOUT_SECONDS` | `60` | 空闲超时 |
| `TRUSTED_PROXIES` | （空） | 可信代理 IP 列表，逗号分隔 |

### 3.14 分页 / 慢请求

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `DEFAULT_PAGE_SIZE` | `10` | 默认分页大小 |
| `MAX_PAGE_SIZE` | `100` | 最大分页大小 |
| `SLOW_REQUEST_THRESHOLD_MS` | `2000` | 慢请求阈值（毫秒） |

### 3.15 功能开关

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `FEATURE_GPU_ALLOCATION` | `true` | GPU 分配功能开关 |
| `FEATURE_JOB_SCHEDULER` | `true` | 作业调度功能开关 |
| `FEATURE_MONITORING` | `true` | 监控功能开关 |
| `FEATURE_SECURITY_POLICIES` | `true` | 安全策略功能开关 |

### 3.16 注册 / Cookie

| 环境变量 | 默认值 | 说明 |
|----------|--------|------|
| `ALLOW_PUBLIC_REGISTRATION` | `false` | 允许公开注册（生产必须 false） |
| `COOKIE_SAME_SITE` | `lax` | Cookie SameSite 模式（lax/strict/none） |

---

## 4. 部署步骤

### 4.1 本地开发（SQLite 内存模式）

```powershell
cd D:\YCYD\metaclouds-backend-rust

# 设置最小必需环境变量
$env:JWT_SECRET = "dev-only-jwt-secret-key-at-least-32-chars-long"
$env:SERVER_PORT = "8001"
$env:USE_SQLITE = "true"
$env:MEMORY_STORE_ENABLED = "true"
$env:REDIS_ENABLED = "false"
$env:LOG_LEVEL = "debug"
$env:LOG_FORMAT = "pretty"

# 编译并启动（SQLite 内存库，启动时自动迁移 + 种子 admin）
cargo run
```

启动后访问：
- API 基础路径：`http://localhost:8001/api/v1`
- Swagger UI：`http://localhost:8001/swagger-ui`
- Prometheus 指标：`http://localhost:8001/metrics`
- 默认管理员：`admin` / `Admin@123456`

### 4.2 生产部署

#### 4.2.1 Docker 镜像构建

> **[待目标环境实演]** 本机无 Docker，以下为推荐方案，未在本机构建验证。

多阶段构建 Dockerfile（推荐）：

```dockerfile
# ---- 构建阶段 ----
FROM rust:1.81-alpine AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo build --release --locked

# ---- 运行阶段 ----
FROM alpine:3.20
RUN addgroup -S app && adduser -S app -G app
USER 10001
WORKDIR /app
COPY --from=builder /app/target/release/metaclouds-backend-rust .
COPY migrations ./migrations
EXPOSE 8000
ENTRYPOINT ["./metaclouds-backend-rust"]
```

构建与运行：

```bash
docker build -t metaclouds-backend-rust:0.1.0 .
docker run -d --name metaclouds-rust \
  -p 8000:8000 \
  -e SERVER_ENV=production \
  -e SERVER_PORT=8000 \
  -e JWT_SECRET=<生产密钥> \
  -e DATABASE_URL=postgres://metaclouds:<密码>@<pg-host>:5432/metaclouds \
  -e USE_SQLITE=false \
  -e MEMORY_STORE_ENABLED=false \
  -e DATABASE_SSL_MODE=require \
  -e ALLOWED_ORIGINS=https://metaclouds.example.com \
  -e REDIS_ENABLED=true \
  -e REDIS_URL=redis://:<密码>@<redis-host>:6379/0 \
  metaclouds-backend-rust:0.1.0
```

#### 4.2.2 docker-compose（推荐用于测试/预发）

> **[待目标环境实演]** 本机无 docker-compose。

```yaml
version: "3.8"
services:
  postgres:
    image: postgres:14-alpine
    environment:
      POSTGRES_DB: metaclouds
      POSTGRES_USER: metaclouds
      POSTGRES_PASSWORD: <密码>
    volumes:
      - pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U metaclouds"]
      interval: 5s
      timeout: 3s
      retries: 5

  redis:
    image: redis:6-alpine
    command: redis-server --requirepass <密码>
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5

  backend:
    build: .
    ports:
      - "8000:8000"
    environment:
      SERVER_ENV: production
      SERVER_PORT: "8000"
      JWT_SECRET: <生产密钥>
      DATABASE_URL: postgres://metaclouds:<密码>@postgres:5432/metaclouds
      USE_SQLITE: "false"
      MEMORY_STORE_ENABLED: "false"
      DATABASE_SSL_MODE: disable
      REDIS_ENABLED: "true"
      REDIS_URL: redis://:<密码>@redis:6379/0
      ALLOWED_ORIGINS: http://frontend:3000
    depends_on:
      postgres:
        condition: service_healthy
      redis:
        condition: service_healthy

volumes:
  pgdata:
```

#### 4.2.3 Kubernetes 部署

> **[待目标环境实演]** 本机无 kubectl。以下为推荐清单结构。

推荐 14 个 K8s 清单：

| # | 清单 | 关键配置 |
|---|------|----------|
| 1 | `namespace.yaml` | metaclouds 命名空间 |
| 2 | `secret.yaml` | JWT_SECRET、DATABASE_PASSWORD、REDIS_PASSWORD |
| 3 | `configmap.yaml` | 非敏感环境变量 |
| 4 | `deployment.yaml` | 三探针（liveness/readiness/startupProbe）、非 root UID 10001、readOnlyRootFilesystem |
| 5 | `service.yaml` | ClusterIP 8000 |
| 6 | `hpa.yaml` | 按 CPU 50% / 内存 70% 扩缩 |
| 7 | `pdb.yaml` | minAvailable: 1 |
| 8 | `networkpolicy.yaml` | 限制入站仅前端/监控 |
| 9 | `ingress.yaml` | HTTPS + 域名路由 |
| 10 | `prometheus-rule.yaml` | 16 条告警规则 |
| 11 | `service-monitor.yaml` | Prometheus 抓取 /metrics |
| 12 | `cronjob-migration.yaml` | 数据库迁移 Job |
| 13 | `otel-collector.yaml` | OTLP 接收（可选） |
| 14 | `grafana-dashboard.yaml` | Dashboard 配置（可选） |

Deployment 探针配置建议：

```yaml
livenessProbe:
  httpGet: { path: /health, port: 8000 }
  initialDelaySeconds: 10
  periodSeconds: 15
readinessProbe:
  httpGet: { path: /health, port: 8000 }
  initialDelaySeconds: 5
  periodSeconds: 10
startupProbe:
  httpGet: { path: /health, port: 8000 }
  failureThreshold: 30
  periodSeconds: 5
```

> **注意**：Rust 版已实现根级 `/health` 端点（无 JWT，返回 200）。K8s 三探针统一使用 `/health`；`/metrics` 仅供 Prometheus 抓取。

### 4.3 数据库迁移

迁移通过 sqlx 内置宏 `sqlx::migrate!("./migrations")` 在启动时自动执行。

| 迁移文件 | 内容 |
|----------|------|
| `001_initial.sql` | 初始表结构（users 等） |
| `002_tenants_clusters_resources.sql` | 租户/集群/资源表 |
| `003_b1_seed.sql` | B1 种子数据 |
| `004_b2_resources.sql` | B2 资源表补充 |
| `005_b5_acceleration.sql` | B5 加速套件表 |
| `006_b3_jobs.sql` | B3 作业/GPU 表 |
| `007_b4_scheduler.sql` | B4 调度器/分区/配额表 |
| `008_b6_governance.sql` | B6 告警/安全策略/监控表 |

- **双驱动**：SQLite 与 PostgreSQL 共用 `migrations/` 目录下的 SQL。当前 SQLite 是唯一实际运行的驱动；PostgreSQL 分支为代码预留。
- **启动时自动迁移**：`db.rs::connect_and_migrate()` 在进程启动时运行待执行迁移，无需手动 `migrate up`。
- **种子数据**：迁移完成后自动播种默认租户（id=1）和管理员（admin / Admin@123456）。

---

## 5. 升级步骤

### 5.1 滚动更新流程

> **[待目标环境实演]** 本机无 K8s，以下为标准滚动更新流程。

1. **前置检查**
   - 确认新版本镜像在测试环境通过 241 项测试
   - 确认无新的不可逆数据库迁移
   - 备份当前数据库

2. **推送新镜像**
   ```bash
   docker build -t registry.example.com/metaclouds-backend-rust:<new-tag> .
   docker push registry.example.com/metaclouds-backend-rust:<new-tag>
   ```

3. **K8s 滚动更新**
   ```bash
   kubectl set image deployment/metaclouds-backend-rust \
     backend=registry.example.com/metaclouds-backend-rust:<new-tag> \
     -n metaclouds

   kubectl rollout status deployment/metaclouds-backend-rust -n metaclouds --timeout=300s
   ```

4. **验证**
   - 检查新 Pod Ready 状态
   - 访问 `/metrics` 确认 200
   - 登录验证业务功能

### 5.2 数据库迁移注意事项

- Rust 版迁移在启动时自动执行，无需单独运行迁移 Job
- 新增迁移文件（`009_*.sql` 等）需向后兼容：不得 DROP/ALTER 已有列
- 生产环境建议先在预发环境执行迁移，确认无锁表/长时间阻塞

### 5.3 配置变更

- 修改 ConfigMap 或 Secret 后需重启 Pod 生效
- 敏感配置（JWT_SECRET、DATABASE_PASSWORD）通过 Secret 注入
- 配置变更后通过 `kubectl rollout restart deployment/metaclouds-backend-rust` 触发滚动重启

### 5.4 回滚触发条件

- 新版本 Pod CrashLoopBackOff
- 登录成功率下降 >5%
- 5xx 错误率上升 >1%
- 数据库迁移失败

---

## 6. 回滚方案

### 6.1 回滚到 Go 版

当 Rust 版出现严重问题需要切回 Go 版后端时：

1. **前端 API base 切换**
   - 开发环境：Vite proxy target 从 `http://localhost:8001` 改回 `http://localhost:8000`
   - 生产环境：Nginx 反向代理 `/api/` upstream 从 Rust 改回 Go

2. **K8s 流量切换**（待目标环境实演）
   ```bash
   # 将 Service 的 selector 指向 Go 版 Deployment
   kubectl patch service metaclouds-backend -n metaclouds -p '{"spec":{"selector":{"app":"metaclouds-backend-go"}}}'
   ```

3. **停止 Rust 版**
   ```bash
   kubectl scale deployment/metaclouds-backend-rust --replicas=0 -n metaclouds
   ```

4. **确认 Go 版正常**
   - 验证 Go 版 Pod Running
   - 前端登录、Dashboard、CRUD 正常

### 6.2 回滚到 Rust 上一版本

1. **镜像 tag 回退**
   ```bash
   kubectl rollout undo deployment/metaclouds-backend-rust -n metaclouds
   ```
   或指定历史版本：
   ```bash
   kubectl rollout undo deployment/metaclouds-backend-rust --to-revision=<n> -n metaclouds
   ```

2. **配置回退**
   ```bash
   kubectl apply -f configmap-previous.yaml
   kubectl rollout restart deployment/metaclouds-backend-rust -n metaclouds
   ```

3. **验证**
   - Pod Ready 后访问 `/metrics`
   - 登录验证

### 6.3 数据兼容性说明

- Rust 版使用**独立数据库**（SQLite 内存或独立 PostgreSQL 库），不修改 Go 版表结构
- 两套后端不共享数据存储；切换期间数据不互通
- 如需数据迁移，需单独编写 ETL 脚本将 Go 版 PostgreSQL 数据导入 Rust 版数据库
- 回滚到 Go 版后，Rust 版期间产生的数据不自动同步回 Go 版

---

## 7. 故障排查

### 7.1 服务无法启动

| 症状 | 排查方向 | 解决 |
|------|----------|------|
| 进程立即退出 | 检查 `JWT_SECRET` 是否设置且 ≥32 字符 | 设置 `JWT_SECRET` 环境变量 |
| 进程立即退出 | 生产环境 `MEMORY_STORE_ENABLED=true` | 设为 `false` |
| 进程立即退出 | 生产环境 `USE_SQLITE=true` | 设为 `false` |
| 进程立即退出 | 生产环境 `DATABASE_SSL_MODE=disable` | 改为 `require` 或 `prefer` |
| 进程立即退出 | 生产环境 `ALLOWED_ORIGINS` 为空 | 设置具体域名，不含 `*` |
| 端口绑定失败 | `SERVER_PORT` 被占用 | `netstat -ano | findstr :8000` 查占用进程；换端口 |
| 数据库连接失败 | PostgreSQL 不可达 | 检查 `DATABASE_HOST`/`DATABASE_PORT`/网络连通性 |
| Redis 连接失败 | `REDIS_ENABLED=true` 但 Redis 不可达 | 设 `REDIS_ENABLED=false` 降级为 NoopCache |

### 7.2 认证失败

| 症状 | 排查方向 | 解决 |
|------|----------|------|
| 登录后 401 | JWT 过期 | 重新登录；检查 `JWT_EXPIRATION_HOURS` |
| 登录后 401 | JWT 签名不匹配 | 确认前后端使用同一 `JWT_SECRET` |
| Cookie 不携带 | `SameSite` 配置不当 | 开发用 `lax`；跨域生产用 `none`+Secure |
| 写操作 403 | RBAC 权限不足 | 检查用户角色（admin/manager/user） |
| CSRF 403 | 缺少 `X-CSRF-Token` 头 | 先 `GET /auth/csrf` 获取令牌，写请求携带 |

Cookie 属性确认（开发模式）：
- `access_token`：`HttpOnly; SameSite=Lax; Path=/; Max-Age=86400`
- `csrf_token`：`SameSite=Lax; Path=/; Max-Age=86400`（非 HttpOnly，前端可读）

### 7.3 数据库错误

| 症状 | 排查方向 | 解决 |
|------|----------|------|
| 连接池耗尽 | 并发连接超过上限 | SQLite 内存模式强制单连接；PostgreSQL 最大 100 连接 |
| 迁移失败 | SQL 语法不兼容 | 确认迁移文件与当前驱动匹配 |
| `RowNotFound` 错误 | 查询记录不存在 | 前端处理 404 信封 `{success:false, code:"NOT_FOUND"}` |
| 唯一约束冲突 | 重复插入 | 前端处理 409 信封 `{success:false, code:"CONFLICT"}` |
| SQLite vs Postgres 差异 | 方言不兼容 | 当前仅 SQLite 实际运行；PostgreSQL 待迁移方言就绪 |

### 7.4 性能问题

| 症状 | 排查方向 | 解决 |
|------|----------|------|
| 慢请求 | `SLOW_REQUEST_THRESHOLD_MS` 默认 2000ms | 查看日志中超过阈值的请求 |
| 慢 SQL | `SLOW_QUERY_THRESHOLD_MS` 默认 500ms | 查看 warn 日志中的慢查询 |
| 缓存命中率低 | Redis 未启用或命中率低 | 启用 `REDIS_ENABLED=true` |
| 高延迟 | 查看 `/metrics` 中 `http_request_duration_seconds` | 分析 P99 延迟分布 |

### 7.5 可观测性

**日志字段**：
- `trace_id` / `span_id`：分布式追踪 ID
- `request_id`：请求 UUID（响应头 `X-Request-Id`）
- `X-Trace-Id`：响应头携带追踪 ID
- `X-Response-Time`：响应耗时（毫秒）

**端点**：
- `/metrics`：Prometheus 文本格式指标
- `/swagger-ui`：交互式 API 文档
- `/api-docs/openapi.json`：原始 OpenAPI 3.0 spec

---

## 8. 生产性能优化

> 本章基于 Phase 4 P4-03 压测报告（`docs/performance-benchmark.md`）结论编写，
> 数据均来自该报告，不臆造新数据。核心结论：**SQLite 文件锁是中并发瓶颈，
> PostgreSQL 接入是最高影响优化**。

### 8.1 Phase 4 压测结论摘要

压测对象：Rust 版（SQLite 文件库）vs Go 版（内存存储），6 个端点，并发度 10 / 50。

| 并发 | 指标 | Rust (SQLite) | Go (内存存储) | 结论 |
|------|------|---------------|---------------|------|
| conc=10 | 单条查询吞吐 | 7556 req/s | 6339 req/s | Rust 单查询占优 ~1.19x，内存略低 |
| conc=50 | 整体吞吐 | 仅较 conc=10 增 1.04x | 领先 Rust 1.4x–2.3x | Go 全面领先 |
| conc=50 | 登录吞吐 | 50 req/s | 169 req/s | argon2id CPU 密集，登录端点受限 |

**关键诊断**：

- **低并发(10)**：Rust 单条查询占优（7556 vs 6339 req/s），内存占用略低——说明
  Rust 运行时本身不弱，瓶颈不在语言。
- **中并发(50)**：Go 全面领先 1.4x–2.3x，主因是 **Go 用内存存储 vs Rust 用
  SQLite 文件锁**。
- **Rust 扩展性差**：conc=10→50 吞吐仅增 1.04x，远低于线性预期——典型的
  SQLite **单写者文件锁（database-level lock）**瓶颈：写事务串行化，读阻塞于写。

### 8.2 PostgreSQL 接入对性能的影响

| 场景 | 适用 | 说明 |
|------|------|------|
| SQLite | 开发 / 测试 / 小规模单实例 | 零依赖、零运维；单写者文件锁，中并发即瓶颈 |
| PostgreSQL | 生产 / 多实例 / 高并发 | 连接池 + 行级锁 / MVCC 替代文件锁，支持水平扩展 |

**预期提升**：中并发(conc=50)吞吐 **2–5x**（连接池复用 + PostgreSQL MVCC/行级锁
替代 SQLite 文件锁）。这是 C 类运维优化中**单项影响最大**的动作。

**迁移步骤**：

1. 配置 `DATABASE_URL=postgres://<user>:<pwd>@<host>:5432/<db>`
2. 设 `USE_SQLITE=false`、`MEMORY_STORE_ENABLED=false`
3. 设生产安全约束：`DATABASE_SSL_MODE=require`、`SERVER_ENV=production`
4. 启动时自动执行 `sqlx::migrate!("./migrations")`（见 §4.3）
5. 如需数据迁移：单独编写 ETL 脚本从 SQLite 文件导出→导入 PostgreSQL
   （Rust 版与 Go 版本就不共享数据，见 §6.3）

### 8.3 连接池调优（sqlx PgPool）

sqlx 默认 `max_connections=10`，生产需上调。对照 Go 版参数
（max_open=100 / max_idle=20 / max_lifetime=300s），Rust 版在 `src/db.rs`
中通过 `PgPoolOptions` 对齐：

| 参数 | sqlx 方法 | 建议值 | 说明 |
|------|-----------|--------|------|
| 最大连接 | `max_connections()` | 20–50 | 生产起步 20；按 DB 实例 max_connections 与 Pod 副本数反推 |
| 最小空闲连接 | `min_connections()` | 5–10 | 避免冷启动新建连接抖动 |
| 获取超时 | `acquire_timeout()` | 3–5s | 连接池耗尽时快速失败，避免请求堆积 |
| 空闲超时 | `idle_timeout()` | 300s（5min） | 对齐 Go max_idle_time |
| 连接生命周期 | `max_lifetime()` | 1800s（30min） | 定期回收，防 DB 侧/网络中间件断连 |

> **注意**：以上为 `src/db.rs` 中 PgPool 构造的调优方向，属代码侧改动（本次
> C8/C9 不改源码，仅给出建议值）。连接池使用率 >80% 应告警（见 §8.7）。

### 8.4 索引建议

对照 `migrations/` 现有索引梳理（已存在 ✅ / 建议补充 ⬇）：

**已有索引（无需重复创建）**：

- ✅ users(username)、users(email)、users(last_login_at)
- ✅ jobs(status, cluster_id, user_id)、jobs(tenant_id)、jobs(type)
- ✅ gpu_devices(cluster_id, status)
- ✅ gpu_allocations(job_id, status)、gpu_allocations(device_id)
- ✅ alerts(status, severity)、alerts(type, cluster_id)、alerts(tenant_id)
- ✅ resources(cluster_id)、resources(type)、resources(status)
- ✅ partitions(cluster_id, status)、resource_quotas(tenant_id, partition_id)
- ✅ acceleration_suites(tenant_id, status)、checkpoints(tenant_id, job_id)

**建议新增索引（高频查询但当前缺失）**：

| 表 | 建议索引 | 服务的查询 |
|----|----------|-----------|
| tenants | `CREATE INDEX idx_tenants_name ON tenants (name);` | 租户按 name 搜索/列表 |
| clusters | `CREATE INDEX idx_clusters_name ON clusters (name);` | 集群按 name 搜索（/clusters list + search） |
| clusters | `CREATE INDEX idx_clusters_tenant_id ON clusters (tenant_id);` | 按租户过滤集群 |
| jobs | `CREATE INDEX idx_jobs_tenant_status_created ON jobs (tenant_id, status, created_at DESC);` | Dashboard 作业统计 / 按租户+状态+时间倒序列表（现有 idx_jobs_tenant_id 仅单列，覆盖不了 status+排序） |

> 新增索引需以后续迁移文件（如 `009_perf_indexes.sql`）落地，遵循 §5.2
> 向后兼容原则（CREATE INDEX IF NOT EXISTS，不 DROP/ALTER 已有列）。

### 8.5 缓存优化

Redis 缓存层已在 P3-01 实现，未启用时 `NoopCache` 优雅降级（见 §3.3）。

**建议缓存对象与 TTL**：

| 缓存对象 | 建议 TTL | 说明 |
|----------|----------|------|
| 用户会话 jti / 登出令牌黑名单 | 对齐 JWT 有效期 | P3-01 `src/cache/session.rs` 已实现 |
| 租户配置 | 60s | 变更低频，读多 |
| 集群状态（/clusters、/k8s/.../health） | 10–30s | 短 TTL，容忍短暂陈旧 |
| Dashboard 统计（/monitoring/dashboard 13 指标） | 30s | 避免每次看板刷新打全表 count |

**命中率监控**：在 `/metrics` 中暴露 `cache_hit_rate`（建议
`rate(cache_hits_total[5m]) / rate(cache_requests_total[5m])`），命中率 <80% 告警
（对应 §9 告警规则 #8 LowRedisCacheHit）。

### 8.6 JWT / 认证端点优化

压测数据：conc=50 时登录端点 Rust 50 req/s vs Go 169 req/s。根因：**argon2id
是 CPU 密集型哈希**，每个登录请求都做一次 argon2id 计算，单核串行成为吞吐天花板。

**优化建议**：

1. **登录结果短 TTL 缓存**：对同一用户名的登录校验结果做 5s 短 TTL 缓存，
   削峰（注意安全：仅缓存"认证失败/成功"的短时结论，不缓存密码本身）。
2. **连接池复用**：见 §8.3，登录也走连接池，避免每请求新建 PG 连接。
3. **哈希成本权衡**：argon2id 安全性高但慢；Go 版用 bcrypt。Rust 版已实现
   argon2id + bcrypt 双读（见 §9 安全清单），新密码继续用 argon2id 保安全；
   若登录吞吐成为瓶颈，可在生产适当调低 argon2id 时间/内存成本参数，
   或对非敏感场景评估更轻量方案。**不建议为吞吐牺牲主密码哈希安全级别**。

### 8.7 监控告警阈值（性能相关）

在 §9 告警规则基础上，补充生产性能告警阈值：

| 指标 | 告警阈值 | 级别 | 说明 |
|------|----------|------|------|
| P99 延迟 | `> 500ms` 持续 5m | warning | `histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 0.5` |
| 错误率（5xx） | `> 1%` | critical | `rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m]) > 0.01` |
| 连接池使用率 | `> 80%` 持续 5m | warning | 已借出连接 / max_connections；接近上限需调大 max_connections 或扩容 |
| CPU 使用率 | `> 80%` 持续 5m | warning | 对应 §9 #16 CPUSaturation（阈值 85%）；按 HPA 触发扩容 |

> 上述阈值为建议起点，上线后按实际流量基线调整。

---

## 9. 监控告警

### 8.1 业务指标（13 个 Gauge）

| 指标名 | 类型 | 说明 |
|--------|------|------|
| `metaclouds_total_users` | Gauge | 用户总数 |
| `metaclouds_active_users` | Gauge | 活跃用户数 |
| `metaclouds_total_tenants` | Gauge | 租户总数 |
| `metaclouds_total_clusters` | Gauge | 集群总数 |
| `metaclouds_total_resources` | Gauge | 资源总数 |
| `metaclouds_total_jobs` | Gauge | 作业总数 |
| `metaclouds_running_jobs` | Gauge | 运行中作业数 |
| `metaclouds_total_gpus` | Gauge | GPU 设备总数 |
| `metaclouds_allocated_gpus` | Gauge | 已分配 GPU 数 |
| `metaclouds_total_datasets` | Gauge | 数据集总数 |
| `metaclouds_total_alerts` | Gauge | 告警总数 |
| `metaclouds_active_alerts` | Gauge | 活跃告警数 |
| `metaclouds_system_uptime` | Gauge | 系统运行时间（秒） |

### 8.2 HTTP 指标（3 个）

| 指标名 | 类型 | 标签 | 说明 |
|--------|------|------|------|
| `http_requests_total` | CounterVec | method, path, status | 请求总数 |
| `http_request_duration_seconds` | HistogramVec | method, path | 请求延迟（buckets: 5ms~10s） |
| `http_requests_in_flight` | Gauge | — | 当前在处理的请求数 |

### 8.3 告警规则建议（16 条 PrometheusRule）

| # | 告警名 | 条件 | 建议级别 |
|---|--------|------|----------|
| 1 | ServiceDown | `up{job="metaclouds-backend"} == 0` 持续 1m | critical |
| 2 | High5xxRate | `rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m]) > 0.05` | critical |
| 3 | HighLatencyP99 | `histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 2` | warning |
| 4 | HighInFlight | `http_requests_in_flight > 100` | warning |
| 5 | HighActiveAlerts | `metaclouds_active_alerts > 20` | warning |
| 6 | GPUAllocationHigh | `metaclouds_allocated_gpus / metaclouds_total_gpus > 0.9` | warning |
| 7 | RunningJobsHigh | `metaclouds_running_jobs > 50` | info |
| 8 | LowRedisCacheHit | （Redis 命中率 <80%） | warning |
| 9 | DatabaseConnectionPoolExhausted | 连接池等待 >5s | critical |
| 10 | SlowQueryCountHigh | `rate(sql_slow_queries_total[5m]) > 0.1` | warning |
| 11 | CircuitBreakerOpen | 熔断器打开持续 1m | critical |
| 12 | RateLimitTriggered | `rate(http_requests_total{status="429"}[5m]) > 0` | warning |
| 13 | UptimeDropped | `metaclouds_system_uptime` 重置到低值（重启） | info |
| 14 | DiskSpaceLow | 容器磁盘 >85% | warning |
| 15 | MemoryUsageHigh | 容器内存 >80% | warning |
| 16 | CPUSaturation | CPU >85% 持续 5m | warning |

### 8.4 Grafana 面板建议

- **Overview**：13 业务 Gauge 仪表盘
- **HTTP 延迟**：P50/P95/P99 直方图
- **请求量**：`http_requests_total` 按路径/状态码
- **In-flight**：`http_requests_in_flight` 趋势
- **GPU 分配**：`allocated_gpus / total_gpus` 饼图
- **告警**：`active_alerts` 趋势

---

## 10. 安全清单

| 项目 | 实现 | 配置 |
|------|------|------|
| JWT 算法 | HS256 | `JWT_SECRET` ≥32 字符 |
| 密码哈希 | argon2id（新）+ bcrypt 双读（兼容 Go 版） | 自动 |
| CSRF 防护 | 双提交令牌（Cookie + `X-CSRF-Token` 头） | 自动 |
| RBAC | 30 权限 + 3 角色（admin/manager/user） | `authz/mod.rs` |
| HttpOnly Cookie | `access_token` 标记 HttpOnly | 自动 |
| SameSite | 默认 Lax；生产可配 None（需 Secure） | `COOKIE_SAME_SITE` |
| CSP | 开发宽松 / 生产严格（`default-src 'self'`） | `SERVER_ENV=production` 自动收紧 |
| HSTS | 生产环境 `max-age=31536000; includeSubDomains` | `SERVER_ENV=production` 自动启用 |
| X-Frame-Options | `DENY` | 自动 |
| X-Content-Type-Options | `nosniff` | 自动 |
| 非 root 容器 | UID 10001 | Dockerfile `USER 10001` |
| readOnlyRootFilesystem | 推荐启用 | K8s securityContext |
| 密码错误不泄露 | 统一信封 `{success:false, code:"UNAUTHORIZED"}` | 自动 |
| 内部错误不泄露 | 500 不暴露内部堆栈，仅日志记录 | 自动 |
