# Metaclouds 生产部署 Runbook

> **版本**：v1.2（2026-09-16）
> **适用环境**：Kubernetes 生产集群
> **部署方式**：Kustomize（`kubectl apply -k`）
> **关联文档**：[生产落地复盘（2026-09-03）](./production-review-2026-09-03.md)、[优化报告](./optimization-report.md)、[Vue 迁移说明](./vue-migration-guide.md)

> **前端技术栈变更（2026-09-16）**：前端已由 React 版 `metaclouds-frontend/` 全量迁移至 Vue 3 版 `metaclouds-frontend-vue/`。
> 本文中出现的 `deployment/metaclouds-frontend`、`svc/metaclouds-frontend`、容器名 `metaclouds-frontend` 均为**运行时 K8s 资源名**，保持不变；
> 但镜像的**构建上下文 / Dockerfile 路径已切换为 `metaclouds-frontend-vue/`**（CI 见 `.github/workflows/ci-cd.yml`，Dockerfile 与 nginx.conf 位于 `metaclouds-frontend-vue/` 下）。
> 旧 React 目录 `metaclouds-frontend/` 已标记 DEPRECATED，仅保留 git 历史参考，不再参与构建与部署。

---

## 目录

1. [前置条件](#1-前置条件)
2. [命名空间与 Secret 创建](#2-命名空间与-secret-创建)
3. [数据库初始化与迁移](#3-数据库初始化与迁移)
4. [部署清单](#4-部署清单)
5. [健康验证](#5-健康验证)
6. [滚动更新流程](#6-滚动更新流程)
7. [回滚流程](#7-回滚流程)
8. [常见故障排查](#8-常见故障排查)
9. [日志查看](#9-日志查看)
10. [监控接入](#10-监控接入)
11. [备份与恢复](#11-备份与恢复)
12. [高级功能配置](#12-高级功能配置2026-09-09-新增)
    - [12.1 多租户资源配额](#121-多租户资源配额resourcequota--limitrange)
    - [12.2 多 GPU 厂商节点池](#122-多-gpu-厂商节点池配置)
    - [12.3 网络拓扑与 RDMA](#123-网络拓扑与-rdma-配置)
    - [12.4 Fluid 数据加速](#124-fluid-数据加速集成)
    - [12.5 Slurm/LSF/SGE 集成](#125-slurm--lsf--sge-集成配置)
    - [12.6 数据库迁移 000004](#126-数据库迁移-000004-执行说明)

---

## 1. 前置条件

### 1.1 集群与工具

| 组件 | 最低版本 | 验证命令 |
|---|---|---|
| Kubernetes | 1.24+ | `kubectl version --short` |
| kubectl | 与集群 ±1 版本 | `kubectl version --client` |
| Helm | 3.10+ | `helm version` |
| cert-manager | 1.12+ | `kubectl get pods -n cert-manager` |
| ingress-nginx | 1.8+ | `kubectl get pods -n ingress-nginx` |
| metrics-server | 0.6+ | `kubectl get pods -n kube-system \| grep metrics-server` |

### 1.2 中间件依赖

以下中间件需提前部署并可从 `metaclouds` 命名空间访问：

| 中间件 | 版本 | 预期 Service DNS | 端口 |
|---|---|---|---|
| PostgreSQL | 16 | `postgresql.metaclouds.svc.cluster.local` | 5432 |
| Redis | 7 | `redis.metaclouds.svc.cluster.local` | 6379 |
| etcd | 3.5+ | `etcd.metaclouds.svc.cluster.local` | 2379 |
| Prometheus | 2.45+ | `prometheus.monitoring.svc.cluster.local` | 9090 |
| Grafana | 10+ | `grafana.monitoring.svc.cluster.local` | 3000 |
| Jaeger | 1.45+ | `jaeger-collector.monitoring.svc.cluster.local` | 14268 |

> 若中间件部署在不同命名空间或使用不同 Service 名称，需同步修改 `01-configmap.yaml` 中的对应地址。

### 1.3 镜像仓库

镜像托管在 GHCR（GitHub Container Registry）：

- 后端：`ghcr.io/customizedvalidation/yunchuang/backend:<tag>`
- 前端：`ghcr.io/customizedvalidation/yunchuang/frontend:<tag>`

若仓库为私有，需创建 imagePullSecret：

```bash
kubectl -n metaclouds create secret docker-registry ghcr-pull-secret \
  --docker-server=ghcr.io \
  --docker-username=<github-username> \
  --docker-password=<github-pat> \
  --docker-email=<email>

# 然后在 03-backend-deployment.yaml / 05-frontend-deployment.yaml 中取消注释：
# imagePullSecrets:
#   - name: ghcr-pull-secret
```

---

## 2. 命名空间与 Secret 创建

### 2.1 创建命名空间

```bash
kubectl apply -f metaclouds-backend/deploy/kubernetes/00-namespace.yaml
# 验证
kubectl get namespace metaclouds
```

### 2.2 生成强密码并创建 Secret

> **安全警告**：`02-secrets.yaml` 是 fail-secure 模板（`data: {}`），直接 apply 会得到空 Secret，后端因读不到密钥而拒绝启动。必须先创建真实 Secret。

```bash
# 生成所有密钥并创建 Secret（推荐方式）
kubectl -n metaclouds create secret generic metaclouds-secrets \
  --from-literal=jwt-secret="$(openssl rand -base64 48)" \
  --from-literal=database-password="$(openssl rand -base64 24)" \
  --from-literal=redis-password="$(openssl rand -base64 24)" \
  --from-literal=default-admin-password="$(openssl rand -base64 18)" \
  --from-literal=csrf-secret="$(openssl rand -base64 32)"
```

> **Windows PowerShell 替代命令**（无 openssl 时）：
> ```powershell
> $jwt = -join ((1..64) | ForEach-Object { [char[]](65..90 + 97..122 + 48..57) | Get-Random })
> kubectl -n metaclouds create secret generic metaclouds-secrets `
>   --from-literal=jwt-secret="$jwt" `
>   --from-literal=database-password="$(-join ((1..32) | ForEach-Object { [char[]](65..90+97..122+48..57) | Get-Random }))" `
>   --from-literal=redis-password="$(-join ((1..32) | ForEach-Object { [char[]](65..90+97..122+48..57) | Get-Random }))" `
>   --from-literal=default-admin-password="$(-join ((1..24) | ForEach-Object { [char[]](65..90+97..122+48..57) | Get-Random }))" `
>   --from-literal=csrf-secret="$(-join ((1..44) | ForEach-Object { [char[]](65..90+97..122+48..57) | Get-Random }))"
> ```

### 2.3 验证 Secret

```bash
# 确认 Secret 存在且包含所有键
kubectl -n metaclouds get secret metaclouds-secrets -o jsonpath='{.data}' | jq keys
# 预期输出：["csrf-secret", "database-password", "default-admin-password", "jwt-secret", "redis-password"]

# 确认 jwt-secret 长度 ≥ 32 字节（config.Validate 强制）
kubectl -n metaclouds get secret metaclouds-secrets -o jsonpath='{.data.jwt-secret}' | base64 -d | wc -c
# 预期输出 ≥ 48（base64 48 字符 ≈ 36 字节）
```

### 2.4 生产环境推荐：Sealed Secrets / External Secrets

```bash
# 方式 A：Sealed Secrets（Bitnami）
kubeseal --controller-name=sealed-secrets --controller-namespace=kube-system \
  --format yaml < secret.yaml > sealed-secret.yaml

# 方式 B：External Secrets Operator（引用 AWS Secrets Manager / Vault）
# 创建 SecretStore + ExternalSecret，详见 ESO 文档
```

---

## 3. 数据库初始化与迁移

### 3.1 初始化数据库

初始化 SQL 位于仓库根目录 `D:\YCYD\init.sql`，包含建库、建用户、初始 schema。

```bash
# 方式 1：通过 PostgreSQL Pod 执行
kubectl -n metaclouds exec -it <postgresql-pod> -- psql -U postgres -f /tmp/init.sql

# 方式 2：本地 psql 远程执行（需端口转发）
kubectl -n metaclouds port-forward svc/postgresql 5432:5432 &
psql -h localhost -U postgres -f init.sql
```

### 3.2 数据库迁移

迁移文件位于 `metaclouds-backend/deploy/migrations/`：

| 文件 | 说明 |
|---|---|
| `20260528_add_indexes.sql` | PostgreSQL 索引迁移 |
| `20260528_add_indexes_sqlite.sql` | SQLite 索引迁移（开发用，生产忽略） |
| `migration-job.yaml` | K8s Job 形式执行迁移 |
| `run_migration.sh` | 迁移执行脚本 |
| `configmap.yaml` | 迁移脚本 ConfigMap |

```bash
# 通过 K8s Job 执行迁移
kubectl apply -f metaclouds-backend/deploy/migrations/configmap.yaml
kubectl apply -f metaclouds-backend/deploy/migrations/migration-job.yaml

# 查看迁移 Job 状态
kubectl -n metaclouds get jobs -l app=metaclouds
kubectl -n metaclouds logs job/metaclouds-db-migration

# 迁移完成后清理 Job
kubectl -n metaclouds delete job metaclouds-db-migration
```

> **注意**：迁移 Job 使用的 Secret 名称为 `metaclouds-db-secret`（见 `migration-job.yaml`），需确保该 Secret 存在或修改为引用 `metaclouds-secrets`。

---

## 4. 部署清单

### 4.1 一键部署（推荐）

```bash
cd D:\YCYD

# 生产环境部署（使用 overlay）
kubectl apply -k metaclouds-backend/deploy/kubernetes/overlays/production

# 或直接使用基础清单
kubectl apply -k metaclouds-backend/deploy/kubernetes/
```

### 4.2 按顺序手动部署（调试用）

```bash
cd D:\YCYD/metaclouds-backend/deploy/kubernetes

# 1. 命名空间
kubectl apply -f 00-namespace.yaml

# 2. 配置（ConfigMap）
kubectl apply -f 01-configmap.yaml

# 3. Secret（模板，真实值已在第 2 节创建）
kubectl apply -f 02-secrets.yaml

# 4. 服务账户与 RBAC
kubectl apply -f 10-serviceaccount.yaml

# 5. 后端
kubectl apply -f 03-backend-deployment.yaml
kubectl apply -f 04-backend-service.yaml

# 6. 前端
kubectl apply -f 05-frontend-deployment.yaml
kubectl apply -f 06-frontend-service.yaml

# 7. Ingress
kubectl apply -f 07-ingress.yaml

# 8. HPA
kubectl apply -f 08-hpa.yaml

# 9. NetworkPolicy
kubectl apply -f 09-networkpolicy.yaml

# 10. PDB
kubectl apply -f 11-pdb.yaml

# 11. 多租户资源配额（新增，对应 skill.md §4.2）
kubectl apply -f 12-resourcequota.yaml
kubectl apply -f 13-limitrange.yaml

# 12. 多 GPU 厂商节点池配置（新增，对应 skill.md §4.1）
kubectl apply -f 14-gpu-node-pools.yaml

# 13. 网络拓扑与 RDMA 配置（新增，对应 skill.md §4.1/§4.3）
kubectl apply -f 15-network-topology.yaml

# 14. Fluid 数据加速集成（新增，对应 skill.md §4.4，需先安装 Fluid）
kubectl apply -f 16-fluid-integration.yaml

# 15. 作业模板与调度策略（新增，对应 skill.md §4.3/§4.4）
kubectl apply -f 17-job-templates.yaml

# 16. 备份 CronJob（保留原有文件）
kubectl apply -f backup-cronjob.yaml
```

### 4.3 部署后等待

```bash
# 等待所有 Pod 就绪
kubectl -n metaclouds wait --for=condition=Ready pods --all --timeout=300s

# 等待后端滚动更新完成
kubectl -n metaclouds rollout status deployment/metaclouds-backend --timeout=300s

# 等待前端滚动更新完成
kubectl -n metaclouds rollout status deployment/metaclouds-frontend --timeout=120s
```

---

## 5. 健康验证

### 5.1 Pod 状态

```bash
# 查看所有 Pod 状态
kubectl -n metaclouds get pods -o wide

# 预期：backend 3/3 Running，frontend 2/2 Running，无 Restart
# 若有 CrashLoopBackOff / ImagePullBackOff，见第 8 节

# 查看 Deployment 状态
kubectl -n metaclouds get deployments
kubectl -n metaclouds describe deployment metaclouds-backend
```

### 5.2 Service 与 Endpoints

```bash
# 查看 Service
kubectl -n metaclouds get svc

# 确认 Endpoints 已关联到 Pod（无 endpoints 意味着 selector 不匹配或 Pod 未就绪）
kubectl -n metaclouds get endpoints metaclouds-backend
kubectl -n metaclouds get endpoints metaclouds-frontend
```

### 5.3 健康检查端点

```bash
# 方式 1：端口转发后 curl
kubectl -n metaclouds port-forward svc/metaclouds-backend 8080:8080 &
curl -f http://localhost:8080/health
# 预期 200：{"status":"healthy","dependencies":1}

curl -f http://localhost:8080/metrics | head -20
# 预期：Prometheus 格式指标

# 方式 2：通过临时 Pod 在集群内测试
kubectl -n metaclouds run curl-test --image=curlimages/curl --rm -it -- \
  curl -f http://metaclouds-backend.metaclouds.svc.cluster.local:8080/health

# 前端健康检查
kubectl -n metaclouds port-forward svc/metaclouds-frontend 8080:80 &
curl -f http://localhost:8080/health
# 预期 200：ok
curl -f -I http://localhost:8080/
# 预期 200，Content-Type: text/html
```

### 5.4 Ingress 与 TLS

```bash
# 查看 Ingress
kubectl -n metaclouds get ingress
kubectl -n metaclouds describe ingress metaclouds-ingress

# 确认 cert-manager 已签发证书
kubectl -n metaclouds get certificate
kubectl -n metaclouds describe certificate metaclouds-tls

# 通过域名测试（需 DNS 已解析到 ingress-nginx LoadBalancer）
curl -fI https://app.metaclouds.example.com/health
curl -fI https://app.metaclouds.example.com/api/v1/health
```

### 5.5 HPA 状态

```bash
kubectl -n metaclouds get hpa
# 确认 TARGETS 列显示 CPU/内存使用率（若为 <unknown>，说明 metrics-server 未正常工作）
```

---

## 6. 滚动更新流程

### 6.1 更新镜像

```bash
# 更新后端镜像（使用 git commit SHA 作为 tag，可追溯）
kubectl -n metaclouds set image deployment/metaclouds-backend \
  metaclouds-backend=ghcr.io/customizedvalidation/yunchuang/backend:${GIT_SHA}

# 更新前端镜像
kubectl -n metaclouds set image deployment/metaclouds-frontend \
  metaclouds-frontend=ghcr.io/customizedvalidation/yunchuang/frontend:${GIT_SHA}

# 记录变更原因（可选，便于审计）
kubectl -n metaclouds annotate deployment/metaclouds-backend \
  kubernetes.io/change-cause="Deploy ${GIT_SHA}: $(git log -1 --pretty=%s)"
```

### 6.2 监控滚动更新

```bash
# 实时监控滚动更新状态
kubectl -n metaclouds rollout status deployment/metaclouds-backend --timeout=300s
kubectl -n metaclouds rollout status deployment/metaclouds-frontend --timeout=120s

# 查看滚动更新历史
kubectl -n metaclouds rollout history deployment/metaclouds-backend

# 查看新版本 Pod 启动情况
kubectl -n metaclouds get pods -l component=backend -w
```

### 6.3 配置变更触发滚动更新

修改 ConfigMap 后，需手动触发滚动更新（K8s 不会自动检测 ConfigMap 变更）：

```bash
# 方式 1：更新 annotation 触发滚动更新
kubectl -n metaclouds patch deployment metaclouds-backend -p \
  "{\"spec\":{\"template\":{\"metadata\":{\"annotations\":{\"metaclouds.io/config-checksum\":\"$(date +%s)\"}}}}}"

# 方式 2：使用 kubectl rollout restart
kubectl -n metaclouds rollout restart deployment/metaclouds-backend
kubectl -n metaclouds rollout restart deployment/metaclouds-frontend
```

---

## 7. 回滚流程

### 7.1 回滚到上一版本

```bash
# 查看滚动更新历史，确认要回滚到的 revision
kubectl -n metaclouds rollout history deployment/metaclouds-backend

# 回滚到上一版本
kubectl -n metaclouds rollout undo deployment/metaclouds-backend

# 回滚到指定版本
kubectl -n metaclouds rollout undo deployment/metaclouds-backend --to-revision=2

# 等待回滚完成
kubectl -n metaclouds rollout status deployment/metaclouds-backend --timeout=300s
```

### 7.2 前端回滚

```bash
kubectl -n metaclouds rollout undo deployment/metaclouds-frontend
kubectl -n metaclouds rollout status deployment/metaclouds-frontend --timeout=120s
```

### 7.3 紧急回滚（服务完全不可用）

```bash
# 1. 立即回滚
kubectl -n metaclouds rollout undo deployment/metaclouds-backend
kubectl -n metaclouds rollout undo deployment/metaclouds-frontend

# 2. 若回滚后仍不可用，缩容到 0 并排查
kubectl -n metaclouds scale deployment metaclouds-backend --replicas=0

# 3. 排查完成后恢复
kubectl -n metaclouds scale deployment metaclouds-backend --replicas=3
```

---

## 8. 常见故障排查

### 8.1 CrashLoopBackOff

**症状**：Pod 反复重启，RESTARTS 持续增长。

**排查步骤**：

```bash
# 1. 查看 Pod 日志
kubectl -n metaclouds logs <pod-name> --tail=100

# 2. 查看上一次崩溃的日志
kubectl -n metaclouds logs <pod-name> --previous

# 3. 查看 Pod 事件
kubectl -n metaclouds describe pod <pod-name>

# 4. 常见原因：
#    - JWT_SECRET 缺失或 < 32 字节 → config.Validate 拒绝启动
#    - DATABASE_PASSWORD 错误 → DB 连接失败 → /health 失败 → startupProbe 超时
#    - ALLOWED_ORIGINS 缺失或含 '*' → 生产校验失败
#    - DATABASE_SSL_MODE=disable → 生产校验失败
#    - USE_SQLITE=true → 生产校验失败
```

### 8.2 ImagePullBackOff / ErrImagePull

**症状**：Pod 无法拉取镜像。

**排查步骤**：

```bash
# 1. 查看事件
kubectl -n metaclouds describe pod <pod-name> | grep -A5 Events

# 2. 常见原因：
#    - 镜像 tag 不存在（拼写错误 / CI 构建失败）
#    - 私有仓库未配置 imagePullSecret
#    - 网络策略阻止出站到镜像仓库

# 3. 验证镜像可拉取
kubectl -n metaclouds run image-test --image=ghcr.io/customizedvalidation/yunchuang/backend:latest --rm -it -- echo "pull ok"
```

### 8.3 OOMKilled

**症状**：Pod 被 OOM Killer 终止，退出码 137。

**排查步骤**：

```bash
# 1. 确认 OOM
kubectl -n metaclouds describe pod <pod-name> | grep -i "last state" -A5
# 预期：Reason: OOMKilled

# 2. 查看内存使用趋势
kubectl -n metaclouds top pods -l component=backend

# 3. 解决方案：
#    - 临时：提高 limits.memory（kubectl edit deployment）
#    - 长期：分析内存泄漏（pprof），优化代码
#    - 检查是否有大文件上传 / 大数据查询导致内存峰值
```

### 8.4 502 Bad Gateway / 503 Service Unavailable

**症状**：通过 Ingress 访问返回 502/503。

**排查步骤**：

```bash
# 1. 确认 Pod 就绪（readinessProbe 通过）
kubectl -n metaclouds get pods -o wide
# READY 列应为 1/1 或 3/3

# 2. 确认 Service Endpoints
kubectl -n metaclouds get endpoints metaclouds-backend
# 若 ENDPOINTS 为空，说明 Pod 未通过 readinessProbe

# 3. 直接访问 Pod（绕过 Service）
kubectl -n metaclouds port-forward <pod-name> 8080:8080
curl -f http://localhost:8080/health

# 4. 查看 ingress-nginx 日志
kubectl -n ingress-nginx logs <ingress-nginx-controller-pod> --tail=50 | grep metaclouds

# 5. 常见原因：
#    - readinessProbe 失败（DB 连接超时 / 端口错误）
#    - NetworkPolicy 阻止 ingress-nginx → backend
#    - 后端容器端口与 Service targetPort 不匹配
```

### 8.5 数据库连接失败

**症状**：后端日志显示 `connection refused` / `timeout` / `password authentication failed`。

**排查步骤**：

```bash
# 1. 确认 PostgreSQL 运行
kubectl -n metaclouds get pods -l app=postgresql
kubectl -n metaclouds get svc postgresql

# 2. 从后端 Pod 测试 DB 连通性
kubectl -n metaclouds exec -it <backend-pod> -- sh -c \
  "nc -zv postgresql.metaclouds.svc.cluster.local 5432"

# 3. 验证凭据（使用 Secret 中的密码）
PGPASSWORD=$(kubectl -n metaclouds get secret metaclouds-secrets -o jsonpath='{.data.database-password}' | base64 -d) \
  kubectl -n metaclouds exec -it <postgresql-pod> -- \
  psql -h localhost -U metaclouds_user -d metaclouds -c "SELECT 1"

# 4. 常见原因：
#    - database-password 与 PostgreSQL 中设置的密码不一致
#    - NetworkPolicy 阻止 backend → postgresql:5432
#    - PostgreSQL 未就绪 / 正在恢复
#    - DATABASE_SSL_MODE=require 但 PostgreSQL 未配置 SSL
```

### 8.6 HPA 显示 `<unknown>` / `<missing>`

**症状**：`kubectl get hpa` 的 TARGETS 列显示 `<unknown>`。

**排查步骤**：

```bash
# 1. 确认 metrics-server 运行
kubectl -n kube-system get pods -l k8s-app=metrics-server

# 2. 测试 metrics API
kubectl get --raw "/apis/metrics.k8s.io/v1beta1/namespaces/metaclouds/pods" | jq .

# 3. 确认 Pod 有 resources.requests（HPA 计算利用率需要 requests 作为基数）
kubectl -n metaclouds get pod <pod-name> -o jsonpath='{.spec.containers[0].resources}'
```

---

## 9. 日志查看

### 9.1 后端日志

```bash
# 查看所有后端 Pod 日志（按标签选择）
kubectl -n metaclouds logs -l app=metaclouds-backend --tail=100

# 查看单个 Pod 日志
kubectl -n metaclouds logs <pod-name> --tail=200

# 实时跟踪日志
kubectl -n metaclouds logs -f <pod-name>

# 查看上一次崩溃的日志
kubectl -n metaclouds logs <pod-name> --previous

# 过滤错误日志（JSON 格式日志）
kubectl -n metaclouds logs -l component=backend --tail=500 | grep '"level":"error"'

# 查看特定时间范围的日志
kubectl -n metaclouds logs <pod-name> --since=1h --tail=1000
```

### 9.2 前端日志

```bash
kubectl -n metaclouds logs -l component=frontend --tail=100
# nginx 日志格式：access.log + error.log（输出到 stdout/stderr）
```

### 9.3 日志聚合（推荐）

生产环境建议部署日志聚合系统：

| 方案 | 部署方式 |
|---|---|
| EFK（Elasticsearch + Fluentd + Kibana） | Helm chart |
| Loki + Promtail | Grafana Labs |
| Datadog / Splunk（商用） | Agent DaemonSet |

---

## 10. 监控接入

### 10.1 Prometheus Targets 验证

```bash
# 方式 1：通过 Prometheus UI
kubectl -n monitoring port-forward svc/prometheus 9090:9090
# 浏览器打开 http://localhost:9090/targets
# 确认 metaclouds-backend target 状态为 UP

# 方式 2：通过 Prometheus API
curl -s http://localhost:9090/api/v1/targets | jq '.data.activeTargets[] | select(.labels.job=="metaclouds-backend")'

# 方式 3：查询指标
curl -s 'http://localhost:9090/api/v1/query?query=up{job="metaclouds-backend"}' | jq .
```

### 10.2 关键指标查询

```promql
# 后端实例数
up{job="metaclouds-backend"}

# HTTP 请求速率（5 分钟速率）
rate(http_requests_total{job="metaclouds-backend"}[5m])

# HTTP 错误率（5xx）
rate(http_requests_total{job="metaclouds-backend", status=~"5.."}[5m])

# P99 延迟
histogram_quantile(0.99, rate(http_request_duration_seconds_bucket{job="metaclouds-backend"}[5m]))

# CPU 使用率
rate(container_cpu_usage_seconds_total{namespace="metaclouds", container="metaclouds-backend"}[5m])

# 内存使用率
container_memory_working_set_bytes{namespace="metaclouds", container="metaclouds-backend"}
```

### 10.3 Grafana Dashboard Import

1. 打开 Grafana → Dashboards → Import
2. 输入 Dashboard ID 或上传 JSON
3. 推荐 Dashboard：
   - **Go Process Runtime**：ID 6671（Go 运行时指标：goroutine、GC、内存）
   - **Kubernetes / Compute Resources / Workload**：ID 14394（Pod CPU/内存）
   - **Nginx Ingress Controller**：ID 9614（Ingress 流量）
4. 选择 Prometheus 数据源为 `Prometheus`
5. 设置变量 `namespace=metaclouds`

### 10.4 告警规则

建议配置以下告警（PrometheusRule）：

| 告警 | 表达式 | 严重度 |
|---|---|---|
| BackendDown | `up{job="metaclouds-backend"} == 0` 持续 1m | critical |
| HighErrorRate | `rate(http_requests_total{status=~"5.."}[5m]) / rate(http_requests_total[5m]) > 0.05` | warning |
| HighLatencyP99 | `histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 2` | warning |
| PodCrashLooping | `rate(kube_pod_container_status_restarts_total{namespace="metaclouds"}[15m]) > 0` | critical |
| HPAAtMax | `kube_horizontalpodautoscaler_status_current_replicas{namespace="metaclouds"} == kube_horizontalpodautoscaler_spec_max_replicas{namespace="metaclouds"}` | warning |

---

## 11. 备份与恢复

### 11.1 自动备份

备份 CronJob 已在 `backup-cronjob.yaml` 中定义，包含三个计划：

| 作业 | 调度 | 保留 | 压缩 |
|---|---|---|---|
| `metaclouds-backup-hourly` | 每小时整点 | 24 小时 | gzip -6 |
| `metaclouds-backup-daily` | 每天 02:00 | 30 天 | gzip -9 + sha256 |
| `metaclouds-backup-weekly` | 每周日 03:00 | 4 周 | gzip -9 + sha256 + metadata |

备份存储在 PVC `metaclouds-backup-pvc`（50Gi），挂载路径 `/backups`。

```bash
# 查看备份 CronJob 状态
kubectl -n metaclouds get cronjobs -l component=backup

# 手动触发一次备份
kubectl -n metaclouds create job --from=cronjob/metaclouds-backup-daily manual-backup-$(date +%Y%m%d%H%M%S)

# 查看备份文件
kubectl -n metaclouds exec -it <backup-pod> -- ls -lh /backups/
```

### 11.2 备份验证

```bash
# 验证备份文件完整性（sha256）
kubectl -n metaclouds exec -it <postgresql-pod> -- sh -c \
  "cd /backups && sha256sum -c metaclouds_daily_*.dump.gz.sha256"

# 验证备份可恢复（dry-run restore 到临时数据库）
pg_restore --list /backups/metaclouds_daily_<timestamp>.dump.gz | head -20
```

### 11.3 恢复流程

> **警告**：恢复操作会覆盖现有数据！执行前务必先创建当前数据库的备份。

```bash
# 1. 停止后端（避免恢复期间有写入）
kubectl -n metaclouds scale deployment metaclouds-backend --replicas=0

# 2. 终止所有活跃连接
kubectl -n metaclouds exec -it <postgresql-pod> -- psql -U postgres -c \
  "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='metaclouds' AND pid <> pg_backend_pid();"

# 3. 删除并重建数据库（或使用不同名称做并行恢复验证）
kubectl -n metaclouds exec -it <postgresql-pod> -- psql -U postgres -c \
  "DROP DATABASE IF EXISTS metaclouds; CREATE DATABASE metaclouds OWNER metaclouds_user;"

# 4. 从备份恢复
kubectl -n metaclouds exec -it <postgresql-pod> -- pg_restore \
  -h localhost -U metaclouds_user -d metaclouds -v \
  /backups/metaclouds_daily_<timestamp>.dump.gz

# 5. 验证恢复
kubectl -n metaclouds exec -it <postgresql-pod> -- psql -U metaclouds_user -d metaclouds -c \
  "SELECT count(*) FROM users; SELECT count(*) FROM clusters;"

# 6. 恢复后端
kubectl -n metaclouds scale deployment metaclouds-backend --replicas=3
kubectl -n metaclouds rollout status deployment/metaclouds-backend

# 7. 验证服务
curl -f https://app.metaclouds.example.com/api/v1/health
```

### 11.4 异地备份（推荐）

生产环境建议将备份同步到异地存储（S3 / OSS / GCS）：

```bash
# 在 backup-cronjob.yaml 的 daily/weekly 作业中已配置 S3 上传（需取消注释并填写）
# 环境变量：
#   S3_BUCKET: "your-s3-bucket-name"
#   S3_PREFIX: "metaclouds/backups"

# 或使用 restic / rclone 做增量加密备份
# restic -r s3:s3.amazonaws.com/your-bucket/metaclouds backup /backups
```

---

## 12. 高级功能配置（2026-09-09 新增）

### 12.1 多租户资源配额（ResourceQuota / LimitRange）

对应 `12-resourcequota.yaml` 和 `13-limitrange.yaml`，为三个团队 Namespace 设置资源配额。

#### 部署前提

三个团队 Namespace 已由 `00-namespace.yaml` 创建：
- `team-infra`：基础服务团队（2 GPU, 4 CPU, 8Gi memory）
- `team-data`：数据分析团队（4 GPU, 8 CPU, 16Gi memory）
- `team-algorithm`：算法训练团队（8 GPU, 16 CPU, 32Gi memory）

#### 部署步骤

```bash
# 部署 ResourceQuota（GPU/CPU/内存/PVC/Pod 总量限制）
kubectl apply -f 12-resourcequota.yaml

# 部署 LimitRange（单 Pod/Container 资源上下限与默认值）
kubectl apply -f 13-limitrange.yaml
```

#### 验证

```bash
# 查看各 Namespace 配额使用情况
kubectl describe resourcequota -n team-infra
kubectl describe resourcequota -n team-data
kubectl describe resourcequota -n team-algorithm

# 查看 LimitRange
kubectl describe limitrange -n team-infra
kubectl describe limitrange -n team-data
kubectl describe limitrange -n team-algorithm
```

#### GPU 细粒度说明

- 1/2 GPU、1/4 GPU 通过 NVIDIA MIG（A100）或 vGPU（T4）实现
- ResourceQuota 的 `requests.nvidia.com/gpu` 按等效整卡计数
- 细粒度分配由 Metaclouds GPU 管理服务换算，K8s 层通过 MIG 资源名（如 `nvidia.com/mig-1g.5gb`）调度
- 显存超发（oversubscription）在 Metaclouds 平台层配置，K8s ResourceQuota 不直接限制显存

#### 调整配额

```bash
# 临时调整（不推荐，应修改 yaml 文件后重新 apply）
kubectl patch resourcequota team-algorithm-quota -n team-algorithm \
  -p '{"spec":{"hard":{"requests.nvidia.com/gpu":"16"}}}'

# 推荐：修改 12-resourcequota.yaml 后重新 apply
kubectl apply -f 12-resourcequota.yaml
```

### 12.2 多 GPU 厂商节点池配置

对应 `14-gpu-node-pools.yaml`，支持 NVIDIA（A100/T4）、燧原、摩尔线程、国产 X 五类 GPU 节点池。

#### 节点标签与污点设置

集群管理员需为 GPU 节点设置标签和污点：

```bash
# NVIDIA A100 节点池
kubectl label nodes <a100-node> gpu-vendor=nvidia gpu-model=A100
kubectl taint nodes <a100-node> nvidia.com/gpu=true:NoSchedule

# NVIDIA T4 节点池（无污点，可同时运行 CPU 任务）
kubectl label nodes <t4-node> gpu-vendor=nvidia gpu-model=T4

# 燧原节点池
kubectl label nodes <enflame-node> gpu-vendor=enflame gpu-model=ENflame-T20
kubectl taint nodes <enflame-node> enflame.com/gpu=true:NoSchedule

# 摩尔线程节点池
kubectl label nodes <mt-node> gpu-vendor=moore_threads gpu-model=MTT-S80
kubectl taint nodes <mt-node> moorethreads.com/gpu=true:NoSchedule

# 国产 X 节点池
kubectl label nodes <dx-node> gpu-vendor=domestic_x
kubectl taint nodes <dx-node> domestic-x.com/gpu=true:NoSchedule
```

#### 部署节点池配置

```bash
# 部署节点池配置 ConfigMap（供 Metaclouds 平台读取元数据）
kubectl apply -f 14-gpu-node-pools.yaml
```

#### 验证

```bash
# 查看节点标签
kubectl get nodes -L gpu-vendor,gpu-model

# 查看节点污点
kubectl describe nodes | grep -A5 Taints

# 测试调度到特定厂商节点
kubectl run test-a100 --image=nvcr.io/nvidia/pytorch:23.10-py3 \
  --overrides='{"spec":{"nodeSelector":{"gpu-vendor":"nvidia","gpu-model":"A100"},"tolerations":[{"key":"nvidia.com/gpu","operator":"Equal","value":"true","effect":"NoSchedule"}]}}' \
  -- nvidia-smi
```

#### 国产 GPU 驱动部署

- **燧原**：需部署燧原驱动和 enflame-device-plugin，资源名 `enflame.com/gpu`
- **摩尔线程**：需部署 MUSA 驱动和设备插件，资源名 `moorethreads.com/gpu`
- **国产 X**：按实际厂商文档部署驱动和设备插件，替换资源名

### 12.3 网络拓扑与 RDMA 配置

对应 `15-network-topology.yaml`，配置 RoCE/InfiniBand 高速网络和三网隔离。

#### 部署前提

- 集群节点配备支持 RoCEv2 或 InfiniBand 的网卡
- 交换机配置 PFC（优先级流量控制）和 ECN（显式拥塞通知）
- Multus CNI 已部署（如需 NetworkAttachmentDefinition）

#### 部署步骤

```bash
# 部署三网隔离 NetworkPolicy（管理网/存储网/计算网）
kubectl apply -f 15-network-topology.yaml
```

#### RDMA 设备插件部署（可选）

```bash
# 部署 Mellanox RDMA shared device plugin
kubectl apply -f https://raw.githubusercontent.com/Mellanox/k8s-rdma-shared-dev-plugin/master/deployments/rdma-shared-dev-plugin.yml

# 配置 RDMA 设备（修改 ConfigMap 中的网卡名）
kubectl edit configmap rdma-devices -n kube-system
```

#### RoCE 网络附件定义（需 Multus）

`15-network-topology.yaml` 中的 NetworkAttachmentDefinition 为注释形式，取消注释前需：

```bash
# 部署 Multus CNI
kubectl apply -f https://raw.githubusercontent.com/k8snetworkplumbingwg/multus-cni/master/deployments/multus-daemonset.yml

# 确认 Multus 运行
kubectl get pods -n kube-system | grep multus
```

#### 节点网络标签

```bash
# 为有 RDMA 网卡的节点打标
kubectl label nodes <rdma-node> network.metaclouds.io/rdma=true
kubectl label nodes <rdma-node> network.metaclouds.io/gpu-direct-rdma=true
kubectl label nodes <rdma-node> network.metaclouds.io/switch-id=switch-01
```

### 12.4 Fluid 数据加速集成

对应 `16-fluid-integration.yaml`，配置 Fluid 分布式缓存系统。

#### 部署前提

- Fluid 已安装（最低版本 0.9.0+）
- 数据源可访问（CephFS / NFS / S3 / GlusterFS）
- GPU 计算节点有足够的内存和磁盘空间用于缓存

#### Fluid 安装

```bash
# 添加 Fluid Helm 仓库
helm repo add fluid https://fluid-cloudnative.github.io/charts
helm repo update

# 安装 Fluid
helm install fluid fluid/fluid --namespace fluid-system --create-namespace

# 验证安装
kubectl get pods -n fluid-system
kubectl get crd | grep fluid
# 预期：datasets.data.fluid.io, alluxioruntimes.data.fluid.io, dataloads.data.fluid.io
```

#### 部署 Fluid 配置

```bash
# 部署 Dataset + AlluxioRuntime + DataLoad 示例
kubectl apply -f 16-fluid-integration.yaml
```

#### 验证

```bash
# 查看 Dataset 状态
kubectl get dataset -n team-algorithm
kubectl describe dataset metaclouds-training-data-ceph -n team-algorithm

# 查看 AlluxioRuntime（缓存集群）
kubectl get alluxioruntime -n team-algorithm
kubectl get pods -n team-algorithm -l role=alluxio-worker

# 查看数据预取状态
kubectl get dataload -n team-algorithm
kubectl describe dataload metaclouds-training-data-prefetch -n team-algorithm

# 验证缓存 PVC 已创建
kubectl get pvc metaclouds-training-data-ceph -n team-algorithm
```

#### 缓存配置说明

- **缓存介质**：内存（MEM）+ 磁盘（HDD）分层，热数据自动留在内存
- **副本数**：默认 3，多机训练建议与 GPU 节点数一致
- **压缩**：默认 GZIP，训练数据已压缩时可关闭以减少 CPU 开销
- **元数据加速**：RocksDB 存储元数据，支持百万级文件
- **数据预取**：训练前创建 DataLoad CR 预热数据，避免冷启动 I/O 等待

### 12.5 Slurm / LSF / SGE 集成配置

对应 `17-job-templates.yaml` 中的 Slurm 集成模板，通过 SlurmAdapter 将 Metaclouds 作业提交到外部调度器。

#### 部署前提

- Slurm 集群 23.02+（或 LSF 10.2+ / SGE 8.1.9+）
- Metaclouds 后端可通过 SSH 访问 Slurm 控制节点
- Slurm 控制节点已创建 metaclouds 用户和 SSH 密钥

#### 配置步骤

1. **创建 SSH 密钥 Secret**：

```bash
kubectl create secret generic slurm-ssh-key -n metaclouds \
  --from-file=private-key=/path/to/slurm-ssh-key \
  --from-literal=username=metaclouds
```

2. **通过 API 注册调度器集成**：

```bash
curl -X POST https://api.metaclouds.com/api/v1/schedulers \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d '{
    "type": "slurm",
    "name": "slurm-cluster-01",
    "endpoint": "slurmctl.example.com",
    "auth_config": {
      "type": "ssh",
      "username": "metaclouds",
      "private_key_secret": "slurm-ssh-key"
    },
    "version": "23.02",
    "partitions": [
      {"name": "gpu-a100", "gpu_model": "A100", "max_nodes": 32, "max_gpus_per_node": 8}
    ]
  }'
```

3. **验证连通性**：

```bash
curl https://api.metaclouds.com/api/v1/schedulers/1/health \
  -H "Authorization: Bearer <token>"
```

4. **同步队列和节点信息**：

```bash
curl -X POST https://api.metaclouds.com/api/v1/schedulers/1/sync \
  -H "Authorization: Bearer <token>"
```

#### 作业提交流程

1. 用户在 Metaclouds 平台提交作业，选择调度器类型为 `slurm`
2. SlurmAdapter 生成 sbatch 脚本（含 NCCL、DeepSpeed 配置）
3. 通过 SSH 提交到 Slurm 集群，获取 Slurm Job ID
4. 定期同步作业状态（squeue/sacct）
5. 作业完成后同步结果和日志

### 12.6 数据库迁移 000004 执行说明

本次新增数据模型需要执行数据库迁移 `000004_gpu_fine_grained.up.sql`。

#### 迁移内容

- 新增 `gpu_devices` 表（GPU 设备信息）
- 新增 `gpu_allocations` 表（GPU 细粒度分配记录）
- 新增 `partitions` 表（分区管理）
- 新增 `partition_permissions` 表（分区权限）
- 新增 `partition_resource_shares` 表（分区资源共享）
- 新增 `resource_quotas` 表（多维度配额）
- 新增 `scheduler_integrations` 表（调度器集成）
- 新增 `vendor_driver_configs` 表（厂商驱动配置）
- 新增 `node_topologies` 表（节点拓扑）
- 新增 `scheduling_profiles` 表（调度策略）
- 新增 `job_elastic_events` 表（弹性训练事件）
- 新增 `checkpoints` 表（Checkpoint 管理）
- 新增 `datasets` 表（数据集管理）
- 新增 `fluid_caches` 表（Fluid 缓存配置）
- 新增 `distributed_training_configs` 表（分布式训练配置）
- 新增 `inference_configs` 表（推理配置）
- 扩展 `jobs` 表（新增 gpu_fraction、gpu_memory_gb、partition_id 等字段）
- 扩展 `resources` 表（新增 vram_total_mb、vram_used_mb 等字段）
- 扩展 `tenants` 表（新增 gpu_vendor 偏好等字段）

#### 执行步骤

```bash
# 方式 1：通过 K8s Job 执行（推荐）
kubectl apply -f metaclouds-backend/deploy/migrations/configmap.yaml
kubectl apply -f metaclouds-backend/deploy/migrations/migration-job.yaml

# 查看迁移 Job 状态
kubectl -n metaclouds get jobs -l app=metaclouds
kubectl -n metaclouds logs job/metaclouds-db-migration

# 方式 2：手动执行（需端口转发）
kubectl -n metaclouds port-forward svc/postgresql 5432:5432 &
psql -h localhost -U metaclouds_user -d metaclouds \
  -f metaclouds-backend/deploy/migrations/000004_gpu_fine_grained.up.sql
```

#### 验证

```bash
# 确认新表已创建
kubectl -n metaclouds exec -it <postgresql-pod> -- \
  psql -U metaclouds_user -d metaclouds -c "\dt" | grep -E "gpu_|partition|quota|scheduler|topology|checkpoint|dataset|fluid|distributed|inference"

# 确认迁移记录
kubectl -n metaclouds exec -it <postgresql-pod> -- \
  psql -U metaclouds_user -d metaclouds -c "SELECT * FROM schema_migrations ORDER BY version DESC LIMIT 5;"
```

#### 回滚

```bash
# 执行回滚迁移
psql -h localhost -U metaclouds_user -d metaclouds \
  -f metaclouds-backend/deploy/migrations/000004_gpu_fine_grained.down.sql
```

> **注意**：迁移使用 `IF NOT EXISTS`，可安全重复执行。生产环境建议在低峰期执行，并先备份数据库。

---

## 附录 A：快速命令速查

```bash
# 查看所有资源
kubectl -n metaclouds get all

# 查看所有资源（含 configmap / secret / pvc / ingress / hpa / pdb）
kubectl -n metaclouds get all,cm,secret,pvc,ing,hpa,pdb,netpol

# 查看事件（按时间排序）
kubectl -n metaclouds get events --sort-by='.lastTimestamp'

# 查看资源使用
kubectl -n metaclouds top pods
kubectl -n metaclouds top nodes

# 进入后端 Pod 调试
kubectl -n metaclouds exec -it <backend-pod> -- sh

# 查看后端环境变量（确认注入正确）
kubectl -n metaclouds exec <backend-pod> -- env | grep -E "SERVER_|DATABASE_|REDIS_|JWT_|PROMETHEUS_"
```

## 附录 B：生产必配环境变量校验清单

后端 `config.Validate()` 在生产环境强制校验以下项，缺失或不合法将拒绝启动：

| 变量 | 要求 | 配置位置 |
|---|---|---|
| `SERVER_ENV` | `production` | ConfigMap |
| `JWT_SECRET` | ≥ 32 字符 | Secret |
| `USE_SQLITE` | `false` | ConfigMap |
| `MEMORY_STORE_ENABLED` | `false` | ConfigMap |
| `DATABASE_SSL_MODE` | 非 `disable`（如 `require`） | ConfigMap |
| `ALLOWED_ORIGINS` | 非空且不含 `*` | ConfigMap |
| `ALLOW_PUBLIC_REGISTRATION` | `false` | ConfigMap |
| `COOKIE_SAME_SITE=none` | 需 `SERVER_ENV=production` | ConfigMap |

---

*本文档由 Metaclouds 团队维护，最后更新：2026-09-09*
