# P4-01 Golden 全量对等回归报告

- 任务：P4-01 Golden 全量对等回归（L1/L2/L3 三档 + manager/user 角色 403 矩阵）
- 执行时间：2026-09-18（Asia/Shanghai, UTC+8）
- 执行方：Phase 4 回归执行代理
- 对照源（Go）：`D:\YCYD\metaclouds-backend`（保留不改）
- 待测（Rust）：`D:\YCYD\metaclouds-backend-rust`

---

## 1. 测试环境

| 项 | Go 版（对照源） | Rust 版（待测） |
|---|---|---|
| 监听端口 | `http://localhost:8000` | `http://localhost:8001` |
| 健康检查 | `GET /health` | `POST /api/v1/auth/login`（探活） |
| 运行模式 | 内存存储 `MEMORY_STORE_ENABLED=true` | SQLite `sqlite:metaclouds.db`（持久） |
| 关键环境变量 | `SERVER_PORT=8000`、`SERVER_ENV=development`、`DEFAULT_ADMIN_PASSWORD=Admin@123456`、`DEFAULT_USER_PASSWORD=User@123456`、`ALLOW_PUBLIC_REGISTRATION=true`、`RATE_LIMIT_ENABLED=false` | `SERVER_PORT=8001`（`.env`） |
| 路由前缀 | `/api/v1` | `/api/v1` |

### 三角色凭据与 token

| 角色 | Go 版(:8000) | Rust 版(:8001) |
|---|---|---|
| admin | `admin / Admin@123456` ✅ | `admin / Admin@123456` ✅（DB 播种） |
| manager | **无法获取** ⚠️ | `manager / Manager@123456` ✅（admin 经 `POST /users` 创建，JWT `role=manager`） |
| user | `testuser / User@123456` ✅（自助注册，Go 强制 `role=user`） | `rbacuser / User@123456` ✅（admin 经 `POST /users` 创建，JWT `role=user`） |

> **Go 侧 manager 不可得（发现项）**：Go `api/routes.go` 未注册任何 `/users` CRUD 端点；自助注册 `POST /auth/register` 即便开启，`auth_service.Register` 也会把请求体中的 `role` 一律降级为 `"user"`。因此 Go 参考侧**无法经 API 签发 manager token**。Go 的 L3 矩阵仅覆盖 admin / user / 未认证三身份；manager 列在 Go 侧记为 `N/A`。Rust 侧 manager 矩阵完整。

> **限流说明**：Go 默认 `RATE_LIMIT_ENABLED=true`（100 次/60s）。首轮遍历触发大量 `429` 污染结果，已重启 Go 并设 `RATE_LIMIT_ENABLED=false` 后重跑。Rust 无此限流。

---

## 2. 端点总数统计

| 项 | 数量 |
|---|---|
| 本次遍历的路由变体（方法+路径模板） | **145** |
| 其中 GET（读） | 78 |
| 其中写（POST/PUT/DELETE） | 67 |
| 公开/仅 JWT 路由 | 见下表 |

> 覆盖度：145 ≥ 验收要求的 80。端点清单从 Rust `src/routes.rs` 与 Go `api/routes.go` 合并提取（含 Vue3 别名 `/gpu/*`、`/topology/nodes`、`/fluid-caches`）。

---

## 3. L1 档（状态码 + 信封）结果汇总

> admin token 双发，对比 HTTP 状态码与响应信封顶层字段。Go `NoRoute` 回退为前端 `index.html`（200 + HTML），脚本已将其归一为 `NOTIMPL`。

| 指标 | 数量 |
|---|---|
| admin 状态码一致 | 41 |
| admin 状态码不一致 | 103（排除 `/auth/login`） |
| 信封顶层字段集合一致 | 106 |
| 信封顶层字段集合不一致 | 38 |

不一致 103 项按性质分类（**并非全部为缺陷**）：

| 分类 | 典型 | 判定 |
|---|---|---|
| **路由单侧存在**（一端 404/405，另一端 2xx） | Rust 独有 `/k8s/*`、`/jobs/stats`、`/alerts*`、`/monitoring/dashboard|alert-rules*`、`/acceleration/:id/start|stop`、`/security/policies/:id/enable|disable`；Go 独有 `/monitoring/alerts*`、`/datasets/:id/caches`、`/clusters/:id/status`、`/quotas/usage`、`/topology/score` | 见 §6 路由差异裁决 |
| **空写请求校验严格度**（Go 宽松→201，Rust 严格→422） | `POST /gpus`、`/gpu/devices`、`/partitions`、`/quotas`、`/schedulers`、`/datasets`、`/checkpoints`（Go=201，Rust=422） | **可接受**：用对齐请求体复测预期收敛 |
| **详情路由数据依赖**（Rust 空库→404，Go 有种子→200） | `GET /jobs/:id`、`/gpus/:id`、`/clusters/:id`、`/acceleration/:id` 等 | **可接受**：数据独立，非契约差异 |
| **配额校验路径** | `POST /quotas/check`（Go=400/Rust=405）、`POST /quotas/:id/check`（Go=404/Rust=422） | **必须修复**，见 §6 |

信封结构本身两版一致：成功 `{success, data, timestamp}`，失败 `{success, error:{code,message}, timestamp}`。唯一信封字段名差异：`POST /auth/logout` 成功时 Rust 顶层为 `message` 而非 `data`。

---

## 4. L2 档（数据形状）结果汇总

| 指标 | 数量 |
|---|---|
| GET 端点 data 形状一致 | 2 |
| GET 端点 data 形状不一致 | 9 |
| 不适用（写/无数据） | 133 |

### 🔴 关键 L2 差异（系统性，必须修复/裁决）

**列表分页信封两版不一致——这是本次回归最大的契约分歧：**

- **Go**：`data` 直接是**裸数组**
  ```json
  {"success":true,"data":[{"id":2,"name":"CPU-Cluster-1",...}],"timestamp":...}
  ```
- **Rust**：`data` 是**分页对象** `{data,total,page,page_size,total_pages}`
  ```json
  {"success":true,"data":{"data":[],"total":0,"page":1,"page_size":10,"total_pages":0},"timestamp":...}
  ```

受影响端点（Go `data`=数组 vs Rust `data`=分页对象）：
`/tenants`、`/clusters`、`/resources`、`/topology(/nodes)`、`/jobs`、`/gpus(/allocations)`、`/gpu/devices`、`/partitions`、`/quotas`、`/schedulers`、`/datasets`、`/checkpoints`、`/acceleration`、`/security/policies`。

> 任务书把分页结构 `{data,total,page,page_size,total_pages}` 列为期望契约；Rust 实现了它，Go 参考仍是裸数组。需产品/前端裁决以哪端为准并对齐另一端。

另一处：`GET /monitoring/metrics` 字段集完全不同（Go=`{cpu,gpu,jobs,memory,network,storage}`；Rust=`{active_alerts,running_jobs,total_*,system_uptime,...}`），属监控指标口径差异。

---

## 5. L3 档（RBAC 403 矩阵，重点）

四身份状态码分布（145 端点）：

| 身份 | Rust 分布 | Go 分布 |
|---|---|---|
| 未认证(无 token) | 401:123, 404:21, 422:1 | 401:113, 404:29, 400:2, 200:1 |
| user | 403:72, 200:30, 404:38, 400:2, 422:2, 401:1 | 403:62, 200:34, 404:43, 400:5, 401:1 |
| manager | 403:69, 200:32, 404:39, 400:2, 422:2, 401:1 | N/A（见 §1） |
| admin | 404:87, 200:31, 422:19, 405:3, 503:2, 400:2, 401:1 | 200:45, 404:71, 400:15, 500:4, 409:6, 201:3 |

### 5.1 正确行为（PASS）

- **未认证**：两版所有受保护端点均返回 `401`（Rust 123/145 为 401，其余 404 为该端点对端不存在）。✅
- **user 只读**：user 对所有写端点收到 `403`（Rust 72 项），对只读 GET 收到 `200`。✅
- **manager 越权点正确 403**：`/users*`（admin 权限）、`POST /tenants*`（tenant:write）、`POST /security/policies*`（security:write）对 manager/user 均正确 `403`。✅

### 5.2 🔴 必须修复：manager 在 Rust 运行时被过度收紧（关键发现）

按 Rust 源码 `src/authz/mod.rs` 与 Go 参考 `pkg/authz/authz.go` 的同一权限矩阵，**manager 应具备** `cluster:write`、`resource:write`、`job:write`、`gpu:write`、`topology:write`、`partition:write`、`quota:write`、`scheduler:write`、`dataset:write`、`checkpoint:write`、`acceleration:write`。

但运行时实测（干净 `cargo build` 后复测，已排除二进制陈旧）：

| 端点 | 所需权限 | manager 实测 | 预期 |
|---|---|---|---|
| `POST /clusters` | cluster:write | **403** | 应放行（空体→422） |
| `POST /gpus` / `POST /gpu/devices` | gpu:write | **403** | 应放行 |
| `POST /jobs` | job:write | **403** | 应放行 |
| `POST /tenants` | tenant:write | 403 | 403 ✅ |
| `POST /security/policies` | security:write | 403 | 403 ✅ |
| `POST /monitoring/alert-rules/evaluate` | monitoring:write | **200** | 应放行 ✅ |
| `GET /tenants` | tenant:read | 200 | ✅ |
| `GET /users` | admin | 403 | ✅ |

即：**manager 的读权限（含 tenant:read）与 monitoring:write 生效，但其余十余类写权限全部被错误地 403**。JWT 已确认携带 `role=manager`（解码 payload 验证），排除 token 问题。

> **疑似根因**：`require_permission` 经 `from_fn_with_state(<perm>.to_string(), …)` 绑定的权限字符串，在多数业务路由组（clusters/gpus/jobs/…）的 `.route_layer` 上未正确传入（唯 monitoring 组与 users 组表现正常）。需 P4 修复 Rust 路由层状态绑定。这正是 Phase 2/3 遗留、本次重点补测 manager/user 403 矩阵要抓的问题。

### 5.3 RBAC 矩阵（节选，全量见 `scripts/p4work/matrix.md` / `results.csv`）

| Method | Path | 所需 | R_admin | R_mgr | R_user | R_none | G_admin | G_user | G_none |
|---|---|---|---|---|---|---|---|---|---|
| GET | /tenants | tenant:read | 200 | 200 | 403 | 401 | 200 | 200 | 401 |
| POST | /tenants | tenant:write | 422 | 403 | 403 | 401 | 201 | 403 | 401 |
| GET | /clusters | jwt | 200 | 200 | 200 | 401 | 200 | 200 | 401 |
| POST | /clusters | cluster:write | 422 | **403⚠** | 403 | 401 | 201 | 403 | 401 |
| GET | /users | admin | 200 | 403 | 403 | 401 | 404 | 404 | 404 |
| POST | /jobs | job:write | 422 | **403⚠** | 403 | 401 | 201 | 403 | 401 |
| POST | /monitoring/alert-rules/evaluate | monitoring:write | 200 | 200 | 403 | 401 | 404 | 404 | 404 |
| GET | /alerts | jwt | 200 | 200 | 200 | 401 | 404 | 404 | 404 |

> ⚠ = manager 应放行却 403 的缺陷行。其余 `404` 为单侧路由不存在，非权限问题。

---

## 6. 路由差异逐项裁决

### 6.1 必须修复（契约/行为不一致）

| # | 差异 | 裁决 |
|---|---|---|
| M1 | **列表分页信封**：Go `data`=裸数组；Rust `data`=`{data,total,page,page_size,total_pages}` | 产品/前端裁决统一契约；建议 Rust 为准（任务书期望分页结构），Go 前端适配或 Rust 回退，二选一 |
| M2 | **manager 写权限运行时被错误 403**（cluster/gpu/job/resource/topology/partition/quota/scheduler/dataset/checkpoint/acceleration:write） | 修复 Rust `route_layer` 权限状态绑定，使 manager 按矩阵放行 |
| M3 | **配额校验路径 + 契约**：Go `POST /quotas/check`（body=scopeType/scopeID/resourceType/requested，返回 `{allowed}`）；Rust `POST /quotas/{id}/check`（body=行 id） | 非纯改名：请求/响应契约均不同。建议 Rust 按 Go 契约新增 `POST /quotas/check`（按 scope 查配额），保留或弃用 `:id` 版本待前端确认 |

### 6.2 可接受差异

- 空写请求校验严格度：Go 宽松 201 / Rust 严格 422（用合法请求体复测应收敛）。
- 详情路由 `:id=1` 在 Rust 空库返回 404、Go 有种子返回 200（数据独立，非契约）。
- `POST /auth/logout` 成功信封字段名 Rust 用 `message`、Go 用 `data`。

### 6.3 已知遗留（单侧路由，不阻断）

**Rust 独有（合理扩展，建议 Go 后续对齐或记录）**：
`/users` CRUD(admin)、`/k8s/clusters/:id/pods|nodes|health`、`/jobs/stats`、`/alerts*` 独立域（含 `/stats`、`/acknowledge`、`/resolve`）、`/monitoring/dashboard`、`/monitoring/alert-rules*`、`/acceleration/:id/start|stop`、`/security/policies/:id/enable|disable`、`/partitions/:id/resources`、`/schedulers/:id/test-connection`、`/auth/change-password`。

**Go 独有（Rust 待补/记录遗留）**：
`/clusters/:id/status`、`/resources/gpu`、`/jobs/:id/submit`、`/jobs/:id/status`、`/monitoring/alerts` + `PUT /monitoring/alerts/:id/resolve`、`/partitions/:id/priority`、`/partitions/:id/max-runtime`、`/partitions/:id/permissions`(GET)、`/quotas/usage`、`/schedulers/:id/queues|nodes|health`、`/topology/score`、`/checkpoints/latest/:jobId`、`/datasets/:id/caches`（+ 增删改）。

---

## 7. 修复建议（按优先级）

1. **P0 — M2**：定位并修复 Rust `require_permission` 的 `route_layer` 状态绑定，使 manager 写权限按矩阵放行；补一条「manager POST /clusters 非 403」的回归断言。
2. **P0 — M1**：裁决并统一列表分页信封（裸数组 vs 分页对象），这是跨全部列表端点的前端契约。
3. **P1 — M3**：按 Go 契约对齐 Rust 配额校验端点路径与请求/响应结构。
4. **P2**：收敛单侧路由（Rust 补 Go 独有 16 项 或 Go 补 Rust 独有 12 项），消除 `404` 差异面。
5. **P2**：`POST /auth/logout` 信封字段名对齐（`message`→`data`）。

---

## 8. 交付物与复现

- 本报告：`docs/golden-regression-phase4.md`
- 可复用对比脚本：`scripts/golden-compare.ps1`（输入端点清单 + 双基址 + 三角色 token，输出 L1/L2/L3 CSV）
- 原始数据：`scripts/p4work/results.csv`（145 行逐端点四身份状态码）、`matrix.md`（RBAC 矩阵表）
- 复现：先启动双服务 → 登录写 `scripts/p4work/tokens.json` → `powershell -ExecutionPolicy Bypass -File scripts/golden-compare.ps1`。

> 说明：按任务要求，本次未做 git 提交，由 Phase 4 整合代理统一提交。
