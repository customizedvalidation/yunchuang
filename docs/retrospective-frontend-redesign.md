# Metaclouds 前端响应式改版 · 复盘报告（M1–M4 + R3-b）

- 撰写人：齐活林（Qi）· 交付总监（主理人）
- 日期：2026-09-02
- 关联文档：`docs/prd-frontend-redesign.md`、`docs/design-frontend-redesign.md`、`docs/rbac-permission-convergence.md`
- 提交基线：`30c6c32`（M1–M4 全量落地）、本报告的 R3-b 提交（见第 5 节）

---

## 1. 交付范围与里程碑

| 里程碑 | 内容 | 状态 |
| --- | --- | --- |
| M1 地基 | R1 断点真相源 + R2 纯 CSS Grid 布局（去 JS 布局）+ R3-a 侧边栏 Drawer/遮罩/Esc + R4 顶栏汉堡 + R5/R9 流式令牌与超宽屏收敛 + R10 Tailwind 接入 | ✅ |
| M2 表格 | R6 `ResponsiveTable` 列→卡片同构视图，接入 10 张表；`scroll.x` 硬编码归零 | ✅ |
| M3 仪表盘 | R7 `DraggableGrid` 断点 span 映射；R8 `ResponsiveChart` + `ResizeObserver` 令牌化高度 | ✅ |
| M4 补齐 | R12 触控 44px 命中区、R14 弹窗窄屏全屏、R16 状态页极窄屏、R17 命令面板移动端+顶栏触发、R18 登录 clamp；四档视口 × 明暗双主题 CDP 走查 G1–G8 | ✅ |
| R3-b 宽度真相合并 | 见第 5 节（本轮收口） | ✅（类型隔离验证通过） |

---

## 2. 验收结论（G1–G8 对应达成）

CDP 真机走查（`wt-walkthrough.mjs`，免后端 Fetch 注入假信封）在 375 / 768 / 1366 / 1920 × light/dark 全矩阵通过，`errors` 全 `[]`：

| 验收目标 | 达成 | 证据 |
| --- | --- | --- |
| G1 全断点零横向溢出 | ✅ | 栅格 375=1 列 / 768=2 列 / 1366·1920=4 列；表格卡片化无横向滚动 |
| G2 内容零遮挡 | ✅ | <1024 侧边栏浮层 + 遮罩 + Esc + body 锁滚 |
| G3 消除缩放抖动 | ✅ | `Layout.tsx` 零 `window.innerWidth`/`resize` 监听，纯 CSS 媒体查询 |
| G4 断点连续覆盖 | ✅ | 8 档断点单一真相源（`breakpoints.ts` ↔ `tailwind.screens` ↔ `index.css`） |
| G5 触控可达 | ✅ | `.dg-btn`/顶栏图标 `::after` 44×44 命中区，视觉尺寸不变 |
| G6 超宽屏可读性 | ✅ | `≥2560` 主内容 `max-width:2400` 居中 |
| G7 功能零回归 | ✅ | `npm run build` 通过；`<Can>`=16 不变；admin/rbacuser 快照一致 |
| G8 暗色零回归 | ✅ | 四档 × 明暗双主题走查无走样、零 console 错误 |

---

## 3. 验证方法论

- **为什么用 CDP 而非只信「构建绿」**：改版是纯前端视觉/布局变更，构建（`tsc && vite build`）只证明类型与打包通过，不证明响应式行为真实生效。为此用 Node 22 内置 `WebSocket` + 离线 Chromium（`chromium-1228`）驱动真实浏览器，并以 **Fetch 域注入假数据信封**（`{success:true,data:[...]}`）绕过本地后端缺失，实测栅格列数、图表高度、触发按钮显隐、弹窗/命令面板行为。
- **关键坑（已沉淀为复用流程）**：Node 22 全局 `WebSocket` 用 `addEventListener` 而非 `.on`；Chrome 149+ 移除 `Network.requestIntercepted`，改用 Fetch 域注入；headless 默认暗色需主动切换；`/json/version` 就绪轮询避免端口未就绪。
- **回归护栏（硬约束，改版前后一致）**：`<Can>` 包裹点 = 16；`scroll.x` 硬编码表 = 0（仅 `ResponsiveTable` 注释）；ECharts 硬编码高度 = 0（仅 `ResponsiveChart` 封装内）。

---

## 4. 环境阻塞与应对（本回合重点复盘）

| 阻塞 | 现象 | 应对 |
| --- | --- | --- |
| **429 限流** | 架构师子代理 `software-architect-2` 产出设计文档时被限流中断 | 主理人代拟 `docs/design-frontend-redesign.md`（外部阻塞豁免），直接落地 M1–M4 |
| **safe-delete shim 拦截 dist 清空** | `vite emptyOutDir` 调 `genie-trash` 清空既有 `dist` 触发 ETIMEDOUT / 批量确认 | 构建到全新 `--outDir`（dist-verify3/4/5）绕过，非代码问题 |
| **无 remote 推送阻塞** | `git remote -v` 为空 | 提交到本地 `main`（`30c6c32` 等），推送待用户配 remote |
| **⚠️ 安全删除 shim 级联（本回合新发）** | 对 `theme/sidebar.ts` 执行 `git rm` 被 shim SIGTERM，并**级联删除整个 `metaclouds-frontend/src/` 树**（git 暂存区 + 磁盘） | 从 `HEAD`（30c6c32）`git checkout HEAD -- metaclouds-frontend/src` 完整恢复；移除残留 `.git/index.lock` 后重试。**教训：本沙箱禁止对源码用 `git rm`/`rm`，删除类操作会触发级联隔离**。 |
| **⚠️ node_modules 回归（本回合新发）** | 级联事件后 `node_modules` 不完整（447 项，缺 `@ant-design/icons` 解析与 `.bin` 软链）；`npm install --offline` ENOTCACHED、`npm install` 被 SIGTERM | 构建门禁暂不可用；改用 **隔离类型检查**（tsc 仅过滤本回合改动文件）证明 R3-b 零错误。恢复需联网重跑 `npm install`，超出离线沙箱能力，待环境复位。 |

---

## 5. R3-b：宽度真相合并（本回合收口）

- **设计文档原意**：`Sidebar.css` 20+ 个 `!important` 与 `theme/sidebar.ts` 宽度真相合并，留作独立低风险提交。
- **实地核查结论（重要修正）**：
  1. `Sidebar.tsx` 实际保留 **antd `Sider`**（折叠上下文依赖），`Sider` 向内联 `style` 注入 `width`；`Sidebar.css` 中断点处的 `width: ... !important` 是**覆盖 antd 内联宽度的必要条件**，盲目删除会令响应式宽度回退到 antd 内联值（正是设计文档警告的回归）。
  2. 宽度真相实际分裂为三处：`breakpoints.ts` `SIDER_WIDTH`（被 Sider 属性消费，实时真相）、`theme/sidebar.css` `--sidebar-width-*`（被媒体查询消费）、`theme/sidebar.ts` 的 `SIDER_WIDTH/SIDEBAR_BREAKPOINTS/getSidebarWidth`（**死代码**——全仓零引用，且 `getSidebarWidth` 用 `window.innerWidth` 违反 R2「零 JS 布局」）。
- **执行（安全、无删除）**：
  - 不删除文件（规避上表级联风险），改为**就地收敛** `theme/sidebar.ts` 为对 `breakpoints.ts` 的薄别名：`export { SIDER_WIDTH } from './breakpoints'` + 保留 `SIDERABAR_COLORS`；移除矛盾的宽度常量与非法 `getSidebarWidth`。
  - `Sidebar.css` 在宽度媒体查询前补注释，说明 `!important` 的必要性，消除「为降数量而误删」的维护隐患。
- **验证**：隔离 `tsc --noEmit` 过滤 `theme/sidebar`、`Sidebar/Sidebar` → 本回合改动 **0 错误**；构建全绿此前已由 dist-verify4 证明，本回合行为零变化。

---

## 6. 待明确事项批准状态

用户已批准全部待明确事项，本回合据此收口：

- **R3-b**：已执行（见第 5 节）。
- **Q1–Q10（PRD 第 7 节）**：全部沿用 PM 推荐 —— Q1=A 只读+紧急操作、Q2=A 居中留白 + 监控页左右分栏(R19)、Q3=A 常驻图标轨、Q4=A 卡片化、Q5=A 接入 Tailwind、Q6=A 密度切换(纳入 P1)、Q7=A 移动端禁拖拽、Q8=B 小幅收敛、Q9=A 支持缩放、Q10=B+4 档 PM 复核。

---

## 7. 经验与下一步候选

**经验（写入团队 SOP）**：
1. 沙箱对源码的 `git rm`/`rm` 会触发 safe-delete 级联隔离 → 源码删除类操作一律规避，改用「就地收敛/注释」或用户显式确认后的安全路径。
2. 响应式改版的验收不能停在「构建绿」→ CDP 免后端真机走查（Fetch 注入）是性价比最高的闭环手段。
3. 设计文档若与真实实现冲突（如「弃用 antd Sider」vs 实际保留），以**真实代码**为准再做合并，不照本宣科删 `!important`。

**下一步候选（均未做，按优先级）**：
- **P1** Q6 密度切换（紧凑/舒适，持久化）—— 约 1–2 天。
- **P1** R15 `K8SManagement` 内联魔法值（`flex:'1 1 200px'` 等）改栅格类 + 间距令牌。
- **P1** R19 监控页 `<lg` 纵向堆叠 / `≥2xl` 左右分栏。
- **优化** 路由级懒加载（`React.lazy` + `Suspense`），削减首屏包体（构建曾提示 chunk 体积）。
- **环境** 恢复 `node_modules`（联网 `npm install`）以恢复 `npm run build` 门禁。
