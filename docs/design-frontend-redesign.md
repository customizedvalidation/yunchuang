# Metaclouds 算力调度平台 · 前端改版设计文档

| 项 | 内容 |
| --- | --- |
| 文档版本 | v1.0 |
| 撰写人 | 高见远（Gao）· 架构师（由主理人齐活林在 429 限流期代拟） |
| 日期 | 2026-09-01 |
| 状态 | 已落地 M1 地基，待 M2–M4 |
| 关联文档 | `docs/prd-frontend-redesign.md`、`docs/rbac-permission-convergence.md` |

> 说明：原架构师子代理因 429 限流（2026-09-02 14:35:48 UTC+8 重置）未能产出本设计文档，主理人基于已评审 PRD + 用户 4 项决策 + 已校正基线，代拟本文档以推进实现。

---

## 1. 实现方案与框架选型

- **不新增任何 npm 依赖**（离线环境硬约束）。继续沿用：React 18 + TS + Vite 5 + antd 5 + RTK Query + ECharts。
- **Tailwind 3（用户决策 Q5=A 接入）**：补 `@tailwind base/components/utilities` 指令，并 `corePlugins.preflight=false` 关闭 preflight，避免重置 antd 5（CSS-in-JS）基础样式。`PrivateRoute.tsx:57` 的失效类名因此真正生效（R10）。
- **布局范式：CSS Grid 替代 JS 布局计算**。外壳 `display:grid; grid-template-columns: var(--mc-sider-w) minmax(0,1fr)`；`--mc-sider-w` 由断点媒体查询与 `collapsed` 状态切换，组件内零 `window.innerWidth` / resize 监听（G3）。
- **侧边栏：栅格列（桌面）+ Drawer 浮层（移动）双形态**。≥1024 常驻（lg=64px 图标轨，xl+=240px 展开）；<1024 转 `position:fixed` 浮层 + 遮罩 + Esc + 锁定 body 滚动（G2）。
- **动效只动 transform / opacity / grid-template-columns**，缓动统一 `--mc-ease`，时长沿用 `--mc-t-*`（6.7）。

---

## 2. 文件清单（相对 `metaclouds-frontend/`）

| 文件 | 变更 | 里程碑 | 说明 |
| --- | --- | --- | --- |
| `src/theme/breakpoints.ts` | 新增 | M1 | 断点 + 侧边栏宽度单一真相源（TS） |
| `tailwind.config.js` | 改 | M1 | `screens` 对齐断点；`preflight:false` |
| `src/index.css` | 改 | M1/R5/R9 | 注入 Tailwind 指令；流式令牌；App Shell 网格；超宽屏收敛 |
| `src/components/Layout.tsx` | 改 | M1/R2/R3-a | 纯 CSS Grid 外壳；移除 JS 布局；移动浮层态 |
| `src/components/Sidebar/Sidebar.tsx` | 改 | M1/R3-a | 弃用 antd `Sider`，改 `<aside>` 栅格列 + `mobile-open` 类 |
| `src/components/Sidebar/Sidebar.css` | 改 | M1/R3-a | 桌面 sticky 栅格列；<1024 fixed Drawer + 隐藏折叠按钮 |
| `src/components/Topbar/Topbar.tsx` | 改 | M1/R4 | 新增 `onMenuClick`；插入汉堡触发按钮 |
| `src/components/Topbar/Topbar.css` | 改 | M1/R4 | `.mc-menu-trigger` 样式（<1024 显示） |
| `src/components/ResponsiveTable/*` | 新增 | M2/R6 | 表格列→卡片同构视图 |
| `src/pages/Dashboard.tsx` | 改 | M3/R7/R8 | DraggableGrid 断点 span；ECharts 令牌高度 + ResizeObserver |
| `src/components/DraggableGrid.tsx` | 改 | M3/R7 | span 断点映射 |
| `src/pages/MonitoringAlert.tsx` | 改 | M3/R8 | 图表令牌高度 |
| 9 张业务表（Job/Cluster/Resource/MultiTenant/K8S×3/Monitoring/Security/Acceleration） | 改 | M2/R6 | 接入 ResponsiveTable |
| 弹窗 4 个（Job/Cluster/MultiTenant/K8S） | 改 | M4/R14 | 窄屏全屏/底部抽屉 |
| `src/pages/Login.tsx` | 改 | M4/R18 | 卡片 clamp；去内联 `<style>` |
| `src/components/States/*` | 改 | M4/R16 | 骨架/空态窄屏缩放 |
| `src/components/CommandPalette/*` | 改 | M4/R17 | 移动端 100vw + dvh |

> **回归护栏（R11，硬约束）**：`Can.tsx` / `utils/auth.ts` / `PrivateRoute.tsx` / `App.tsx` 路由表 / `store/api.ts` 零改动。`<Can>` 包裹点 = **16**（已校正），`scroll.x` 固定横向滚动表 = **10 处**，ECharts 硬编码高度 = **3 处**，改版前后均须快照一致。

---

## 3. 数据结构与接口契约

### 3.1 断点真相源（`src/theme/breakpoints.ts`）
```ts
export const BREAKPOINTS = { xs:0, sm:576, md:768, lg:1024, xl:1280, '2xl':1440, '3xl':1920, '4xl':2560 } as const;
export const MOBILE_MAX = 1023;
export const SIDER_WIDTH = { expanded:240, collapsed:64, rail:64, drawer:280 } as const;
```
三处同步：TS 常量 ↔ `tailwind.config.js.screens` ↔ `index.css` 的 `@media` + `--mc-sider-w*`。

### 3.2 外壳组件契约
```ts
// Layout.tsx
<div className="mc-app-shell [is-collapsed] [is-mobile-open]">
  <Sidebar collapsed onCollapse mobileOpen />
  <div className="mc-app-main">
    <Topbar onMenuClick />
    <main className="mc-app-content">{children}</main>
  </div>
  <div className="mc-app-overlay" onClick={close} />
</div>

// Sidebar.tsx
interface SidebarProps { collapsed: boolean; onCollapse: (b:boolean)=>void; mobileOpen?: boolean }

// Topbar.tsx
const Topbar: React.FC<{ onMenuClick: () => void }> = ...
```

### 3.3 共享 CSS 令牌（已注入 `index.css`）
- 间距：`--mc-gutter / --mc-gap / --mc-section / --mc-measure / --mc-page-max`
- 栅格：`--mc-sider-w / --mc-sider-w-collapsed`
- 字号：`--mc-fs-display/metric/h1/h2/h3/body/sm/xs`（全部 `clamp()`）
- 图表：`--mc-chart-h-sm/md/lg`

---

## 4. 调用流程（外壳渲染）

```
浏览器 resize
  └─ (无 JS 监听) CSS 媒体查询切换 :root --mc-sider-w
       ├─ ≥1024：.mc-app-shell 网格列 1 宽度 = --mc-sider-w（240 / 64 / 64）
       └─ <1024 ：--mc-sider-w = 0；.metaclouds-sidebar 转 fixed + translateX(-100%)
用户点汉堡(Topbar) → onMenuClick → setMobileOpen(true)
  └─ .mc-app-shell.is-mobile-open → 遮罩 opacity:1 + .metaclouds-sidebar.mobile-open translateX(0)
  └─ useEffect：keydown Esc → close；body overflow:hidden
用户点遮罩 / Esc / 菜单项 → mobileOpen=false → 复位
```

---

## 5. 任务列表（有序 + 依赖）

### M1 · 地基（已落地 ✅）
- R1 断点真相源（breakpoints.ts + tailwind + index.css）✅
- R2 纯 CSS Grid 布局（去 JS 布局）✅
- R3-a 侧边栏 Drawer + 遮罩 + Esc（R3-b 清理 !important 留待后续）✅
- R10 Tailwind 接入 + PrivateRoute 类名生效 ✅
- R4 耦合项：Topbar 汉堡触发 ✅
- R5/R9 部分：流式 `.mc-page`/标题/描述 + 超宽屏收敛 ✅

### M2 · 页面模板与表格（已落地 ✅）
1. R4 收尾：Topbar 窄屏面包屑/搜索/环境标签降级策略打磨（依赖 M1）
2. R5 收尾：区块纵向节奏 `--mc-section` 应用到页面内分区（依赖 M1 令牌）
3. R6 新增 `ResponsiveTable`：columns→卡片同构；≥lg `scroll.x:'max-content'`，<md 卡片视图（依赖 M1 网格）
4. R6 接入 10 张表（基线已校正：Acceleration/Cluster/Job/K8S×3/Monitoring/MultiTenant/Resource/Security，共 10 处 `scroll.x`）
5. R9 正文 `max-width: var(--mc-measure)` 应用（依赖 M1）

### M3 · 仪表盘与图表
6. R7 DraggableGrid：`span` 断点映射 {xs:12,sm:12,md:6,lg:6,xl:3,2xl:3}（依赖 M1 断点）
7. R8 ECharts：`--mc-chart-h-*` + 原生 `ResizeObserver`（依赖 M1 令牌）

### M4 · 补齐与验收
8. R11 回归核验：`<Can>`=16、`scroll.x`=10、ECharts=3 快照一致
9. R12 触控 44px（`.dg-btn`/Topbar 图标按钮 `::after` 扩大命中区）
10. R14 弹窗窄屏全屏/底部抽屉
11. R16 States 骨架/空态窄屏
12. R17 CommandPalette 移动端
13. R18 Login 卡片 clamp + 去内联 `<style>`
14. 四档视口（375/1366/1920/3440）× 明暗双主题走查（G1–G8）

---

## 6. 依赖包

无新增依赖。`tailwindcss/postcss/autoprefixer` 已存在（死依赖→本里程碑激活）。

---

## 7. 共享知识（跨文件约定）

- **断点三源同步**：改 `breakpoints.ts` 必须同步 `tailwind.config.js.screens` 与 `index.css` 媒体查询，否则出现双真相（R1 根因）。
- **只动 transform/opacity/grid-template-columns**：新增动效不得引入 `width/margin/top/left` 过渡（侧边栏折叠除外），保持 `--mc-ease` 与 `--mc-t-*` 三档。
- **`min-width:0` 铁律**：每个新增 flex/grid 子项必须带 `min-width:0`，防长文本撑破（G1 最常见失败源）。
- **暗色双侧定义**：所有新增 CSS 变量须同时在 `:root` 与 `html[data-theme='dark']` 定义。
- **权限资产零改动**：`<Can>`=16 与 `hasPermission` 语义不得动；前端收敛纯 UX，后端 `RequirePermission` 为真边界。

---

## 8. 待明确事项

- R3-b：`Sidebar.css` 20+ 个 `!important` 与 `theme/sidebar.ts` 宽度真相合并，留作独立低风险提交（每步单独验证）。
- Q6 密度切换、Q7 移动端拖拽保留策略，按用户后续拍板纳入 P1。
- M2–M4 实现仍建议由工程师子代理执行；若 429 未重置，主理人可继续代拟实现。
