# Metaclouds 发布说明 · v2.0.1

- **发布日期**：2026-10-26
- **版本类型**：补丁版本（Bug Fix）
- **对应提交**：`5dc6c03` — `fix(frontend): 本地 dev 代理锁定 127.0.0.1:8000 并绑 IPv4，修复 /api 代理 500 致登录被锁`
- **Git Tag**：`v2.0.1`（指向 `5dc6c03`）

---

## 本次修复：本地开发环境登录失败并被锁定

### 现象
本地执行 `npm run dev` 启动前端后，所有 `/api` 请求经 Vite 代理返回 **500**，导致登录必然失败；连续失败触发前端 `Login.vue` 的 5 次失败 / 60 秒 `isLocked` 客户端锁，登录页提示"**账号已锁定**"，无法登录。

### 根因
- `vite.config.ts` 的默认代理目标曾使用 `localhost`，在 Windows 下 `localhost` 优先解析为 IPv6 地址 `::1`，而 Rust 后端仅以 `SERVER_HOST=127.0.0.1` 监听 IPv4。
- `localhost` → `::1` 的连接被拒（`AggregateError [ECONNREFUSED]`），Vite 代理整体失败，所有 `/api` 返回 500。
- 叠加因素：本机 `.env.development` 曾将 `VITE_API_PROXY_TARGET` 覆盖为 `http://localhost:8001`（端口错误），已本地修正；该文件被 gitignore，不入库。

### 修复内容
- Vite 默认代理目标改为 `http://127.0.0.1:8000`。
- 显式设置 `server.host: '127.0.0.1'` 强制 IPv4 绑定，从根上消除 IPv4/IPv6 错位。

### 影响范围
- **仅影响本地开发环境**（`npm run dev`）。
- Docker / 生产环境经 nginx 反代 `backend:8000`（容器 IPv4），不受此问题影响。

---

## 升级 / 部署说明
- 本次仅改动前端 `metaclouds-frontend-vue/vite.config.ts` 的代理配置默认值；**无需数据库迁移、无需后端改动**。
- 本地开发者拉取后若仍遇登录问题，请确认本机 `.env.development` 中 `VITE_API_PROXY_TARGET` 未被覆盖为错误端口（应指向 `http://127.0.0.1:8000` 或留空由默认值兜底）。
- 登录地址：`http://127.0.0.1:3000/`，默认账号 `admin` / `Admin@123456`。

---

## 文件清单
| 文件 | 变更 |
|------|------|
| `metaclouds-frontend-vue/vite.config.ts` | 默认 `proxyTarget` 改 `127.0.0.1:8000`；`server.host` 设 `127.0.0.1`（+10 / −1） |

## 验证
- 经代理 `POST /api/v1/auth/login` → 200 + token。
- 真实浏览器端到端：填入 `admin` / `Admin@123456` → 跳转 `/dashboard`、`.mc-topbar` 出现、`lockedText=false`、`errorText=false`、写入 `access_token`+`csrf_token` Cookie，零 console/page/net 错误、零 4xx/5xx。
