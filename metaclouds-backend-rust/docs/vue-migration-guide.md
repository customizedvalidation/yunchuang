# 前端 Vue 迁移指南 — Go 版 → Rust 版后端对接

> 本文档指导前端 Vue3 项目（`metaclouds-frontend-vue`）从 Go 版后端（:8000）切换到 Rust 版后端（:8001）。
> 核心结论：**契约完全一致，前端业务代码零修改，仅需修改 API base URL 代理目标。**

---

## 1. API Base URL 切换

### 1.1 当前配置（指向 Go 版）

前端 `vite.config.ts` 中配置了开发代理：

```typescript
// vite.config.ts
server: {
  port: 3000,
  proxy: {
    '/api': {
      target: 'http://localhost:8000',   // ← 当前指向 Go 版
      changeOrigin: true,
    },
  },
},
```

前端 `src/api/http.ts` 使用相对路径 baseURL：

```typescript
const http = axios.create({
  baseURL: '/api/v1',          // 相对路径，由 Vite 代理转发
  timeout: 30000,
  withCredentials: true,
})
```

### 1.2 切换到 Rust 版（开发环境）

修改 `vite.config.ts` 的 proxy target：

```typescript
server: {
  port: 3000,
  proxy: {
    '/api': {
      target: 'http://localhost:8001',   // ← 改为 Rust 版
      changeOrigin: true,
    },
  },
},
```

重启 Vite 开发服务器：

```powershell
cd D:\YCYD\metaclouds-frontend-vue
npm run dev
```

### 1.3 生产环境

生产环境不使用 Vite 代理，而是通过 Nginx 反向代理：

```nginx
# /etc/nginx/conf.d/metaclouds.conf
location /api/ {
    proxy_pass http://rust-backend:8000/api/;   # ← 从 go-backend:8000 改为 rust-backend:8000
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    # JWT Cookie 需要
    proxy_cookie_path / /;
}
```

或通过环境变量注入（推荐）：

```nginx
location /api/ {
    proxy_pass http://${BACKEND_UPSTREAM}/api/;
    ...
}
```

---

## 2. 契约一致性（前端无需修改业务代码）

### 2.1 响应信封

Rust 版与 Go 版使用**完全相同**的 JSON 信封：

```json
// 成功
{
  "success": true,
  "data": { ... },
  "timestamp": 1789699958
}

// 失败
{
  "success": false,
  "message": "resource not found",
  "code": "NOT_FOUND",
  "timestamp": 1789699958
}
```

前端 `http.ts` 响应拦截器已统一解包 `{success, data, ...}`，Rust 版无需调整。

### 2.2 错误码（9 类，逐字一致）

| code 字符串 | HTTP 状态 | 说明 |
|-------------|-----------|------|
| `BAD_REQUEST` | 400 | 请求参数错误 |
| `UNAUTHORIZED` | 401 | 未认证 / Token 过期 |
| `FORBIDDEN` | 403 | 权限不足 |
| `NOT_FOUND` | 404 | 资源不存在 |
| `CONFLICT` | 409 | 资源冲突（重复） |
| `INTERNAL_SERVER_ERROR` | 500 | 服务器内部错误 |
| `VALIDATION_ERROR` | 400 | 参数校验失败 |
| `RATE_LIMIT_EXCEEDED` | 429 | 限流 |
| `SERVICE_UNAVAILABLE` | 503 | 服务不可用 |

### 2.3 分页参数

- 请求参数：`?page=1&page_size=10`（与 Go 版一致）
- 响应体：`{ data: [...], total: N, page: 1, page_size: 10, total_pages: M }`

### 2.4 认证机制

| 项目 | 实现 | 前端是否需改 |
|------|------|-------------|
| JWT 存储 | HttpOnly Cookie（`access_token`） | 否（自动携带） |
| CSRF 令牌 | Cookie（`csrf_token`）+ `X-CSRF-Token` 请求头 | 否（http.ts 已自动注入） |
| 登录响应 | `{success, data: {token, user, expires_at}}` | 否 |
| 401 处理 | 响应拦截器自动跳登录页 | 否 |
| 登出 | `POST /auth/logout` 清除 Cookie | 否 |

### 2.5 RBAC 权限名

30 个权限字符串与 Go 版逐字一致：

```
cluster:read, cluster:write,
resource:read, resource:write,
job:read, job:write, job:submit,
tenant:read, tenant:write,
monitoring:read, monitoring:write,
acceleration:read, acceleration:write,
security:read, security:write,
gpu:read, gpu:write,
partition:read, partition:write,
quota:read, quota:write,
scheduler:read, scheduler:write,
topology:read, topology:write,
dataset:read, dataset:write,
checkpoint:read, checkpoint:write,
admin（超级管理员，短路放行全部）
```

前端权限判断逻辑（v-if 指令 / 路由守卫）无需修改。

---

## 3. 已知差异

### 3.1 端点差异

| 差异项 | Go 版 | Rust 版 | 影响 |
|--------|-------|---------|------|
| 根级健康检查 | `GET /health` | `GET /health`（根级，无 JWT） | 已对齐；K8s 探针用 `/health`；前端无需调用 |
| 配额校验 | `POST /quotas/check` | `POST /quotas/{id}/check` | **P4-01 正在修复**，修复后路径一致；当前前端需注意路径 |
| GPU 资源端点 | `GET /resources/gpu` | 未实现 | 前端如有调用需改用 `GET /gpus` |
| 作业提交 | `POST /jobs/:id/submit` | 未实现（用 `POST /jobs` 创建） | 前端如有单独提交按钮需适配 |
| 集群状态 | `GET /clusters/:id/status` | 未实现（用 `GET /clusters/:id`） | 前端如有独立状态轮询需适配 |

### 3.2 Rust 版独有端点

以下端点 Rust 版提供，Go 版可能未实现或路径不同：

| 端点 | 说明 |
|------|------|
| `GET /jobs/stats` | 作业统计信息 |
| `GET /alerts` | 告警列表 |
| `GET /alerts/stats` | 告警统计 |
| `POST /alerts/:id/acknowledge` | 确认告警 |
| `POST /alerts/:id/resolve` | 解决告警 |
| `PUT /auth/change-password` | 修改密码 |
| `GET /auth/csrf` | 获取 CSRF 令牌 |
| `GET /monitoring/dashboard` | 监控 Dashboard 数据 |
| `GET /monitoring/metrics` | 监控指标 |
| `GET /monitoring/alert-rules` | 告警规则列表 |
| `POST /monitoring/alert-rules/evaluate` | 触发告警规则评估 |
| `POST /schedulers/:id/sync` | 同步调度器资源 |
| `POST /schedulers/:id/test-connection` | 测试调度器连接 |
| `POST /acceleration/:id/start` | 启动加速套件 |
| `POST /acceleration/:id/stop` | 停止加速套件 |
| `POST /security/policies/:id/enable` | 启用安全策略 |
| `POST /security/policies/:id/disable` | 禁用安全策略 |

### 3.3 别名路径（前端兼容用）

Rust 版额外提供了 Vue3 风格的别名路径，与主路径指向同一 handler：

| 主路径 | 别名路径 |
|--------|----------|
| `/topology` | `/topology/nodes` |
| `/gpus` | `/gpu/devices` |
| `/gpus/allocations` | `/gpu/allocations` |
| `/gpus/utilization` | `/gpu/utilization` |

前端可继续使用任一命名风格。

---

## 4. 前端配置切换步骤

### 4.1 开发环境切换

1. **修改代理目标**（`vite.config.ts`）：
   ```typescript
   proxy: {
     '/api': {
       target: 'http://localhost:8001',  // Go: 8000 → Rust: 8001
       changeOrigin: true,
     },
   }
   ```

2. **重启 Vite 开发服务器**：
   ```powershell
   # 如果有运行中的 dev server，Ctrl+C 停止后重启
   npm run dev
   ```

3. **验证清单**：
   - [ ] 打开浏览器，访问 `http://localhost:3000`
   - [ ] 使用 `admin` / `Admin@123456` 登录成功
   - [ ] Dashboard 页面正常加载（用户数、集群数等指标显示）
   - [ ] 用户列表页正常显示
   - [ ] 资源列表页正常显示
   - [ ] 集群列表页正常显示
   - [ ] 作业列表页正常显示
   - [ ] GPU 列表页正常显示
   - [ ] 创建/编辑/删除操作正常（注意 CSRF 令牌自动注入）
   - [ ] 浏览器 DevTools → Application → Cookies 中可见 `access_token`（HttpOnly）和 `csrf_token`

### 4.2 生产环境切换

1. **修改 Nginx 配置**，将 `/api/` 反向代理从 Go 版后端改到 Rust 版后端
2. ** reload Nginx**：`nginx -s reload`
3. **验证**：登录、Dashboard、各业务模块 CRUD

### 4.3 回滚步骤

如需切回 Go 版：

1. **开发环境**：`vite.config.ts` proxy target 改回 `http://localhost:8000`，重启 dev server
2. **生产环境**：Nginx upstream 改回 Go 版后端，reload
3. **确认**：登录、Dashboard 正常

---

## 5. 常见问题

### Q: 切换后登录返回 401？
A: 确认 Rust 版服务已启动且 `JWT_SECRET` 已设置（≥32 字符）。浏览器中旧的 Cookie 可能与新密钥不匹配，清除浏览器 Cookie 后重新登录。

### Q: 切换后写操作返回 403？
A: 检查 CSRF 令牌。前端 `http.ts` 会自动从 Cookie 读取 `csrf_token` 并注入 `X-CSRF-Token` 头。确认浏览器中存在 `csrf_token` Cookie（登录响应自动设置）。

### Q: 切换后页面白屏或报错？
A: 检查浏览器 DevTools Network 面板：
- 请求是否到达 Rust 版后端（状态码是否为 200）
- 响应信封是否为 `{success: true/false, ...}` 格式
- 如有 404，确认路径是否在 Rust 版路由表中（参考 `docs/api-reference-rust.md`）

### Q: Rust 版和 Go 版可以同时运行吗？
A: 可以。Rust 版默认监听 8001，Go 版监听 8000。通过修改 Vite proxy target 即可在两者间切换，无需停止任一服务。
