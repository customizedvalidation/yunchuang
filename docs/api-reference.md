# Metaclouds Backend API 参考文档

## 目录

- [API 概览](#api-概览)
- [认证说明](#认证说明)
- [错误响应格式](#错误响应格式)
- [分页规范](#分页规范)
- [速率限制](#速率限制)
- [API 分组说明](#api-分组说明)
- [OpenAPI 规范](#openapi-规范)

---

## API 概览

| 项目 | 说明 |
|------|------|
| 基础 URL（开发） | `http://localhost:8000/api/v1` |
| 基础 URL（生产） | `https://api.metaclouds.com/api/v1` |
| API 版本 | v1 |
| 数据格式 | JSON（`Content-Type: application/json`） |
| 字符编码 | UTF-8 |
| 时间格式 | ISO 8601 / Unix 时间戳（秒） |

### 非 API 端点

| 端点 | 方法 | 说明 | 认证 |
|------|------|------|------|
| `/health` | GET | 健康检查（依赖探针全部通过才返回 200） | 无需 |
| `/metrics` | GET | Prometheus 指标端点 | 无需 |
| `/api/docs` | GET | API 文档首页（开发环境） | 无需 |
| `/api/docs/swagger.yaml` | GET | OpenAPI 规范文件（开发环境） | 无需 |

---

## 认证说明

Metaclouds 支持两种认证方式，可根据客户端类型选择：

### 1. JWT Bearer（推荐用于非浏览器客户端）

在 HTTP 请求头中携带 JWT 令牌：

```http
Authorization: Bearer <your-jwt-token>
```

- 令牌通过 `POST /api/v1/auth/login` 获取
- 令牌有效期由服务端配置 `JWT_EXPIRATION_HOURS` 决定
- 可通过 `POST /api/v1/auth/refresh` 刷新令牌

### 2. HttpOnly Cookie（浏览器客户端）

登录成功后，服务端自动设置以下 Cookie：

| Cookie 名 | httpOnly | 说明 |
|-----------|----------|------|
| `access_token` | 是 | JWT 令牌，浏览器自动携带 |
| `csrf_token` | 否 | CSRF 双提交令牌，供 JS 读取 |

- Cookie 的 `SameSite` 模式由配置 `COOKIE_SAME_SITE` 决定（默认 Lax）
- 生产环境下 Cookie 标记为 `Secure`，仅通过 HTTPS 传输

### CSRF 防护

使用 Cookie 认证时，所有状态变更请求（`POST` / `PUT` / `DELETE`）必须满足以下条件之一：

1. 在请求头中携带 `X-CSRF-Token: <csrf_token_value>`，该值需与 `csrf_token` Cookie 一致
2. 使用 Bearer 认证（`Authorization` 头），此时跳过 CSRF 校验

跨域部署下，前端 JS 可能无法读取 Cookie，可通过 `GET /api/v1/auth/csrf` 获取令牌。

### 权限模型

部分接口除了需要认证外，还需要特定权限：

| 权限 | 说明 |
|------|------|
| `cluster_write` | 集群的创建/更新/删除 |
| `resource_write` | 资源的更新 |
| `job_write` | 作业的创建/更新/删除/取消 |
| `job_submit` | 作业提交到 Kubernetes |
| `monitoring_write` | 告警的标记解决 |
| `tenant_read` | 租户的读取（仅管理员） |
| `tenant_write` | 租户的创建/更新/删除（仅管理员） |
| `accel_write` | 加速套件的创建/更新/删除 |
| `security_write` | 安全策略的创建/更新/删除 |
| `gpu_read` | GPU 设备和分配的读取（2026-09-09 新增） |
| `gpu_write` | GPU 设备和分配的管理（2026-09-09 新增） |
| `partition_read` | 分区的读取（2026-09-09 新增） |
| `partition_write` | 分区的创建/更新/删除/权限分配（2026-09-09 新增） |
| `quota_read` | 配额的读取（2026-09-09 新增） |
| `quota_write` | 配额的创建/更新/删除（2026-09-09 新增） |
| `scheduler_read` | 调度器集成的读取（2026-09-09 新增） |
| `scheduler_write` | 调度器集成的管理（2026-09-09 新增） |
| `topology_read` | 拓扑信息的读取（2026-09-09 新增） |
| `topology_write` | 拓扑信息的管理（2026-09-09 新增） |
| `dataset_read` | 数据集和缓存的读取（2026-09-09 新增） |
| `dataset_write` | 数据集和缓存的管理（2026-09-09 新增） |
| `checkpoint_read` | Checkpoint 的读取（2026-09-09 新增） |
| `checkpoint_write` | Checkpoint 的创建/删除（2026-09-09 新增） |

---

## 错误响应格式

所有 API 响应遵循统一的包装结构：

### 成功响应

```json
{
  "success": true,
  "data": { ... },
  "message": "操作成功",
  "code": "OK",
  "timestamp": 1757200000
}
```

### 错误响应

```json
{
  "success": false,
  "message": "无效的用户名或密码",
  "code": "UNAUTHORIZED",
  "timestamp": 1757200000
}
```

### HTTP 状态码

| 状态码 | 说明 |
|--------|------|
| 200 | 请求成功 |
| 201 | 资源创建成功 |
| 400 | 请求参数错误 |
| 401 | 未认证或认证失败 |
| 403 | 已认证但无权限 |
| 404 | 资源不存在 |
| 429 | 请求过于频繁（速率限制） |
| 500 | 服务器内部错误 |
| 503 | 服务不可用（健康检查失败） |

---

## 分页规范

列表接口支持分页查询，通过查询参数控制：

| 参数 | 类型 | 默认值 | 最大值 | 说明 |
|------|------|--------|--------|------|
| `page` | integer | 1 | - | 页码，从 1 开始 |
| `page_size` | integer | 10 | 100 | 每页记录数 |

### 分页响应结构

```json
{
  "success": true,
  "data": {
    "data": [ ... ],
    "total": 156,
    "page": 1,
    "page_size": 10,
    "total_pages": 16
  },
  "timestamp": 1757200000
}
```

### 示例

```http
GET /api/v1/jobs?page=2&page_size=20&status=running
```

---

## 速率限制

当配置 `RATE_LIMIT_ENABLED=true` 时，API 启用滑动窗口速率限制：

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| `RATE_LIMIT_REQUESTS` | 时间窗口内最大请求数 | 100 |
| `RATE_LIMIT_DURATION_SECONDS` | 时间窗口（秒） | 60 |

超出限制时返回 `429 Too Many Requests`，并记录 `rate_limit_hits_total` 指标。

---

## API 分组说明

### auth（认证）

| 端点 | 方法 | 说明 | 认证 |
|------|------|------|------|
| `/auth/login` | POST | 用户登录，获取 JWT 令牌 | 无需 |
| `/auth/logout` | POST | 用户登出，清除 Cookie | 无需 |
| `/auth/csrf` | GET | 获取 CSRF 令牌 | 需登录 |
| `/auth/refresh` | POST | 刷新 JWT 令牌 | 需认证 |
| `/auth/profile` | GET | 获取当前用户资料 | 需认证 |
| `/auth/register` | POST | 用户注册（默认关闭） | 无需 |

### cluster（集群管理）

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/clusters` | GET | 获取集群列表 | 认证 |
| `/clusters` | POST | 创建集群 | cluster_write |
| `/clusters/{id}` | GET | 获取集群详情 | 认证 |
| `/clusters/{id}` | PUT | 更新集群 | cluster_write |
| `/clusters/{id}` | DELETE | 删除集群 | cluster_write |
| `/clusters/{id}/status` | GET | 获取集群实时状态 | 认证 |

### resource（资源管理）

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/resources` | GET | 获取资源列表 | 认证 |
| `/resources/{id}` | GET | 获取资源详情 | 认证 |
| `/resources/{id}` | PUT | 更新资源 | resource_write |
| `/resources/gpu` | GET | 获取 GPU 实时资源 | 认证 |

### job（作业管理）

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/jobs` | GET | 获取作业列表 | 认证 |
| `/jobs` | POST | 创建作业 | job_write |
| `/jobs/{id}` | GET | 获取作业详情 | 认证 |
| `/jobs/{id}` | PUT | 更新作业 | job_write |
| `/jobs/{id}` | DELETE | 删除作业 | job_write |
| `/jobs/{id}/cancel` | POST | 取消作业 | job_write |
| `/jobs/{id}/submit` | POST | 提交作业到 K8s | job_submit |
| `/jobs/{id}/status` | GET | 获取作业实时状态 | 认证 |

### monitoring（监控）

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/monitoring/metrics` | GET | 获取监控指标 | 认证 |
| `/monitoring/alerts` | GET | 获取告警列表 | 认证 |
| `/monitoring/alerts/{id}/resolve` | PUT | 标记告警已解决 | monitoring_write |

### tenant（租户管理）

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/tenants` | GET | 获取租户列表 | tenant_read |
| `/tenants` | POST | 创建租户 | tenant_write |
| `/tenants/{id}` | GET | 获取租户详情 | tenant_read |
| `/tenants/{id}` | PUT | 更新租户 | tenant_write |
| `/tenants/{id}` | DELETE | 删除租户 | tenant_write |

### acceleration（加速套件）

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/acceleration` | GET | 获取加速套件列表 | 认证 |
| `/acceleration` | POST | 创建加速套件 | accel_write |
| `/acceleration/{id}` | GET | 获取加速套件详情 | 认证 |
| `/acceleration/{id}` | PUT | 更新加速套件 | accel_write |
| `/acceleration/{id}` | DELETE | 删除加速套件 | accel_write |

### security（安全策略）

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/security/policies` | GET | 获取安全策略列表 | 认证 |
| `/security/policies` | POST | 创建安全策略 | security_write |
| `/security/policies/{id}` | GET | 获取安全策略详情 | 认证 |
| `/security/policies/{id}` | PUT | 更新安全策略 | security_write |
| `/security/policies/{id}` | DELETE | 删除安全策略 | security_write |

### gpu（GPU 细粒度管理）

> 2026-09-09 新增，对应 P0-1 GPU 细粒度分配与显存管理。

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/gpus` | GET | 获取 GPU 设备列表（支持厂商/型号/状态筛选） | gpu_read |
| `/gpus` | POST | 注册 GPU 设备 | gpu_write |
| `/gpus/{id}` | PUT | 更新 GPU 设备信息 | gpu_write |
| `/gpus/{id}` | DELETE | 删除 GPU 设备 | gpu_write |
| `/gpus/allocations` | GET | 获取 GPU 分配记录（支持细粒度 0.25/0.5/1.0） | gpu_read |
| `/gpus/allocations` | POST | 创建 GPU 分配（支持 MIG/vGPU 细粒度） | gpu_write |
| `/gpus/allocations/{id}` | DELETE | 释放 GPU 分配 | gpu_write |
| `/gpus/utilization` | GET | 获取 GPU 利用率统计（时间序列聚合） | gpu_read |

### partition（分区管理）

> 2026-09-09 新增，对应 P0-4 分区管理。

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/partitions` | GET | 获取分区列表 | partition_read |
| `/partitions` | POST | 创建分区 | partition_write |
| `/partitions/{id}` | GET | 获取分区详情 | partition_read |
| `/partitions/{id}` | PUT | 更新分区 | partition_write |
| `/partitions/{id}` | DELETE | 删除分区 | partition_write |
| `/partitions/{id}/priority` | PUT | 调整分区优先级 | partition_write |
| `/partitions/{id}/max-runtime` | PUT | 修改分区最大运行时长 | partition_write |
| `/partitions/{id}/permissions` | GET | 获取分区权限列表 | partition_read |
| `/partitions/{id}/permissions` | POST | 分配分区权限（view/submit/admin） | partition_write |
| `/partitions/{id}/permissions/{permId}` | DELETE | 移除分区权限 | partition_write |

### quota（多维度配额管理）

> 2026-09-09 新增，对应 P0-5 多维度资源配额。

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/quotas` | GET | 获取配额列表（支持 tenant/user/partition/node 维度） | quota_read |
| `/quotas` | POST | 创建配额 | quota_write |
| `/quotas/{id}` | GET | 获取配额详情 | quota_read |
| `/quotas/{id}` | PUT | 更新配额 | quota_write |
| `/quotas/{id}` | DELETE | 删除配额 | quota_write |
| `/quotas/usage` | GET | 获取配额使用情况（已用/剩余/百分比） | quota_read |
| `/quotas/check` | POST | 校验配额是否充足（作业提交前调用） | 认证 |

### scheduler（调度器集成）

> 2026-09-09 新增，对应 P0-3 Slurm/LSF/SGE 集成。

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/schedulers` | GET | 获取调度器集成列表 | scheduler_read |
| `/schedulers` | POST | 创建调度器集成（slurm/lsf/sge/k8s_native） | scheduler_write |
| `/schedulers/{id}` | GET | 获取调度器详情 | scheduler_read |
| `/schedulers/{id}` | PUT | 更新调度器配置 | scheduler_write |
| `/schedulers/{id}` | DELETE | 删除调度器集成 | scheduler_write |
| `/schedulers/{id}/queues` | GET | 获取调度器队列/分区信息 | scheduler_read |
| `/schedulers/{id}/nodes` | GET | 获取调度器节点信息 | scheduler_read |
| `/schedulers/{id}/sync` | POST | 同步调度器作业状态 | scheduler_write |
| `/schedulers/{id}/health` | GET | 调度器健康检查 | scheduler_read |

### topology（拓扑感知调度）

> 2026-09-09 新增，对应 P1-1 拓扑感知调度。

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/topology` | GET | 获取拓扑信息列表（NUMA/NVLink/机架/交换机） | topology_read |
| `/topology` | POST | 注册拓扑信息 | topology_write |
| `/topology/{id}` | GET | 获取拓扑详情 | topology_read |
| `/topology/{id}` | PUT | 更新拓扑信息 | topology_write |
| `/topology/{id}` | DELETE | 删除拓扑信息 | topology_write |
| `/topology/score` | POST | 计算拓扑调度评分（候选节点排序） | topology_read |

### dataset（数据集与 Fluid 缓存）

> 2026-09-09 新增，对应 P1-5 Fluid 数据加速集成。

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/datasets` | GET | 获取数据集列表（ceph/nfs/s3/glusterfs） | dataset_read |
| `/datasets` | POST | 创建数据集 | dataset_write |
| `/datasets/{id}` | GET | 获取数据集详情 | dataset_read |
| `/datasets/{id}` | PUT | 更新数据集 | dataset_write |
| `/datasets/{id}` | DELETE | 删除数据集 | dataset_write |
| `/datasets/{id}/caches` | GET | 获取数据集缓存配置列表 | dataset_read |
| `/datasets/{id}/caches` | POST | 创建缓存配置（Alluxio/JindoFS） | dataset_write |
| `/datasets/{id}/caches/{cacheId}` | PUT | 更新缓存配置 | dataset_write |
| `/datasets/{id}/caches/{cacheId}` | DELETE | 删除缓存配置 | dataset_write |
| `/datasets/caches/{cacheId}/enable` | POST | 启用 Fluid 缓存 | dataset_write |
| `/datasets/caches/{cacheId}/disable` | POST | 停用 Fluid 缓存 | dataset_write |
| `/datasets/caches/{cacheId}/prefetch` | POST | 触发数据预取（DataLoad） | dataset_write |

### checkpoint（Checkpoint 管理）

> 2026-09-09 新增，对应 P1-4 容错训练。

| 端点 | 方法 | 说明 | 权限 |
|------|------|------|------|
| `/checkpoints` | GET | 获取 Checkpoint 列表 | checkpoint_read |
| `/checkpoints` | POST | 创建 Checkpoint 记录 | checkpoint_write |
| `/checkpoints/{id}` | DELETE | 删除 Checkpoint | checkpoint_write |
| `/checkpoints/latest/{jobId}` | GET | 获取作业最新 Checkpoint（故障恢复用） | checkpoint_read |

---

## OpenAPI 规范

完整的 OpenAPI 3.0 规范文件位于：

- **文件路径**：`metaclouds-backend/docs/swagger.yaml`
- **在线访问（开发环境）**：`http://localhost:8000/api/docs/swagger.yaml`
- **Swagger UI（开发环境）**：`http://localhost:8000/api/docs/swagger-ui/`

开发环境下访问 `http://localhost:8000/api/docs` 可查看 API 文档索引页。

---

## 可观测性

### 指标

Prometheus 指标端点：`GET /metrics`

主要指标类别：
- **HTTP 指标**：`http_requests_total`、`http_request_duration_seconds`、`http_requests_in_flight`
- **业务指标**：`metaclouds_jobs_total`、`metaclouds_jobs_active`、`metaclouds_resources_gpu_total`、`metaclouds_auth_login_total` 等
- **Go 运行时指标**：`go_*`（goroutine、内存、GC 等）
- **进程指标**：`process_*`（CPU、内存、文件描述符等）

### 追踪

- 每个请求自动生成或透传 `X-Trace-ID`
- 响应头返回 `X-Trace-ID`，便于客户端日志关联
- 结构化日志中包含 `trace_id` 字段
- 当前为轻量级本地 span 实现，可后续接入 OpenTelemetry + Jaeger

### 日志

- 结构化日志，包含 `request_id`、`trace_id`、`method`、`path`、`status`、`duration` 等字段
- 慢请求（默认 >500ms）以 WARN 级别记录
- 5xx 错误以 INFO 级别记录完整请求信息

### 告警

告警规则定义在 `alerts.yml`，覆盖：
- 基础设施：实例宕机、CPU/内存/磁盘使用率
- 应用：后端不可用、5xx 错误率、P99 延迟、Pod 重启/CrashLoop
- 业务：作业排队数、作业失败率、GPU 分配率、登录失败率
- 中间件：PostgreSQL 连接数、Redis 内存、etcd 可用性
