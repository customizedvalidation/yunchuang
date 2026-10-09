# 系统架构与菜单子模块复盘优化报告

> 日期：2026-10-09 ｜ 仓库根：`D:/YCYD`（monorepo）｜ 当前分支 `main`
> 技术栈（2026-10 现状）：前端 **Vue 3 + Element Plus + Pinia + Vue Router4 + ECharts + Vite5**（metaclouds-frontend-vue v2.0.0）；后端 **Rust**（axum + SQLx，SQLite/Postgres 双驱动但请求层仍为 SQLite 方言）；部署 docker-compose + GHCR 镜像 + k8s manifests；CI 根 `ci-cd.yml`。

---

## 0. 执行摘要

| 项 | 结论 |
|---|---|
| 本地↔GitHub 对齐 | 本地 `main` 领先 `origin/main` **8 个提交**；沙箱**离线**无法 fetch/push；工作树仅 2 个未跟踪文件（已清理 1 个，另 1 个建议纳入 CI） |
| 系统架构复盘 | 架构师只读复盘，识别架构问题 **A1–A10**（P0×1 / P1×7 / P2×2） |
| 菜单子模块优化 | 工程师落地 **P0-1 / P0-2 / P1-1 / P1-4 / P1-5 / P1-10 / P1-2 / P1-3 + A2 / A3**，QA 独立验证 **三道门禁全绿** |
| 验证结果 | `type-check` 0 错误 ｜ `vitest` **56 用例全过**（原 48 + 新增 nav 8）｜ `build` 0 错误 |
| 交付状态 | ✅ 可合入（行为保持，无回归）；架构级决策项留待拍板 |

---

## 1. 本地 ↔ GitHub 对齐

- **远程**：`origin` = `github.com/customizedvalidation/yunchuang.git`。
- **分歧**：`main` 领先 `origin/main` 8 个提交（含 全面复盘优化 / PG 方言移植 / CI 加固 / K8s 部署 / 多租户隔离 / 前端动态化 等），远端无本地缺失提交（基于上次 fetch 的陈旧引用，无法确认 GitHub 真实最新态）。
- **离线约束**：`git fetch/push/pull` 均 `Connection was reset`，**当前无法真正与 GitHub 同步**。
- **工作树漂移（已处理/建议）**：
  - `backend-run.log.err`（空日志产物，`.err` 后缀未被 `*.log` 忽略）→ **已加入 `.gitignore`（`*.err`）**。
  - `verify-k8s-rules.js`（k8s 镜像/规则校验脚本，有用）→ **建议纳入 CI 并在就绪后提交**。
- **安全确认**：`.env` 已被 gitignore 正确忽略；Rust 构建产物（target/）未跟踪（仅 225 个源文件被跟踪）。
- **建议**：网络恢复后 `git push origin main` 推送 8 个提交；将本次前端优化与 `verify-k8s-rules.js` 一并提交；删除死工作流 `ci-rust.yml` 已随本次优化完成。

---

## 2. 系统架构复盘（架构师：高见远）

> 级别说明：P0=架构级/高优；P1=中优（多数可安全立即实施）；P2=需拍板/架构决策。

| # | 级别 | 问题 | 影响 | 证据 |
|---|---|---|---|---|
| **A1** | **P0 架构** | **SQLite 单写者** → 后端 K8s 固定 `replicas:1` + Recreate + HPA 1/1，无 HA、不可水平扩展；Postgres 驱动已建但**请求层仍是 SQLite 方言**（`?N`/`last_insert_rowid`），连 PG 后 fail-closed 退出 | 生产单点故障；多副本即 `SQLITE_BUSY`/损坏 | `docker-compose.yml`、`05-deployment.yaml`、`main.rs`、`Cargo.toml` 双驱动但请求层未移植 |
| A2 | P1/CI | 死工作流 `metaclouds-backend-rust/.github/workflows/ci-rust.yml`（GitHub 不跑子目录 workflow，且与根工具链 `@stable` 不一致） | 误导维护者、行为分裂 | 文件存在 + `ci-cd.yml` 注释矛盾 |
| A3 | P1/CI | 前端单测 `continue-on-error:true` | 测试失败不拦合并/部署 | `ci-cd.yml` frontend-test |
| A4 | P1/部署 | 两套并行部署机制（GHCR 镜像 vs SSH 源码 `cargo build --release`） | 部署语义分裂、不可复现 | `ci-cd.yml` SSH 部署 vs 镜像构建 |
| A5 | P1/部署 | 镜像名硬编码 `ghcr.io/customizedvalidation/yunchuang/...` 与 `${{github.repository}}` 变量不一致 | 仓库名不符即 `ImagePullBackOff` | `ci-cd.yml` / `05-deployment.yaml` |
| A6 | P1/安全 | 限流 `RATE_LIMIT_ENABLED` 默认 `false` | 缺后端级抗爆破/抗滥刷 | `config.rs`、Login 仅前端 5 次/分 |
| A7 | P2 | 后端 `Cargo.toml` 仍自述 "PoC"，却已是唯一生产后端 | 风险认知不清 | `Cargo.toml` |
| A8 | P1/安全 | **角色三源 + 客户端角色可篡改**：`auth.ts` 把含 `role` 的 `user` 存 localStorage；Sidebar 用 `readStoredRole()`、路由守卫用 `auth.role`——伪造 localStorage 可骗过前端越权展示（真实授权在后端） | UI 越权展示、菜单/守卫可能不一致 | `auth.ts`、`utils/auth.ts`、`router`、`Sidebar.vue` |
| A9 | P2/一致性 | `tokens.ts`↔`index.css` 仅人工同步；`sidebar.css`/`Topbar.vue` 硬编码侧栏蓝未走 `--sidebar-primary-*` | 改一处漏改另一处、设计令牌未完整落地 | `sidebar.css`、`Topbar.vue`、`tokens.ts` |
| A10 | P2/主题 | 仅 light 主主题 + 常驻深色侧栏，**无完整 dark theme**（`tokens.ts` 已预留） | 深浅色不完整 | `index.css`、`tokens.ts` |

---

## 3. 菜单子模块复盘与已落地优化

### 3.1 复盘结论
- **导航三源**：`Sidebar.menuItems` / `router.meta` / `Topbar.titleMap` 各自维护，新增模块需改 ≥2 处，角色在两处定义 → 漂移风险。
- **徽标语义错误**：`/k8s/pods` 徽标用"运行中作业数"顶替（后端无真实 Pod 接口）；徽标仅 `onMounted` 加载一次 → 长期陈旧。
- **`Layout.vue` `:key="route.fullPath"`**：子路由（如 `/job/list↔/queue`）切换时整组件重挂载 → 重复全量请求、重置分页/搜索/轮询。
- **a11y**：侧栏父菜单项是不可聚焦 `<div>`（无 `tabindex`/`role`），键盘不可用。
- **页面重复**：自动刷新逻辑 ~40 行在 6 页复制；status 映射重复；多租户页未用共享 `useFetch`。
- **嵌套路由非空壳**：Job/K8S 组件确实按 `route.path` 分支渲染（已核实，非空壳）。
- **设计系统**：`mc-*` 类、`tokens.ts`、`index.css` 主色/语义色抽查一致；问题在硬编码侧栏色 + 无 dark theme。

### 3.2 已落地优化（工程师：寇豆码 ｜ QA：严过关 复核 PASS）

| 编号 | 优化项 | 改动 | 状态 |
|---|---|---|---|
| **P0-1** | 统一导航配置源 | 新建 `src/nav.ts`（单一 `navConfig` + `getNavRoles/getNavTitle/findNavItem`）；`Sidebar`/`Topbar`/`router` 守卫改读 `navConfig`，消除三源 | ✅ |
| **P0-2** | 修正 `/k8s/pods` 徽标语义 | 移除误导的"运行中作业数"徽标与 `badgeColor` 的 `success` 分支（不捏造数据） | ✅ |
| **P1-1** | 去除子路由重挂载 | `Layout.vue` `:key` 改为顶层路由段；同级子路由切换不重挂载、跨顶层仍重挂载（Job/K8S 的 `activeTab` 为 `computed(route.path)`，无需加 watch） | ✅ |
| **P1-4** | 徽标定时刷新 | `Sidebar` `loadBadges` 由单次 → `setInterval(30s)` 并 `onUnmounted` 清理 | ✅ |
| **P1-5** | 侧栏 a11y | 父菜单项加 `tabindex="0"`、`role="button"`、`:aria-expanded`、`@keydown.enter/space` 触发展开 | ✅ |
| **P1-10** | 角色来源单源 | `Sidebar.currentRole` 改用 `auth.role`（store），与路由守卫同源 | ✅ |
| **P1-2** | 抽取重复逻辑 | 新增 `src/composables/useAutoRefresh.ts` + `src/utils/status.ts`，应用于 **Job / K8S / MultiTenant** 三页（其余 10 页因细微差异暂缓，避免行为漂移） | ⚠️ 部分 |
| **P1-3** | 多租户统一取数 | `MultiTenantManagement.vue` 手写 loading/error 改 `useFetch` | ✅ |
| **A2** | 删死工作流 | 删除 `metaclouds-backend-rust/.github/workflows/ci-rust.yml` | ✅ |
| **A3** | 前端单测阻断 | `ci-cd.yml` frontend-test 移除 `continue-on-error:true` | ✅ |

### 3.3 验证（硬性门槛，全绿）
- `npm run type-check`（vue-tsc --noEmit）：**0 错误**
- `npm run test`（vitest run）：**56 passed / 11 files**（含新增 `src/__tests__/nav.test.ts` 8 例）
- `npm run build`（vite build）：**0 错误**，23.78s 产出 `dist/`

### 3.4 非阻断提示
- **P3（建议确认）**：`/tenant` 面包屑文案由"多租户管理"收敛为"多租户"（统一导航源的预期收敛）；如要求保留旧文案，改 `nav.ts` label 会同步改侧栏。
- **P2（注释）**：`router/index.ts` 守卫注释对"旧写法"描述与实际旧代码不符，建议修正措辞（不影响行为）。

---

## 4. 待用户拍板的架构决策（P2 / 架构级）

| 决策项 | 说明 | 建议 |
|---|---|---|
| **A1：PG 请求层方言移植** | 解锁多副本/HA、消除单点故障的**前提里程碑**；工作量中等（请求层 SQL 移植 + 迁移脚本） | 列为最高优先架构里程碑 |
| **A4/A5：部署形态统一** | 收敛到"GHCR 镜像 + k8s"或"系统服务"，弃用 SSH 源码构建；镜像名统一用 `${{github.repository}}` | 二选一并文档化 |
| **A6：生产开启限流** | `RATE_LIMIT_ENABLED=true`（compose 与 k8s 一致），保留登录防护 | 建议开启 |
| **A7：后端 PoC 毕业** | 补测试覆盖、移除 spike 自述、明确 SLA | 视资源 |
| **A8：角色防篡改** | 前端角色仅用于展示，关键在后端授权；可考虑不把 role 暴露在可篡改处 | 维持后端授权为真相 |
| **A9：token 一致性校验** | 加构建期/CI 校验或代码生成，消除 `tokens.ts`↔`index.css` 人工同步 | 可选 |
| **A10：完整 dark theme** | 补 `[data-theme="dark"]` 覆盖 `:root`（tokens 已预留） | 视产品需求 |
| **P1-2 扩展** | 将其余 10 页的自动刷新/状态映射也迁到 `useAutoRefresh`/`status.ts` | 逐个评估迁移 |

---

## 5. 后续建议 / 下一步

1. **网络恢复后**：`git add` 本次前端优化 + `verify-k8s-rules.js` + `*.err` 清理，提交并 `git push origin main`（含此前领先 8 个提交）。
2. **优先决策 A1（PG 移植）**：这是解除单副本单点、支撑 K8s 多副本 HA 的唯一路径。
3. **统一部署与镜像名**（A4/A5），消除 `ImagePullBackOff` 风险。
4. **确认 `/tenant` 面包屑文案**与是否推进 dark theme / P1-2 扩展。
5. 本次改动**未触碰任何后端 Rust 代码与 `.env`/compose/k8s 实质配置**（除删除死工作流），风险可控。

---

*本报告由软件公司专家团（主理人齐活林 / 架构师高见远 / 工程师寇豆码 / QA 严过关）协作产出。架构复盘与菜单优化部分已落地并验证；架构级决策项待用户拍板后进入实施。*
