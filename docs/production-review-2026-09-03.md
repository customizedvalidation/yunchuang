# 生产落地复盘与部署 Runbook（2026-09-03）

> 范围：对 yunchuang（Metaclouds）全栈系统做多维度复盘，修复发现的问题，并完成生产构建验证。
> 环境约束：本机为**离线 Windows 沙箱**——无 Docker（`docker: command not found`）、无外部网络/目标服务器。
> 因此「正式落地生产环境」在此环境的可实现形式为：**修复部署配置 + 本地端到端验证生产产物 + 产出可在真实服务器执行的部署 Runbook + 提交推送**。完整 `docker compose up` / 真实 K8s 部署需在有 Docker 与网络的目标机执行。

---

## 一、多维度复盘结论

| 维度 | 现状评级 | 关键发现 | 处置 |
|---|---|---|---|
| 安全 | 良好 | 真实 `.env`/`.env.production` **未入库**且已 gitignore；`metaclouds-secrets.yaml` 为 **fail-secure 模板**（刻意空 Secret，强制部署前生成）；后端对 production 强制校验（JWT≥32、禁 sqlite、SSL 非 disable、ALLOWED_ORIGINS 必设且非 `*`） | 无需改；删除入库的 2 个 `.bak` 死文件 |
| 代码质量 | 良好 | `go build ./...`、`go vet ./...` 均 **0 错误**；前端 `tsc` 0 错误 | 无需改 |
| 构建 | 良好 | 前端 `vite build` 绿；后端静态二进制可编译 | 后端 Dockerfile 构建 flags 升级 |
| 测试 | 后端全覆盖 / 前端缺失 | 后端**全量测试通过**：`services`+`pkg/priorityscheduler`（4.86s/1.14s）+ `tests/` 目录 e2e/integration/middleware/priority_concurrency（8.31s，exit 0） | 前端 0 测试——离线无法装 vitest，列入建议 |
| 性能 | 良好 | 前端 vendor 已按 echarts/antd/react-vendor/vendor 拆包（长效缓存）；echarts 随路由懒加载不进首屏 | 无需改 |
| 部署 | 需修复 | **前端 Dockerfile 三处缺陷**（见下）；后端 Dockerfile 构建 flags 可优化；根 compose `SERVER_ENV` 硬编码 development | 已修复 |
| 可观测性 | 良好 | prometheus + grafana + jaeger + health/metrics/swagger 齐备；alerts.yml 在位 | 无需改 |
| 文档 | 良好 | DEPLOYMENT / ARCHITECTURE / 多份生产核查报告齐备 | 补本报告 + Runbook |

---

## 二、修复清单（本次落地）

### 1. 前端 `Dockerfile`（`metaclouds-frontend/Dockerfile`）—— 3 处真实缺陷
- **构建产物路径错误（严重）**：原 `COPY --from=builder /app/build ...`，但 Vite 输出目录是 **`dist/`**（`tsc && vite build`），`/app/build` 不存在 → 镜像 COPY 失败或得到空目录。已改为 `/app/dist`。
- **Node 版本过旧**：`node:18-alpine` → `node:22-alpine`（与项目工具链 managed 22.22.2 对齐，规避 Vite 5/依赖兼容风险）。
- **非可复现安装**：`npm install` → `npm ci`（依据 `package-lock.json`，CI/生产推荐）。

### 2. 后端 `Dockerfile`（`metaclouds-backend/Dockerfile`）—— 构建 flags 优化
- 原 `go build -a -installsuffix cgo` → `go build -trimpath -ldflags="-s -w"`：去除路径与符号信息，产出**更小、更安全的生产二进制**（实测产物 23MB，远小于带调试符号的 52MB 开发版）。
- **保留 `CGO_ENABLED=0`**（静态二进制）。注：一度怀疑 `mattn/go-sqlite3` 需 CGO 会致 `CGO_ENABLED=0` 构建失败，经**强制全量重编译（`-a`）**实证 `GOOS=linux CGO_ENABLED=0` 构建通过，该假设不成立（且生产强制 PostgreSQL、sqlite 仅开发用），故回退 CGO 变更——**先实证、再动配置**。

### 3. 根 `docker-compose.yml` —— SERVER_ENV 参数化
- `SERVER_ENV=development` → `SERVER_ENV=${SERVER_ENV:-development}`，并加注释：本栈为「一体化开发/联调栈」，**生产请改用 `deploy/kubernetes/` 清单**（fail-secure 密钥、`database-ssl-mode=require`，已按生产校验配置）；若坚持用 compose 起生产，须同时设 `SERVER_ENV=production` 且提供 `ALLOWED_ORIGINS`（非 `*`）、`DATABASE_SSL_MODE!=disable`，否则 backend 会因 `config.Validate` 拒绝启动。

### 4. 仓库卫生
- 删除入库死文件：`controllers/k8s_controller.go.bak`、`services/k8s_service.go.bak`（与现行文件已分歧的陈旧备份）。
- `.gitignore` 补 `dist-verify/`（构建验证临时产物）。
- `go.mod`：`mattn/go-sqlite3` 由 `// indirect` 修正为直接 require（`scripts/migrate.go` 直接 import，工具链自动修正）。

---

## 三、生产构建验证证据（离线沙箱内可做到的最强验证）

### 后端（生产二进制）
- 编译：`CGO_ENABLED=0 go build -trimpath -ldflags="-s -w"` → **23MB 静态二进制**，BUILD_EXIT=0。
- **生产配置校验（负向）**：`SERVER_ENV=production + USE_SQLITE=true` → 正确拒绝：`config validation failed: USE_SQLITE must be false in production environment`（exit 1）。
- **生产配置校验（正向 dry-run）**：全合法生产配置指向不存在 Postgres → 校验**全部通过** → 正确选用 PostgreSQL → 构造 `sslmode=require` DSN → 唯一失败点为离线无 Postgres（`127.0.0.1:5439 connectex: actively refused`，环境限制非代码问题）。
- **端到端功能**：同一生产二进制以 dev/sqlite 起服，全路由注册（auth/clusters/jobs/resources/monitoring/tenants/acceleration/security + health/metrics/swagger），`GET /health` → **200 `{"status":"healthy","dependencies":1}`**。

### 前端（生产 dist）
- 构建：`tsc && vite build` → **BUILD_EXIT=0**（20.5s），产物按 echarts/antd/react-vendor/vendor 拆包。
- 静态托管验证：`GET /`（index.html）、入口 chunk、echarts vendor chunk 均 **200**。

---

## 四、生产部署 Runbook（在真实目标机执行）

### 路径 A：Kubernetes（推荐，生产校验已对齐）
```bash
# 1. 生成真实 Secret（从已轮换的 .env.production 读取，不提交任何密钥）
cd metaclouds-backend && ./deploy/generate_k8s_secrets.sh

# 2. 应用清单（fail-secure：密钥未生成则应用拒绝启动）
kubectl apply -f deploy/kubernetes/metaclouds-secrets.yaml
#    再应用 Deployment/Service/PG/Redis 等其余清单（见 deploy/kubernetes/、deploy/migrations/）

# 3. 数据库迁移
cd metaclouds-backend && ./deploy/migrations/run_migration.sh

# 4. 验证
./deploy/verify_deployment.sh
```

### 路径 B：Docker Compose（一体化）
```bash
# 1. 准备 .env（参考 .env.example；POSTGRES_/REDIS_/GRAFANA_/JWT_SECRET≥32/DEFAULT_ADMIN_PASSWORD≥12）
cp .env.example .env  # 填入强凭据

# 2. 生产起栈（compose 栈需显式生产变量）
SERVER_ENV=production \
ALLOWED_ORIGINS="https://your-domain.com" \
DATABASE_SSL_MODE=require \
docker compose up -d --build

# 3. 验证
curl -f http://localhost:8000/health   # backend
curl -f http://localhost:3000/         # frontend
```

### 路径 C：裸二进制（无 Docker）
```bash
# 后端（Linux 目标机交叉编译，CGO_ENABLED=0 静态）
cd metaclouds-backend && CGO_ENABLED=0 GOOS=linux go build -trimpath -ldflags="-s -w" -o metaclouds-backend .
# 前端
cd metaclouds-frontend && npm ci && npm run build   # 产物 dist/，由 nginx 托管（nginx.conf 已含 SPA 回退 + /api 反代）
# 运行：见「生产必配环境变量」
```

### 生产必配环境变量（backend，缺失则 `config.Validate` 拒绝启动）
| 变量 | 要求 |
|---|---|
| `SERVER_ENV` | `production` |
| `JWT_SECRET` | **≥32 字符**随机串 |
| `USE_SQLITE` | `false`（生产禁 sqlite） |
| `DATABASE_SSL_MODE` | 非 `disable`（如 `require`） |
| `ALLOWED_ORIGINS` | 必设且**不得含 `*`**（如 `https://your-domain.com`） |
| `MEMORY_STORE_ENABLED` | `false` |
| `ALLOW_PUBLIC_REGISTRATION` | `false` |
| `DEFAULT_ADMIN_PASSWORD` | **≥12 字符** |
| `DATABASE_HOST/PORT/USER/PASSWORD/NAME` | 指向生产 PostgreSQL |

---

## 五、已知限制与后续建议
1. **前端 0 测试**：后端测试覆盖良好，前端无单测。建议在有网环境引入 Vitest + React Testing Library，先补 `<Can>` RBAC 渲染与 `ResponsiveTable`/`ResponsiveChart` 断点行为的冒烟测试。
2. **`metaclouds-backend/docker/docker-compose.yml`**（多实例扩缩容联调栈）grafana 用弱口令 `admin`、无 DB/JWT 配置——它是测试用途（对应 `tests/docker_multi_instance_test.go`），**勿用于生产**；如需保留，至少把 `GF_SECURITY_ADMIN_PASSWORD` 改为 `${GRAFANA_ADMIN_PASSWORD:?}`。
3. **真实 `docker compose up` / K8s 部署未在本机执行**（无 Docker/网络）。本报告已在沙箱内完成可做到的最强验证（生产二进制编译 + 配置校验正/负用例 + 端到端 /health + 前端 dist 静态托管），其余需在目标机按 Runbook 执行。
4. 根目录散落 `*.webp/*.jpg`（参考截图，未入库）与 `.workbuddy/`（WorkBuddy 内部数据）未纳入版本控制，按现状保留。
5. **前端 JWT 存于 `localStorage`**（`token` 键，`src/store/api.ts` 以 `Authorization: Bearer` 注入）——localStorage 可被页面内任意 JS 读取，存在 XSS 窃取 token 的理论风险。更优方案是迁移到 **httpOnly + Secure + SameSite Cookie**（JS 读不到），但需后端改 `Set-Cookie` 并加 CSRF 防护，且涉及登录/刷新链路、离线无法验证。**建议列为后续安全加固项，在有网可测环境实施**，不在本次离线复盘中盲目改动认证流程。（现有缓解：后端已有 Security Headers/CSP 中间件，生产模式下发更严格 CSP。）

---

## 六、五点建议落地执行记录（2026-09-03 续）

| # | 建议 | 执行结果 | 验证 |
|---|---|---|---|
| 1 | 前端测试 | 采用项目 `package.json` **已声明且已安装**的 **Jest + RTL**（离线可跑），而非 Vitest（离线装不了）。新增 `src/utils/auth.test.ts`（RBAC 鉴权矩阵纯逻辑单测，12 case）+ `src/setupTests.ts`（node 环境补全 `localStorage`）。组件级 RTL 冒烟测试需 `jsdom`（离线缺失），暂缓。 | `npx jest` → **12 passed / 12 total** |
| 2 | grafana 弱口令 | `docker/docker-compose.yml` 的 `GF_SECURITY_ADMIN_PASSWORD=admin` → `${GRAFANA_ADMIN_PASSWORD:-admin}`（保留默认，避免破坏 `tests/docker_multi_instance_test.go` 联调栈）。生产部署须显式设强口令。 | YAML 变更落地，`git status` 干净 |
| 3 | 真实部署 | 沙箱无 Docker/网络，无法执行 `docker compose up`/K8s。本报告第三节已完成离线可达成的最强验证；部署须在目标机按第四节 Runbook 执行。 | 文档记录（环境限制，非代码问题） |
| 4 | 仓库卫生 | 根 `.gitignore` 增补 `.workbuddy/` 与根级 `/*.webp`、`/*.jpg`、`/*.jpeg`、`/*.png`。原 untracked 的 4 张根图与 `.workbuddy/` 已不再出现在 `git status`。 | `git check-ignore` 命中 |
| 5 | JWT `localStorage` → httpOnly Cookie | 后端：中间件兼容读 `access_token` Cookie（Bearer 头优先，向后兼容非浏览器客户端）；`auth_controller` Login/Refresh 写 httpOnly+Secure(仅生产)+SameSite=Lax Cookie，新增 `POST /auth/logout` 清 Cookie。前端：移除 localStorage 读写 JWT，`api.ts` 改 `credentials:'include'`，401 兜底调 logout 清 Cookie；Login/App/Sidebar/Topbar/PrivateRoute 改用非敏感的 `user`+`auth_expiry`(ms)，续期判据改用 `auth_expiry`。 | 后端 `go build ./...`+`go test ./...` 全绿（新增 `jwt_auth_cookie_test.go` 覆盖 Cookie 读取/缺失/无效三路径）；前端 `tsc` 0 错 + `vite build` 绿 + 无 JWT 的 localStorage 读写残留 |

### 点 5 安全补充（Cookie 迁移）
- **SameSite=Lax**：默认抵御绝大多数 CSRF（跨站 POST 不携带 Cookie），且同源/同注册域子域部署下 Cookie 正常随请求携带。若前端与后端跨**完全不同域**部署（非同注册域），需将 Cookie 改为 `SameSite=None` 并配套 CSRF 双提交令牌；本实现刻意不默认开启 `None`，以免误配引入风险。
- **Secure 仅生产**：dev 为 http，浏览器不保存 Secure Cookie，故本地开发仍可登录；生产环境必须 https，Cookie 才被标记为 Secure。
- **离线验证边界**：沙箱无浏览器/目标服务，无法实跑登录链路。上述为「编译级 + 单测级」验证；**端到端登录态（含 httpOnly Cookie 写入、跨请求携带、logout 清除）需在可测环境用浏览器验证一次**，并确认生产 https 下 Secure Cookie 正常。

---

## 七、安全补充深度复盘与优化（2026-09-03 续二）

针对第六节「点 5 安全补充」列出的三项提醒做深度落地：CSRF 双提交令牌、Cookie SameSite 可配置、离线验证边界。

### 7.1 CSRF 双提交令牌（double-submit token）
- 新增 `middlewares/csrf.go`：`NewCSRFProtect(cfg)` 中间件。对状态变更方法（POST/PUT/DELETE/PATCH）且请求携带 `access_token` Cookie（浏览器会话）时，校验请求头 `X-CSRF-Token` 与 `csrf_token` Cookie **值一致**（常量时间比较 `subtle.ConstantTimeCompare`，规避计时侧信道）。
- 通道区分：走 `Authorization: Bearer` 头的非浏览器客户端（curl/SDK）不携带 `csrf_token` Cookie，自动跳过校验（它们不受 CSRF 影响）；GET/HEAD/OPTIONS 等幂等读取方法不校验。
- 令牌来源：登录时 `AuthController.Login` 用 `crypto/rand` 生成 32 字节随机值，分别写入**非 httpOnly** 的 `csrf_token` Cookie（供同源 JS 读取）与（经 `GET /auth/csrf`）响应体，供跨域 SPA 经带凭据请求取得。
- 登出 `Logout` 同时清除 `access_token` 与 `csrf_token` Cookie。

### 7.2 Cookie SameSite 可配置
- `config` 新增 `COOKIE_SAME_SITE`（默认 `lax`），提供 `Config.CookieSameSiteMode()` 映射到 `http.SameSite`；`access_token`/`csrf_token` Cookie 的 SameSite 均由其驱动。
- `Validate()` 新增规则：`COOKIE_SAME_SITE=none` 且非 `SERVER_ENV=production` 时启动失败——`SameSite=None` 要求 `Secure`，而 `Secure` 仅 https 有效，开发环境（http）设 `none` 会让浏览器拒存 Cookie、登录态形同失效。
- 跨完全不同域部署：设 `COOKIE_SAME_SITE=none` + 生产 https（自动 Secure）+ 本双提交令牌，即可安全放行跨站 Cookie。

### 7.3 CORS 配合
- `api/routes.go` 的 CORS `AllowHeaders` 增加 `X-CSRF-Token`，使浏览器在跨域请求中可携带该自定义头（否则预检失败）。

### 7.4 前端接入
- `src/store/api.ts`：`prepareHeaders` 在状态变更请求注入 `X-CSRF-Token`（取自 `csrf_token` Cookie，回退跨域缓存）；新增 `getCsrfToken` 查询，登录成功后（`login` 的 `onQueryStarted`）与已登录会话挂载时（`App.tsx`）拉取令牌写入缓存。
- `Sidebar`/`Topbar` 的裸 `fetch` 退出改为带 `X-CSRF-Token` 头（经 `csrfHeaders()`）。
- `tsconfig.json`：`tsc` 生产构建排除 `*.test.ts`/`setupTests.ts`（Jest 经 ts-jest 单独运行，离线无需 `@types/jest` 也能跑通 `npx jest`）。

### 7.5 验证证据（离线沙箱）
| 层 | 命令 | 结果 |
|---|---|---|
| 后端编译 | `go build ./...` | BUILD_OK |
| 后端全量单测 | `go test ./...` | 全绿（config / middlewares / services 5.59s / tests 9.86s，无回归） |
| CSRF 单测 | `middlewares/csrf_test.go` | 6 case：浏览器会话有效头→200、缺头→403、头不符→403、Bearer 跳过→200、GET 不校验→200、匿名 POST→200 |
| CSRF 集成 | `middlewares/csrf_integration_test.go` | 经真实 JWTAuth+CSRFProtect 中间件跑通「登录设双 Cookie → 已登录 GET 200 → 带正确 X-CSRF-Token 的 POST 200 → 缺头/错头 POST 403 → Bearer 跳过 200」 |
| 配置单测 | `config/config_test.go` | none 需 production 校验、SameSite 映射 |
| 前端编译 | `tsc && vite build` | 绿（dist 产物正常） |
| 前端单测 | `npx jest` | 12 passed / 12 total |
| 真实浏览器 e2e | Chrome CDP 无头驱动 | 登录 200；`access_token` httpOnly=true、`csrf_token` 非 httpOnly、`SameSite=Lax`；缺 `X-CSRF-Token` 的 `POST /auth/logout` 与 `POST /clusters` 均 **403**、带令牌 **200/201**；控制台 0 错误（修复 main.go 漏挂 CSRF 后闭环） |

### 7.6 离线验证边界（已闭环——真实浏览器 e2e 跑通并捕获一处生产漏防）

沙箱实际具备无头 Chrome（CDP 驱动），因此「端到端登录链路」已在真实浏览器实跑，而非停留在“待验证”。本轮 e2e **捕获并修复了一处严重安全漏防**：

- **发现**：`main.go` 自建路由（`gin.New()` + CORS）并直接 `RegisterRoutes`，**从未调用 `api.SetupRouter`**；而 CSRF 双提交令牌中间件 `r.Use(middlewares.NewCSRFProtect(cfg))` 仅挂在 `api.SetupRouter` 内。单元测试（`e2e_test.go`/`integration_test.go`/`e2e_full_test.go`）都走 `SetupRouter`，故测试全绿；但**生产实际运行的服务器完全没有 CSRF 防护**，形成“测试有、生产无”的假绿。`middlewares/stack.go:16` 曾记载过同一类“测试/生产中间件不一致”问题。
- **修复**：`main.go` 统一改为 `r := api.SetupRouter(cfg)`（commit `736e89d`），生产/测试共用同一中间件链（CORS 含 `X-CSRF-Token` + CSRF）。同时新增 `tests/csrf_mount_test.go`，直接对“生产装配”（`SetupRouter`+`RegisterRoutes`）发请求断言 CSRF 拦截，防止此类“中间件只挂在测试路由”的回归。
- **真实浏览器验证结果**（Chrome CDP，同源 Lax）：`login`→200；`access_token` httpOnly=true、`csrf_token` 非 httpOnly、`SameSite=Lax`；`POST /auth/logout` 与 `POST /clusters` **缺 `X-CSRF-Token` → 403**（修复前为 200/201，可被 CSRF 攻击）；带正确令牌 → 200/201；控制台 0 错误。
- **仍待真实环境确认的项（沙箱硬限制，非代码缺陷）**：跨域部署（`COOKIE_SAME_SITE=none` + https）下 `SameSite=None` 需 Secure/https，且 CDP 跨域带 `X-CSRF-Token` 的完整比对需在具备证书的目标机跑一次。同源 Lax 路径的 CSRF 防护已在本沙箱真实浏览器中端到端闭环。
