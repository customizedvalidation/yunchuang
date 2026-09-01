# 操作级权限收敛：前后端对照清单

> 关联提交：`381b540`（前端按角色收敛写操作按钮可见性）
> 适用仓库：`D:/YCYD`（前后端同仓，单一 git 仓库）
> 文档性质：交付文档 / 安全收敛审计清单

---

## 1. 背景与核心结论

第 10 轮复盘遗留「e 项」——作业管理页等写操作按钮（新建 / 删除 / 提交 / 启用开关）对 `user` 角色仍可见。
经核查，**后端所有写接口均已挂载 `RequirePermission`**，因此「user 见写按钮」属于 **UX 可见性缺口**（非越权漏洞），与之前的菜单 / 路由守卫收敛同属一类。

本轮在 6 个页面用 `<Can perm="...">` 包裹写按钮，使其按角色可见性收敛。

**核心结论（请重点确认）：**

- 每一个被 `<Can>` 包裹的写按钮，其对应的后端写接口**都已挂 `RequirePermission`**。
- 前端 `<Can>` 收敛是**纯 UX（防御性 UI）**；真实访问控制由后端强制，越权请求必然返回 **403 Forbidden**。
- **不存在「仅靠前端隐藏、后端却未守门」的权限点**——这是本清单要证明的关键。

---

## 2. 后端权威角色→权限矩阵（`pkg/authz/authz.go` · `rolePermissions`）

| 权限 `\` 角色 | `admin` | `manager` | `user` |
| --- | --- | --- | --- |
| `admin` | ✅ | — | — |
| `cluster:read` | ✅ | ✅ | ✅ |
| `cluster:write` | ✅ | ✅ | — |
| `resource:read` | ✅ | ✅ | ✅ |
| `resource:write` | ✅ | ✅ | — |
| `job:read` | ✅ | ✅ | ✅ |
| `job:write` | ✅ | ✅ | — |
| `job:submit` | ✅ | ✅ | — |
| `tenant:read` | ✅ | ✅ | — |
| `tenant:write` | ✅ | ✅ | — |
| `monitoring:read` | ✅ | ✅ | ✅ |
| `monitoring:write` | ✅ | ✅ | — |
| `acceleration:read` | ✅ | ✅ | ✅ |
| `acceleration:write` | ✅ | ✅ | — |
| `security:read` | ✅ | ✅ | ✅ |
| `security:write` | ✅ | ✅ | — |

后端 `HasPermission(role, perm)`：**`admin` 始终放行；未知 / 缺失角色一律返回 `false`（fail-closed）**。
前端 `hasPermission(perm)`：role 为 `null`（读不到 / 非法）或 `admin` 时放行，其余查矩阵（fail-open，因为真正的门禁在后端）。

---

## 3. 后端强制门禁（`api/routes.go` 的 `RequirePermission` 挂载点）

所有 `/api/v1/*` 均在 `JWTAuth` 组内（须登录）。下表列出**实际挂了 `RequirePermission` 的端点**（即真实安全边界）。

| 域 | 方法 + 路径 | 守卫权限 | 控制器动作 |
| --- | --- | --- | --- |
| clusters | `POST /clusters` | `cluster:write` | CreateCluster |
| clusters | `PUT /clusters/:id` | `cluster:write` | UpdateCluster |
| clusters | `DELETE /clusters/:id` | `cluster:write` | DeleteCluster |
| resources | `PUT /resources/:id` | `resource:write` | UpdateResource |
| jobs | `POST /jobs` | `job:write` | CreateJob |
| jobs | `PUT /jobs/:id` | `job:write` | UpdateJob |
| jobs | `DELETE /jobs/:id` | `job:write` | DeleteJob |
| jobs | `POST /jobs/:id/cancel` | `job:write` | CancelJob |
| jobs | `POST /jobs/:id/submit` | `job:submit` | SubmitJob（K8S） |
| monitoring | `PUT /monitoring/alerts/:id/resolve` | `monitoring:write` | ResolveAlert |
| tenants | `GET /tenants` | `tenant:read` | GetTenants |
| tenants | `GET /tenants/:id` | `tenant:read` | GetTenant |
| tenants | `POST /tenants` | `tenant:write` | CreateTenant |
| tenants | `PUT /tenants/:id` | `tenant:write` | UpdateTenant |
| tenants | `DELETE /tenants/:id` | `tenant:write` | DeleteTenant |
| acceleration | `POST /acceleration` | `acceleration:write` | CreateAccelerationSuite |
| acceleration | `PUT /acceleration/:id` | `acceleration:write` | UpdateAccelerationSuite |
| acceleration | `DELETE /acceleration/:id` | `acceleration:write` | DeleteAccelerationSuite |
| security | `POST /security/policies` | `security:write` | CreateSecurityPolicy |
| security | `PUT /security/policies/:id` | `security:write` | UpdateSecurityPolicy |
| security | `DELETE /security/policies/:id` | `security:write` | DeleteSecurityPolicy |

**读端点（仅 `JWTAuth`，不按角色门禁）**：`GET /clusters`、`GET /clusters/:id`、`GET /resources`、`GET /resources/:id`、`GET /jobs`、`GET /jobs/:id`、`GET /monitoring/metrics`、`GET /monitoring/alerts`、`GET /acceleration`、`GET /acceleration/:id`、`GET /security/policies`、`GET /security/policies/:id`。
→ 这些共享平台状态的读操作对**任意已登录用户**开放；对应 `*_read` 权限在前端仅用于菜单 / 路由可见性，**不构成后端安全边界**（除 `tenant:read` 例外，见上表）。

> 注：`pkg/authz` 还导出了 `RequireAdmin()`，但当前 `routes.go` 未在任何路由挂载，属预留能力。

---

## 4. 前端 `<Can>` 收敛映射（提交 `381b540`）

| 页面 | 按钮 / 操作 | 包裹权限 | 对应后端端点 + 守卫 |
| --- | --- | --- | --- |
| `JobManagement.tsx` | 新建作业（页头 / EmptyState） | `job:write` | `POST /jobs` → `job:write` |
| `JobManagement.tsx` | Modal「创建」 | `job:write` | `POST /jobs` → `job:write` |
| `ClusterManagement.tsx` | 删除 Popconfirm | `cluster:write` | `DELETE /clusters/:id` → `cluster:write` |
| `ClusterManagement.tsx` | 创建集群（页头 / EmptyState） | `cluster:write` | `POST /clusters` → `cluster:write` |
| `ClusterManagement.tsx` | Modal「创建」 | `cluster:write` | `POST /clusters` → `cluster:write` |
| `MultiTenantManagement.tsx` | 删除 Popconfirm | `tenant:write` | `DELETE /tenants/:id` → `tenant:write` |
| `MultiTenantManagement.tsx` | 创建租户（页头 / EmptyState） | `tenant:write` | `POST /tenants` → `tenant:write` |
| `MultiTenantManagement.tsx` | Modal「创建」 | `tenant:write` | `POST /tenants` → `tenant:write` |
| `K8SManagement.tsx` | 提交到 K8S | `job:submit` | `POST /jobs/:id/submit` → `job:submit` |
| `K8SManagement.tsx` | 取消作业 | `job:write` | `POST /jobs/:id/cancel` → `job:write` |
| `SecurityManagement.tsx` | 启用 / 禁用 Switch | `security:write` | `PUT /security/policies/:id` → `security:write` |
| `AccelerationSuiteManagement.tsx` | 启用 / 禁用 Switch | `acceleration:write` | `PUT /acceleration/:id` → `acceleration:write` |

> 未改动页面（无真实写按钮，无需收敛）：`ResourceManagement.tsx`（仅 EmptyState 文案）、`MonitoringAlert.tsx`（仅「刷新」）、`Dashboard.tsx`。

---

## 5. 分类判定：哪些由后端强制、哪些是纯 UX 收敛

### A 类 —— 后端强制 + 前端收敛（双保险，最安全）

所有被 `<Can>` 包裹的**写权限**均在此列，且后端都已 `RequirePermission` 守门：

`cluster:write` · `resource:write` · `job:write` · `job:submit` · `tenant:write` · `monitoring:write` · `acceleration:write` · `security:write` · `tenant:read`

- 行为：`<Can>` 隐藏按钮是 UI 友好；即使用户绕过 UI 直接调用 API，后端仍按 `RequirePermission` 返回 403。
- 结论：**这些 `<Can>` 收敛是纯 UX 增强，不引入任何新的安全边界，也不会因绕过 UI 而产生越权**。

### B 类 —— 纯前端可见性（后端不按角色门禁读）

`cluster:read` · `resource:read` · `job:read` · `monitoring:read` · `acceleration:read` · `security:read`

- 行为：这些 `*_read` 权限仅存在于前端 `ROLE_PERMISSIONS` 矩阵，用于菜单 / 路由守卫的可见性收敛；后端对应 GET 端点只要求 `JWTAuth`。
- 性质：**不是安全缺口**——读的是共享平台状态，任意已登录用户可见是设计预期。`tenant:read` 是例外（见 A 类，后端读租户已门禁）。

### 关于 `admin`

`admin` 权限在后端 `HasPermission` 中作为「全放行」开关，不绑定具体端点；前端 `ROLE_PERMISSIONS.admin` 包含全部权限，仅用于让管理员看到所有按钮。后端 `RequireAdmin()` 当前未被任何路由使用（预留）。

---

## 6. 验证证据

| 门禁 | 结果 | 说明 |
| --- | --- | --- |
| `tsc --noEmit` | ✅ 通过 | 修复了 `accel:write`→`acceleration:write` 类型错（前端常量须对齐后端 `PermissionAccelWrite`） |
| `vite build` | ✅ 通过 | 生产构建无误 |
| CDP `verify-buttons.mjs` | ✅ 全绿 | `userWriteButtonsHidden` / `adminWriteButtonsVisible` / `noConsoleErrors` 三项均 `true` |
| 后端 `go build / vet / test ./...` | ✅（前序轮次） | 越权防护与响应契约一致 |

CDP 真实浏览器验证要点：注册 `btnuser`（`user` 角色）→ `/job` 不含「新建作业」、`/cluster` 不含「创建集群」；`admin` 登录后两按钮可见；全程零 console / page 异常。

---

## 7. 维护约定（防矩阵漂移）

1. **前端必须镜像后端**：`src/utils/auth.ts` 的 `ROLE_PERMISSIONS` 与 `pkg/authz/authz.go` 的 `rolePermissions` 保持同形。后端改矩阵时，前端**必须同步**修改（已在 `auth.ts` 注释 `@mapping` 标注）。
2. **新增写按钮的两道关卡（缺一不可）**：
   - (a) 后端对应端点挂 `authz.RequirePermission(...)` —— 这是**安全**；
   - (b) 前端按钮用 `<Can perm="...">` 包裹 —— 这是 **UX**。
   只做 (a) 会暴露按钮（UX 缺口）；只做 (b) 会被后端 403（功能不可用）。两者都做才是完整收敛。
3. **新增权限常量**：后端在 `authz.go` 加 `PermissionXxx` 常量 + 在 `rolePermissions` 各角色补条目；前端在 `auth.ts` 的 `Permission` 联合类型加成员 + 在 `ROLE_PERMISSIONS` 各角色补条目。
4. **JWT 仅携带 `role`**，不携带权限列表，因此前端权限判定永远是「镜像 + 本地判定」，后端 `RequirePermission` 才是唯一权威裁决；前端 fail-open 安全、后端 fail-closed 安全。

---

## 8. 推送状态说明

提交 `381b540` 当前为**本地提交**，`git remote -v` 为空（仓库未配置远端），因此无法推送。如需推送，请先添加远端：

```bash
git remote add origin <your-remote-url>
git push -u origin main
```

（本清单文档本身亦建议随 `381b540` 一并纳入版本管理。）
