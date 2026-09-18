# Metaclouds Backend (Rust) API 参考

> 基于 `docs/openapi-rust.json`（61 路径 / 107 方法）整理。
> 所有端点前缀：`/api/v1`。完整交互式文档见运行中的 `/swagger-ui`。

## 通用约定

- **认证**：除 `POST /auth/login` 和 `/metrics`、`/swagger-ui` 外，所有端点需 JWT（HttpOnly Cookie `access_token`）
- **CSRF**：写操作（POST/PUT/DELETE）需携带 `X-CSRF-Token` 头（从 `csrf_token` Cookie 读取）
- **响应信封**：`{success: bool, data: T | null, message: string?, code: string?, timestamp: number}`
- **分页参数**：`?page=1&page_size=10`（page_size 最大 100）
- **错误码**：`BAD_REQUEST`(400)、`UNAUTHORIZED`(401)、`FORBIDDEN`(403)、`NOT_FOUND`(404)、`CONFLICT`(409)、`INTERNAL_SERVER_ERROR`(500)、`VALIDATION_ERROR`(400)、`RATE_LIMIT_EXCEEDED`(429)、`SERVICE_UNAVAILABLE`(503)

---

## 1. auth（认证）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| POST | `/auth/login` | 公开 | 登录，返回 `{token, user, expires_at}`，设置 HttpOnly Cookie |
| POST | `/auth/logout` | JWT | 登出，清除 Cookie |
| POST | `/auth/refresh` | JWT | 刷新令牌 |
| GET | `/auth/profile` | JWT | 获取当前用户信息 |
| GET | `/auth/csrf` | JWT | 获取 CSRF 令牌 |
| PUT | `/auth/change-password` | JWT | 修改密码（Rust 独有） |

**登录请求体**：
```json
{ "username": "admin", "password": "Admin@123456" }
```

**登录成功响应**（200）：
```json
{
  "success": true,
  "data": {
    "token": "<jwt>",
    "user": { "id": 1, "username": "admin", "role": "admin", "tenant_id": 1 },
    "expires_at": 1789786358
  },
  "timestamp": 1789699958
}
```

---

## 2. users（用户管理）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/users` | admin | 用户列表（分页） |
| POST | `/users` | admin | 创建用户 |
| GET | `/users/{id}` | admin | 用户详情 |
| PUT | `/users/{id}` | admin | 更新用户 |
| DELETE | `/users/{id}` | admin | 删除用户 |

---

## 3. tenants（租户）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/tenants` | `tenant:read` | 租户列表 |
| POST | `/tenants` | `tenant:write` | 创建租户 |
| GET | `/tenants/{id}` | `tenant:read` | 租户详情 |
| PUT | `/tenants/{id}` | `tenant:write` | 更新租户 |
| DELETE | `/tenants/{id}` | `tenant:write` | 删除租户 |

---

## 4. clusters（集群）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/clusters` | JWT | 集群列表 |
| POST | `/clusters` | `cluster:write` | 创建集群 |
| GET | `/clusters/{id}` | JWT | 集群详情 |
| PUT | `/clusters/{id}` | `cluster:write` | 更新集群 |
| DELETE | `/clusters/{id}` | `cluster:write` | 删除集群 |

---

## 5. resources（资源）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/resources` | JWT | 资源列表 |
| POST | `/resources` | `resource:write` | 创建资源 |
| GET | `/resources/{id}` | JWT | 资源详情 |
| PUT | `/resources/{id}` | `resource:write` | 更新资源 |
| DELETE | `/resources/{id}` | `resource:write` | 删除资源 |

---

## 6. topology（拓扑节点）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/topology` | JWT | 节点列表 |
| POST | `/topology` | `topology:write` | 创建节点 |
| GET | `/topology/{id}` | JWT | 节点详情 |
| PUT | `/topology/{id}` | `topology:write` | 更新节点 |
| DELETE | `/topology/{id}` | `topology:write` | 删除节点 |
| GET | `/topology/nodes` | JWT | 别名：节点列表 |
| GET | `/topology/nodes/{id}` | JWT | 别名：节点详情 |
| POST | `/topology/nodes` | `topology:write` | 别名：创建节点 |
| PUT | `/topology/nodes/{id}` | `topology:write` | 别名：更新节点 |
| DELETE | `/topology/nodes/{id}` | `topology:write` | 别名：删除节点 |

---

## 7. k8s（K8s 模拟）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/k8s/clusters/{id}/pods` | JWT | Pod 列表（模拟） |
| GET | `/k8s/clusters/{id}/nodes` | JWT | 节点列表（模拟） |
| GET | `/k8s/clusters/{id}/health` | JWT | 集群健康状态（模拟） |

---

## 8. jobs（作业）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/jobs` | JWT | 作业列表 |
| GET | `/jobs/stats` | JWT | 作业统计（Rust 独有） |
| POST | `/jobs` | `job:write` | 创建作业 |
| GET | `/jobs/{id}` | JWT | 作业详情 |
| PUT | `/jobs/{id}` | `job:write` | 更新作业 |
| DELETE | `/jobs/{id}` | `job:write` | 删除作业 |
| POST | `/jobs/{id}/cancel` | `job:write` | 取消作业 |

> **差异**：Go 版有 `POST /jobs/:id/submit`，Rust 版未实现（通过 `POST /jobs` 创建即提交）。

---

## 9. gpus（GPU）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/gpus` | JWT | GPU 设备列表 |
| POST | `/gpus` | `gpu:write` | 创建 GPU 设备 |
| GET | `/gpus/{id}` | JWT | GPU 详情 |
| PUT | `/gpus/{id}` | `gpu:write` | 更新 GPU 设备 |
| DELETE | `/gpus/{id}` | `gpu:write` | 删除 GPU 设备 |
| GET | `/gpus/allocations` | JWT | GPU 分配列表 |
| POST | `/gpus/allocations` | `job:write` | 分配 GPU |
| DELETE | `/gpus/allocations/{id}` | `job:write` | 释放 GPU |
| GET | `/gpus/utilization` | JWT | GPU 利用率 |
| GET | `/gpu/devices` | JWT | 别名：GPU 设备列表 |
| POST | `/gpu/devices` | `gpu:write` | 别名：创建 GPU 设备 |
| GET | `/gpu/devices/{id}` | JWT | 别名：GPU 详情 |
| PUT | `/gpu/devices/{id}` | `gpu:write` | 别名：更新 GPU 设备 |
| DELETE | `/gpu/devices/{id}` | `gpu:write` | 别名：删除 GPU 设备 |
| GET | `/gpu/allocations` | JWT | 别名：分配列表 |
| POST | `/gpu/allocations` | `job:write` | 别名：分配 GPU |
| POST | `/gpu/allocations/{id}/release` | `job:write` | 别名：释放 GPU |
| GET | `/gpu/utilization` | JWT | 别名：利用率 |

> **差异**：Go 版有 `GET /resources/gpu`，Rust 版用 `/gpus` 替代。

---

## 10. partitions（分区）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/partitions` | JWT | 分区列表 |
| POST | `/partitions` | `partition:write` | 创建分区 |
| GET | `/partitions/{id}` | JWT | 分区详情 |
| PUT | `/partitions/{id}` | `partition:write` | 更新分区 |
| DELETE | `/partitions/{id}` | `partition:write` | 删除分区 |
| GET | `/partitions/{id}/resources` | JWT | 分区资源 |
| POST | `/partitions/{id}/permissions` | `partition:write` | 授权 |
| DELETE | `/partitions/{id}/permissions/{perm_id}` | `partition:write` | 撤销授权 |

---

## 11. quotas（配额）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/quotas` | JWT | 配额列表 |
| POST | `/quotas` | `quota:write` | 创建配额 |
| GET | `/quotas/{id}` | JWT | 配额详情 |
| PUT | `/quotas/{id}` | `quota:write` | 更新配额 |
| DELETE | `/quotas/{id}` | `quota:write` | 删除配额 |
| POST | `/quotas/{id}/check` | JWT | 校验配额 |

> **差异**：Go 版为 `POST /quotas/check`（无路径参数），Rust 版为 `POST /quotas/{id}/check`。P4-01 正在对齐。

---

## 12. schedulers（调度器）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/schedulers` | JWT | 调度器列表 |
| POST | `/schedulers` | `scheduler:write` | 创建调度器 |
| GET | `/schedulers/{id}` | JWT | 调度器详情 |
| PUT | `/schedulers/{id}` | `scheduler:write` | 更新调度器 |
| DELETE | `/schedulers/{id}` | `scheduler:write` | 删除调度器 |
| POST | `/schedulers/{id}/sync` | `scheduler:write` | 同步资源 |
| POST | `/schedulers/{id}/test-connection` | `scheduler:write` | 测试连接 |

---

## 13. datasets（数据集）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/datasets` | JWT | 数据集列表 |
| POST | `/datasets` | `dataset:write` | 创建数据集 |
| GET | `/datasets/{id}` | JWT | 数据集详情 |
| PUT | `/datasets/{id}` | `dataset:write` | 更新数据集 |
| DELETE | `/datasets/{id}` | `dataset:write` | 删除数据集 |

---

## 14. checkpoints（检查点）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/checkpoints` | JWT | 检查点列表 |
| POST | `/checkpoints` | `checkpoint:write` | 创建检查点 |
| GET | `/checkpoints/{id}` | JWT | 检查点详情 |
| PUT | `/checkpoints/{id}` | `checkpoint:write` | 更新检查点 |
| DELETE | `/checkpoints/{id}` | `checkpoint:write` | 删除检查点 |

---

## 15. acceleration（加速套件）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/acceleration` | JWT | 套件列表 |
| POST | `/acceleration` | `acceleration:write` | 创建套件 |
| GET | `/acceleration/{id}` | JWT | 套件详情 |
| PUT | `/acceleration/{id}` | `acceleration:write` | 更新套件 |
| DELETE | `/acceleration/{id}` | `acceleration:write` | 删除套件 |
| POST | `/acceleration/{id}/start` | `acceleration:write` | 启动套件 |
| POST | `/acceleration/{id}/stop` | `acceleration:write` | 停止套件 |

---

## 16. monitoring（监控）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/monitoring/dashboard` | JWT | Dashboard 统计数据 |
| GET | `/monitoring/metrics` | JWT | 监控指标 |
| GET | `/monitoring/alert-rules` | JWT | 告警规则列表 |
| POST | `/monitoring/alert-rules/evaluate` | `monitoring:write` | 评估告警规则 |

---

## 17. alerts（告警）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/alerts` | JWT | 告警列表 |
| GET | `/alerts/stats` | JWT | 告警统计（Rust 独有） |
| POST | `/alerts` | `alert:write` | 创建告警 |
| GET | `/alerts/{id}` | JWT | 告警详情 |
| PUT | `/alerts/{id}` | `alert:write` | 更新告警 |
| DELETE | `/alerts/{id}` | `alert:write` | 删除告警 |
| POST | `/alerts/{id}/acknowledge` | `alert:write` | 确认告警 |
| POST | `/alerts/{id}/resolve` | `alert:write` | 解决告警 |

---

## 18. security（安全策略）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/security/policies` | JWT | 策略列表 |
| POST | `/security/policies` | `security:write` | 创建策略 |
| GET | `/security/policies/{id}` | JWT | 策略详情 |
| PUT | `/security/policies/{id}` | `security:write` | 更新策略 |
| DELETE | `/security/policies/{id}` | `security:write` | 删除策略 |
| POST | `/security/policies/{id}/enable` | `security:write` | 启用策略 |
| POST | `/security/policies/{id}/disable` | `security:write` | 禁用策略 |

---

## 19. metrics（Prometheus）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/metrics` | 公开 | Prometheus 文本格式指标（无 JWT） |

---

## 20. swagger（API 文档）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/swagger-ui` | 公开 | 交互式 API 文档 UI |
| GET | `/api-docs/openapi.json` | 公开 | 原始 OpenAPI 3.0 JSON |

---

## 与 Go 版差异汇总

| # | 差异 | Go 版 | Rust 版 | 状态 |
|---|------|-------|---------|------|
| 1 | 根级健康检查 | `GET /health` | 不存在 | K8s 用 `/metrics` |
| 2 | 配额校验路径 | `POST /quotas/check` | `POST /quotas/{id}/check` | P4-01 对齐中 |
| 3 | GPU 资源端点 | `GET /resources/gpu` | 未实现 | 用 `/gpus` 替代 |
| 4 | 作业提交 | `POST /jobs/:id/submit` | 未实现 | 用 `POST /jobs` 创建 |
| 5 | 集群状态 | `GET /clusters/:id/status` | 未实现 | 用 `GET /clusters/:id` |
| 6 | 作业统计 | 无 | `GET /jobs/stats` | Rust 独有 |
| 7 | 告警域 | 部分 | `/alerts` 完整 CRUD + ack/resolve/stats | Rust 独有/增强 |
| 8 | 修改密码 | 部分 | `PUT /auth/change-password` | Rust 独有 |
| 9 | 别名路径 | 无 | `/topology/nodes`、`/gpu/*` 系列 | Rust 独有兼容 |
