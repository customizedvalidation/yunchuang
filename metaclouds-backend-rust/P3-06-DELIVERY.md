# P3-06 交付说明：Docker 多阶段镜像 + K8s 清单

> 日期：2026-09-17
> 项目：metaclouds-backend-rust
> 状态：配置交付完成，待 CI/目标环境构建验证

---

## 1. 交付文件清单

### 1.1 Docker 构建（项目根目录）

| 文件 | 说明 |
|------|------|
| `Dockerfile` | 多阶段构建（builder: rust:1.81-alpine → runtime: alpine:3.20），非 root UID 10001 |
| `.dockerignore` | 排除 target/.git/tests/docs/.env 等，最小化构建上下文 |

### 1.2 Docker Compose（项目根目录）

| 文件 | 说明 |
|------|------|
| `docker-compose.yml` | backend + postgres:16-alpine + redis:7-alpine，宿主 8001→容器 8000 |
| `docker-compose.prod.yml` | 生产覆盖：固定 tag、资源限制、不暴露 DB/Redis 端口 |

### 1.3 K8s 清单（`k8s/` 目录，共 14 个 YAML）

| # | 文件 | Kind | 说明 |
|---|------|------|------|
| 1 | `00-namespace.yaml` | Namespace | `metaclouds`，含 prometheus scrape label |
| 2 | `01-configmap.yaml` | ConfigMap | 非敏感环境变量（SERVER_PORT=8000, DB/Redis 地址, LOG_LEVEL 等） |
| 3 | `02-secret.yaml` | Secret | base64 占位（jwt-secret, database-url, redis-password），fail-secure |
| 4 | `03-serviceaccount.yaml` | ServiceAccount | `metaclouds-backend`，非 default |
| 5 | `04-role.yaml` | Role + RoleBinding | 作业调度最小权限（pods/jobs/events/configmaps） |
| 6 | `05-deployment.yaml` | Deployment | 3 副本，三探针，非 root，资源限制，拓扑分布，滚动更新 |
| 7 | `06-service.yaml` | Service | ClusterIP，端口 8000 |
| 8 | `07-hpa.yaml` | HPA | CPU 70% / Memory 80%，min 2 max 10 |
| 9 | `08-pdb.yaml` | PDB | minAvailable=2 |
| 10 | `09-networkpolicy.yaml` | NetworkPolicy | 默认拒绝 + 后端入站/出站白名单 |
| 11 | `10-ingress.yaml` | Ingress | /api → backend:8000，TLS (cert-manager) |
| 12 | `11-servicemonitor.yaml` | ServiceMonitor | Prometheus Operator 抓取 /metrics |
| 13 | `12-prometheusrule.yaml` | PrometheusRule | **16 条告警规则**（GPU/CPU/内存/磁盘/Job/集群/配额/安全/登录/API 等） |
| 14 | `13-kustomization.yaml` | Kustomization | 统一资源引用 + commonLabels + images |

### 1.4 校验脚本

| 文件 | 说明 |
|------|------|
| `scripts/validate-k8s-yaml.ps1` | YAML 结构校验（63 项检查全部通过） |

---

## 2. 镜像大小静态评估

### 2.1 各层估算

| 层 | 大小估算 | 说明 |
|----|----------|------|
| `alpine:3.20` 基础镜像 | ~7 MB | Alpine 最小 rootfs |
| `ca-certificates` | ~1 MB | HTTPS 出站所需 |
| `tzdata` (Asia/Shanghai) | ~3 MB | 时区数据 |
| `wget` (busybox 内置) | ~0 MB | Alpine busybox 自带 |
| Rust release 二进制 (strip 后) | ~20-30 MB | axum + sqlx + redis + prometheus + opentelemetry |
| **总计** | **~31-41 MB** | |

### 2.2 结论

- **目标 ≤40 MB**：静态估算 31-41 MB，处于临界值。
- Rust 二进制体积偏大的主要原因：opentelemetry-otlp (gRPC/tonic) + sqlx + redis + prometheus 依赖链。
- **CI 构建后需实测**：`docker images | grep backend-rust`

### 2.3 如果超过 40MB 的优化建议

1. **musl 静态链接**：当前使用 `rust:1.81-alpine`（musl 默认 target），已静态链接。确认 `ldd` 输出 `not a dynamic executable`。
2. **UPX 压缩**（可选）：在 builder 阶段追加 `upx --best --lzma /app/target/release/metaclouds-backend-rust`，可压缩 50-70%，但增加启动延迟 0.5-2s。
3. **cargo profile 优化**：在 `Cargo.toml` 的 `[profile.release]` 追加：
   ```toml
   strip = true
   panic = "abort"
   lto = true
   codegen-units = 1
   ```
   （注意：修改 Cargo.toml 不在 P3-06 范围，需后续 Phase 评估）
4. **裁剪依赖**：opentelemetry-otlp 的 gRPC/tonic 体积大，若生产不启用 OTLP（当前 `OTEL_ENABLED=false`），可考虑在生产构建中条件编译排除。
5. **移除 tzdata**：若日志时间戳使用 UTC，可省去 ~3MB。

---

## 3. 关键设计决策与偏离说明

### 3.1 探针路径：使用 `/metrics` 而非 `/health`

- **Go 版**：三探针使用 `GET /health`。
- **Rust 版现状**：根路径仅有 `/metrics`（无 JWT）和 `/api/v1/*`（需 JWT），**无根级 `/health` 端点**。
- **本任务约束**：禁止修改 `src/`、`routes.rs`、`main.rs`、`lib.rs`。
- **决策**：三探针（startupProbe/livenessProbe/readinessProbe）统一使用 `path: /metrics`（返回 200 当应用运行中且 `PROMETHEUS_ENABLED=true`）。
- **后续建议**：在 Rust 端新增 `GET /health` 端点（返回 `{"status":"ok"}`），然后将探针路径切回 `/health`。这属于代码变更，不在 P3-06 范围。

### 3.2 端口：容器内 8000，宿主映射 8001

- Rust `.env.example` 默认 `SERVER_PORT=8001`，但 `config.rs` 默认值为 8000。
- 通过 `SERVER_PORT=8000` 环境变量覆盖，对齐 Go 版和 K8s Service。
- docker-compose 宿主映射 `8001:8000`，避免与 Go 版 8000 端口冲突。

### 3.3 非 root 用户

- UID/GID = 10001（对齐任务要求和 Go 版常见配置）。
- alpine 使用 `adduser -u 10001`。
- K8s Deployment: `runAsNonRoot: true, runAsUser: 10001, runAsGroup: 10001, fsGroup: 10001`。

### 3.4 Secret fail-secure 设计

- `02-secret.yaml` 的 `data` 字段为 base64 占位值，非真实密钥。
- 部署前必须通过 `kubectl create secret generic` 或 External Secrets 注入真实值。
- 直接 apply 空/占位 Secret 会导致应用因读不到 JWT_SECRET 而拒绝启动（fail-secure）。

### 3.5 16 条告警规则

对齐 Go 版告警体系，覆盖以下类别：

| # | 告警名 | 类别 | 级别 |
|---|--------|------|------|
| 1 | GPUHighUtilization | GPU | warning |
| 2 | CPUHighUtilization | CPU | warning |
| 3 | MemoryHighUtilization | 内存 | warning |
| 4 | StorageLow | 磁盘 | critical |
| 5 | JobQueueLength | Job | warning |
| 6 | GPUUnavailable | GPU | critical |
| 7 | PodCrashLooping | 集群 | warning |
| 8 | HighErrorRate | API | critical |
| 9 | HighLatency | API | warning |
| 10 | RequestRateDrop | API | warning |
| 11 | DatabaseConnectionPoolExhausted | 数据库 | critical |
| 12 | RedisConnectionDown | 缓存 | critical |
| 13 | CertificateExpiringSoon | 安全 | warning |
| 14 | FailedLoginSpike | 安全 | warning |
| 15 | ResourceQuotaExceeded | 配额 | warning |
| 16 | PodNotReady | 集群 | warning |

---

## 4. 验收标准对照

| # | 验收标准 | 状态 |
|---|----------|------|
| 1 | Dockerfile 多阶段构建（builder + runtime，非 root） | ✅ |
| 2 | 镜像大小静态评估 ≤40MB（或记录优化建议） | ✅ 估算 31-41MB，附优化建议 |
| 3 | .dockerignore 排除构建无关文件 | ✅ |
| 4 | docker-compose.yml 可启动 backend + postgres + redis | ✅ 待 CI 验证 |
| 5 | K8s 清单 ≥13 个文件 | ✅ 14 个文件 |
| 6 | Deployment 三探针 + 非 root + 资源限制 + 拓扑分布 | ✅ |
| 7 | K8s YAML 结构校验脚本可运行并输出报告 | ✅ 63 项全部 PASS |
| 8 | PrometheusRule 包含 16 告警规则 | ✅ |
| 9 | ServiceMonitor 抓取 /metrics 端点 | ✅ |
| 10 | 所有 YAML 文件语法正确 | ✅ 结构校验通过（语法待 kubectl dry-run） |

---

## 5. 待 CI/目标环境验证项

本机无 Docker 和 kubectl，以下项目需在 CI 或目标环境执行：

```bash
# 1. 镜像构建与大小验证
docker build -t ghcr.io/customizedvalidation/yunchuang/backend-rust:latest .
docker images | grep backend-rust    # 确认 <= 40MB

# 2. docker-compose 语法验证
docker compose config

# 3. K8s manifest dry-run 验证
kubectl apply --dry-run=client -k k8s/

# 4. 启动验证
docker compose up -d
curl http://localhost:8001/metrics    # 应返回 Prometheus 文本格式
```

---

## 6. 与 P3-05 的边界

本任务（P3-06）**未修改**以下 P3-05 范围文件：
- `src/openapi/`
- `tests/p3_openapi_test.rs`
- `docs/openapi-rust.json`

本任务为纯配置交付，未修改任何 Rust 源码（`src/`、`Cargo.toml`、`main.rs`、`lib.rs`、`routes.rs`）。
