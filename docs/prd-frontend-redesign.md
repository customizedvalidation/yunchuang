# Metaclouds 算力调度平台 · 前端改版 PRD

| 项 | 内容 |
| --- | --- |
| 文档版本 | v1.0（初稿） |
| 撰写人 | 许清楚（Xu）· 产品经理 |
| 日期 | 2026-09-02 |
| 状态 | 待评审 / 含 10 项待确认问题 |
| 关联文档 | `docs/rbac-permission-convergence.md` |

---

## 0. 一句话定义

**在不丢失任何业务功能与 RBAC 权限收敛成果的前提下，把 Metaclouds 前端从「为 1440×900 单一分辨率写死」改造为「CSS 驱动的、覆盖 375–3440px 连续谱的响应式系统」，并把"国际一流水准"落成可测量的栅格、间距、字号、密度与动效规范。**

---

## 1. 产品目标

### 1.1 背景：用户需求原文

> "请将前端页面重新设计下，符合自适应不同分辨率的显示，整个版面具有国际一流水准。"

一句话里有两个诉求，本 PRD 分别对应：

- **诉求 A（功能）**：自适应不同分辨率 —— 可验证、可测量。
- **诉求 B（品质）**：国际一流水准 —— 主观，必须转译成客观设计约束（见第 6 节）。

### 1.2 现状核查结论（我已逐文件读码复核，非二手转述）

| # | 核查项 | 结论 | 证据位置 |
| --- | --- | --- | --- |
| 1 | 设计令牌 | 已存在且质量较好。CSS 变量（品牌/语义/中性/圆角/阴影/动效）+ `html[data-theme='dark']` 双主题齐全 | `src/index.css:5-88`、`src/theme/tokens.ts` |
| 2 | 全局断点 | **仅 1 处**：`@media (max-width: 768px)`，只把 `.mc-page` padding 24→16 | `src/index.css:232-236` |
| 3 | 布局驱动方式 | **JS 驱动**：`window.innerWidth` + `requestAnimationFrame` 监听 resize，手工算 `marginLeft`，内联 `transition: margin-left` | `src/components/Layout.tsx:14-19, 49, 59` |
| 4 | 布局缺陷（实测） | ① 窄屏强制 `setCollapsed(true)` 后**没有反向恢复**，窗口从 1000 拖回 1600 侧边栏仍折叠；② `marginLeft` 是 JS 状态，SSR/首帧与 resize 期间必然抖动；③ 侧边栏宽度有**两套真相**（`theme/sidebar.ts` 常量 vs `Sidebar.css` 的 CSS 变量 + `!important`） | `Layout.tsx:45-47`、`theme/sidebar.ts:1-31`、`Sidebar/Sidebar.css:309-437` |
| 5 | 表格 | **9 张表全部**用固定 `scroll={{ x: 820~1100 }}`，窄屏=横向滚动条，无列优先级、无卡片化 | `JobManagement.tsx:193`、`K8SManagement.tsx:148,176,248`、`ClusterManagement.tsx:114`、`ResourceManagement.tsx:51`、`MultiTenantManagement.tsx:115`、`MonitoringAlert.tsx:83`、`SecurityManagement.tsx:69`、`AccelerationSuiteManagement.tsx:71` |
| 6 | 页面模板 | `.mc-page` padding 固定 24/16；`.mc-page-title` 固定 22px；`.mc-page-desc` 固定 13px，无流式排版 | `src/index.css:157-194` |
| 7 | 超宽屏（≥1920） | **完全未处理**。无任何 `max-width` 收敛，3440 下行宽会拉到 3000px+ | 全局无相关规则 |
| 8 | Tailwind | **名义在栈内，实际零产出**。`tailwind.config.js` + `postcss.config.js` 存在，但所有 CSS 中**没有任何 `@tailwind` / `@apply` 指令** → Tailwind 编译产物为空。且 `PrivateRoute.tsx:57` 用了 Tailwind 类名（`flex items-center justify-center min-h-screen bg-gray-100`），**这些类名全部失效**，导致路由守卫的加载兜底页是裸奔无样式 div | `tailwind.config.js`、`postcss.config.js`、`components/PrivateRoute.tsx:57` |
| 9 | 图表 | ECharts 高度硬编码内联 `style={{ height: 260 }}` / `height: 300 }`，容器变化时不会自适应 | `Dashboard.tsx:349,366`、`MonitoringAlert.tsx:73` |
| 10 | 触控目标 | Topbar 图标按钮、DraggableGrid 的 `.dg-btn`（22px 高）均 < 44px 命中区 | `DraggableGrid.css:89-92` |
| 11 | 权限资产 | 16 处 `<Can>` 包裹分布于 6 个页面；`hasPermission` 仅被 `Can.tsx` 消费 | 见下节回归基线 |

### 1.3 目标（可衡量）

| 编号 | 目标 | 验收口径（可自动化/人工核对） | 现状 | 目标值 |
| --- | --- | --- | --- | --- |
| **G1** | 全断点零横向溢出 | 9 档视口（375/414/768/1024/1366/1440/1920/2560/3440）× 10 个页面，断言 `documentElement.scrollWidth <= clientWidth + 1` | 375/414 多页溢出（表格 `scroll.x`） | **90/90 通过** |
| **G2** | 内容零遮挡 | 侧边栏在任意断点展开时，主内容区不被覆盖；<1024 必须有遮罩层 + Esc 关闭 + 点击遮罩关闭 | <768 侧边栏 `position:fixed` 展开即压内容，无遮罩 | **100% 达标** |
| **G3** | 消除缩放抖动 | `Layout.tsx` 中 `window.innerWidth` 引用数；连续 resize 时主内容左边界与纯 CSS 计算值偏差 | 3 处 JS 布局计算 | **0 处 JS 布局计算，偏差 ≤1px** |
| **G4** | 断点连续覆盖 | 断点体系档位数（≤576 / 768 / 1024 / 1280 / 1440 / 1920 / 2560 / 3440） | 仅 768 一档（另有散落 480/1024/1280） | **8 档统一定义，单一真相源** |
| **G5** | 触控可达 | 手机档下所有可点击元素命中区 ≥ 44×44 CSS px（Lint 规则或人工抽检 20 个） | Topbar 图标按钮、`.dg-btn` 不达标 | **抽检 100% 达标** |
| **G6** | 超宽屏可读性 | ≥2560 时正文/表格行宽受控，主内容列宽 ≤ 2400px，正文行长 ≤ 68ch | 无约束 | **100% 达标** |
| **G7** | 功能零回归 | ①`npm run build`（`tsc && vite build`）通过；②`<Can>` 包裹点数量 ≥ 16 且不减少；③`admin` 与 `rbacuser` 两账号按钮可见性快照与改版前一致 | 16 处 | **16 处不变 + 构建通过 + 快照一致** |
| **G8** | 暗色主题零回归 | 10 个页面 × 明暗两主题视觉走查无走样（对比度、边框、图表配色） | 已支持 | **不退化** |

### 1.4 非目标（本次不做）

- 不做后端接口改造、不新增任何 npm 依赖（离线环境硬约束）。
- 不做信息架构重构（菜单分组、路由路径保持不变）。
- 不做品牌 VI 换色（品牌色 Quantum Blue 保留；若需"去渐变化"见待确认问题 Q8）。
- 不做表格虚拟滚动 / 大数据量性能优化（列为 P2）。
- 不做 i18n。

---

## 2. 目标分辨率矩阵

### 2.1 断点体系（唯一真相源，落地于 `src/theme/breakpoints.ts` + `tailwind.config.js` + CSS 变量三处同步）

采用 **min-width 移动优先（mobile-first）**，命名对齐主流设计系统（xs/sm/md/lg/xl/2xl/3xl/4xl）：

| Token | 断点 | 代表设备 / 分辨率 | 栅格列数 | 侧边栏形态 | 内容左右安全边距（gutter） | 主内容最大宽度 |
| --- | --- | --- | --- | --- | --- | --- |
| `xs` | `< 576px` | iPhone SE 375、iPhone 14 390、iPhone Pro Max 430 | 4 | **Drawer 浮层** 280px + 遮罩 | 16px | 不限制 |
| `sm` | `≥ 576px` | 大屏手机 / 小平板竖屏 600 | 4 | Drawer 浮层 280px + 遮罩 | 16px | 不限制 |
| `md` | `≥ 768px` | iPad 竖屏 768、iPad Air 820 | 8 | Drawer 浮层 280px + 遮罩 | 20px | 不限制 |
| `lg` | `≥ 1024px` | iPad 横屏 1024、1180 | 12 | **常驻折叠轨** 64px（可手动展开为浮层） | 24px | 不限制 |
| `xl` | `≥ 1280px` | 笔记本 1280、主流笔记本 1366 | 12 | **常驻展开** 240px | 24px | 不限制 |
| `2xl` | `≥ 1440px` | 1440、1600、MacBook Pro 1512 | 12 | 常驻展开 240px | 32px | 不限制 |
| `3xl` | `≥ 1920px` | FHD 1920、2K 2560 | 12 | 常驻展开 256px | 40px | **2200px 居中** |
| `4xl` | `≥ 2560px` | 2560、带鱼屏 3440 | 12 | 常驻展开 256px | 48px | **2400px 居中** |

> **3440+ 处理原则**：不铺满。主内容容器 `max-width` + `margin-inline: auto`，多出的宽度作为"呼吸留白"。理由是超过约 2400px 后，表格行的眼动扫视距离超过舒适区，阅读效率反而下降（这也是 Linear / Vercel / Grafana 在超宽屏统一收敛的做法）。表格与图表页默认跟随容器；如需真正铺满，页面根元素加 `.mc-page--full` 显式豁免（仅监控大屏场景使用）。

### 2.2 流式尺寸令牌（随视口连续变化，不跳变）

```css
/* 落地位置：src/index.css，与 tokens.ts 同步 */
--mc-gutter:  clamp(16px, 8px  + 1.2vw, 48px);   /* 页面左右安全边距 */
--mc-gap:     clamp(12px, 8px  + 0.5vw, 24px);   /* 卡片/栅格间距 */
--mc-section: clamp(16px, 12px + 0.7vw, 32px);   /* 区块之间纵向节奏 */
--mc-measure: 68ch;                               /* 正文/表单可读行宽上限 */
--mc-page-max: 2400px;                            /* 主内容最大宽度，≥2560 生效 */
--mc-sider-w: 0px;                                /* 由断点覆盖，见 2.1 */
```

### 2.3 验证视口清单（验收时逐档截图）

```
375×812    414×896    768×1024   1024×768   1280×800
1366×768   1440×900   1920×1080  2560×1440  3440×1440
```

附加验证项：浏览器缩放 200%（等效 1920→960 CSS px）、iPad Safari 横竖屏切换、Windows 缩放 125%/150%。

---

## 3. 用户故事

### 3.1 桌面大屏用户 —— 平台运维负责人（1920 / 2560 / 3440）

> 作为**平台运维负责人**，我希望在 3440 带鱼屏上同时看到更多集群与作业数据，但表格行不要被拉得过长以至于看串行，这样我能在一次扫视内判断全平台水位。

- 在 2560 下打开仪表盘，4 张 KPI 卡 + 2 张图表 + 告警列表**一屏内可见，无需滚动**。
- 在 3440 下，主内容区居中且宽度 ≤ 2400px，两侧留白对称；表格行宽受控，眼睛无需左右长距离扫视。
- 在 1920 下，作业列表表格**首屏可见行数 ≥ 20 行**（紧凑密度下），且操作列固定右侧不随横向滚动消失。
- 拖动窗口从 2560 → 1400 → 2560，布局**无抖动、无闪白、无横向滚动条闪现**。

### 3.2 笔记本小屏用户 —— 算法工程师（1366 / 1440）

> 作为**算法工程师**，我经常在 1366×768 的笔记本上提交和跟踪训练作业，希望页面不要为了"大屏好看"而牺牲小屏的信息密度，一次能看到尽可能多的作业。

- 1366 下侧边栏展开 240px 后，内容区仍有 ≥ 1060px，作业表格**不出现横向滚动条**。
- 页面标题与 KPI 数值在小屏自动缩放，**不换行、不截断**。
- 表格列按重要性自动隐藏次要列（如"创建时间"），并把被隐藏列收纳进"列设置"或行展开详情。
- 新建作业弹窗在小屏高度不足时，内部滚动而非撑破视口。

### 3.3 平板触控用户 —— 值班 SRE（iPad 768 / 1024，横竖屏都会用）

> 作为**值班 SRE**，我在机房巡检时用 iPad 查看告警和集群状态，希望横竖屏切换不会让页面错乱，手指点击不会误触。

- 竖屏 768 下侧边栏是**浮层 + 遮罩**，点击顶栏菜单按钮滑出，点击遮罩或按 Esc 关闭；主内容全宽可用。
- 横屏 1024 下侧边栏自动变为 **64px 图标轨**，主内容不被挤压；点击图标轨顶部按钮可临时展开为浮层。
- 所有按钮、开关、分页器命中区 ≥ 44×44px；**不依赖 hover**（iPad 无 hover，所有 hover 提示必须同时可点击触发）。
- 表格在 768 下自动切换为**卡片列表**，每张卡片展示 3–4 个关键字段 + 状态，操作按钮在卡片底部。
- 切换横竖屏时保持当前滚动位置与激活页签，不重置状态。

### 3.4 手机用户 —— 出差中的技术负责人（375 / 414）

> 作为**技术负责人**，我在外出时只需要用手机快速确认"有没有严重告警、有没有作业失败"，不指望在手机上创建集群，但至少要能看清、能操作最紧急的事。

- 375 下打开任何页面**无横向滚动**，标题与描述正常换行不溢出。
- 首页（仪表盘）优先呈现：4 个 KPI 指标 + 活跃告警列表，图表降级为紧凑卡片或可折叠。
- 告警列表以卡片呈现，卡片上直接可点"确认/处理"（若权限允许），无需缩放页面。
- 顶栏在窄屏只保留：菜单按钮、搜索图标、告警铃铛（带角标）、用户头像；面包屑收起为当前页名。
- 命令面板全屏展示，列表项行高 ≥ 44px，底部留出虚拟键盘安全区（`env(safe-area-inset-bottom)`）。
- 登录页卡片自适应屏宽，输入框高度 ≥ 48px，键盘弹出时不遮挡登录按钮。

---

## 4. 需求池

> 约定：`P0` = 本次改版必须完成（对应 G1–G7）；`P1` = 强烈建议，时间允许应做；`P2` = 后续迭代。
> **所有 P0/P1 条目均不得删除或弱化 `<Can>` 包裹与 `hasPermission` 逻辑。**

### P0 —— 响应式地基与核心页面（必须）

| ID | 页面 / 组件 | 改什么 | 为什么 |
| --- | --- | --- | --- |
| **R1** | `src/theme/breakpoints.ts`（新建）+ `tailwind.config.js` + `src/index.css` | 建立 8 档断点单一真相源：TS 常量、Tailwind `screens`、CSS 自定义媒体变量三处同源导出；在 `index.css` 注入 `--mc-gutter/--mc-gap/--mc-section/--mc-page-max/--mc-sider-w` 流式令牌 | 现状断点散落在 5 个文件、数值互相打架（480/768/1024/1280），改一处漏一处。对应 G4 |
| **R2** | `src/components/Layout.tsx` | **删除** `window.innerWidth`、`requestAnimationFrame` resize 监听、手工 `getContentMargin()` 与内联 `marginLeft`。改为纯 CSS：外层 `display: grid; grid-template-columns: var(--mc-sider-w) minmax(0, 1fr)`，断点只改 `--mc-sider-w`。`collapsed` 状态保留但仅用于给根节点切 `data-sider="collapsed"` 属性 | 现状是 JS 驱动布局，resize 每帧触发 React 重渲染 → 抖动；且 `Layout.tsx:45-47` 只收起不恢复，从窄拖回宽侧边栏仍折叠。对应 G3 |
| **R3** | `src/components/Sidebar/Sidebar.tsx` + `Sidebar.css` + `theme/sidebar.ts` + `src/theme/sidebar.css` | ①**三套宽度真相合并为一套**（以 CSS 变量为准，`theme/sidebar.ts` 降级为 TS 侧镜像并在注释中标注同步关系）；②`<1024` 改为 Drawer 语义：浮层 + 遮罩 + Esc 关闭 + 焦点陷阱 + 打开时锁 body 滚动；③删除 `Sidebar.css:309-437` 中成片的 `!important` 堆叠与 480/landscape 特例；④移除 `Sidebar.tsx:285` 遗留的 `window.innerWidth + orientation` 判断 | `Sidebar.css` 用 20+ 个 `!important` 覆盖 antd 与自身，且 <768 浮层无遮罩，展开直接压住内容无法关闭。对应 G2 |
| **R4** | `src/components/Topbar/Topbar.tsx` + `Topbar.css` | 新增**移动端菜单触发按钮**（仅 `<1024` 显示），点击切换侧边栏浮层；窄屏不再简单 `display:none` 掉面包屑与搜索，改为：面包屑只留当前页名、搜索降级为图标按钮、环境标签移入用户下拉 | `Topbar.css:192-202` 在 <768 直接隐藏面包屑/搜索/用户名，用户失去方位感与全局搜索入口；且没有打开侧边栏的入口。对应 G2/G5 |
| **R5** | `src/index.css` 页面模板 | `.mc-page` padding 改用 `var(--mc-gutter)`；`.mc-page-title` 改 `var(--mc-fs-h1)` 流式字号；`.mc-page-head` 在 `<768` 纵向堆叠（标题组在上、操作区在下并允许横向滚动/换行）；`.mc-page-head-extra` 按钮组在窄屏保持不换行溢出时可滚动 | 现状固定 24/16px 与 22px 字号，在 375 与 3440 两端都不合适。对应 G1/G6 |
| **R6** | 9 张表格（见 1.2 第 5 项清单） | 新建 `src/components/ResponsiveTable/`：`≥lg` 渲染表格且 `scroll={{ x: 'max-content' }}`（去掉硬编码 820/880/900/1100），列按 `priority` 分级在断点间显隐；`<md` 自动渲染**卡片列表视图**（基于同一份 columns 配置自动生成，不写第二份 JSX）。首批接入：JobManagement、ResourceManagement、ClusterManagement、MultiTenantManagement、K8SManagement（3 张） | 现状 9 张表全是固定横向滚动，手机上是"缩小的桌面表格"。这是 G1 最大的失分点 |
| **R7** | `src/components/DraggableGrid.tsx` + `.css` | `GridItem.span` 从单一数字改为**断点映射** `{ xs:12, sm:12, md:6, lg:6, xl:3, '2xl':3 }`；栅格列数随断点在 4/8/12 间切换；`gap` 改用 `var(--mc-gap)`。兼容既有 localStorage 布局数据（`mc-dashboard-layout-v1` 只存 order/hidden，不含 span，向前兼容无风险） | 现状只在 1280/768 两档降列，1366 下 4 张 KPI 卡挤在一行（每卡约 250px）过窄。对应 G1 |
| **R8** | `src/pages/Dashboard.tsx` + `MonitoringAlert.tsx` | ECharts 容器高度改为**令牌驱动**：`--mc-chart-h-sm/md/lg`（如 200/260/320），配合 `echarts-for-react` 的 `opts: { renderer }` 与外层 `ResizeObserver`（antd 无内置，可用原生 `ResizeObserver`，零依赖）；KPI 数值字号改用 `var(--mc-fs-metric)`；KPI 图标容器 44px 在小屏降至 36px | 现状 `style={{ height: 260 }}` / `{ height: 300 }` 硬编码，窄屏图表过高、宽屏过矮。对应 G1 |
| **R9** | 超宽屏收敛 | 在 `3xl`/`4xl` 断点为 `.mc-page` 加 `max-width: var(--mc-page-max); margin-inline: auto`；为正文/描述/表单/空态加 `max-width: var(--mc-measure)`；提供 `.mc-page--full` 豁免类给监控大屏 | 现状 3440 下内容无上限拉伸。对应 G6 |
| **R10** | Tailwind 二选一（**需 Q5 拍板**） | 方案 A（推荐）：在 `index.css` 顶部注入 `@tailwind base/components/utilities`，并把 `tailwind.config.js` 的 `screens` 指向 R1 的断点、`theme.extend` 接入 MDS 色板与间距；同步修复 `PrivateRoute.tsx:57`。方案 B：移除 `tailwindcss/postcss/autoprefixer` 依赖与配置，把 `PrivateRoute.tsx:57` 改为 MDS 类名 | **Tailwind 当前零产出**，`PrivateRoute.tsx:57` 的 `flex items-center ... bg-gray-100` 全部失效，路由守卫加载页是裸奔 div。无论选哪条都必须修掉这个已存在的样式缺陷 |
| **R11** | 回归护栏（约束，非功能） | `Can.tsx`、`utils/auth.ts`、`PrivateRoute.tsx`、`App.tsx` 路由表、`store/api.ts` **零改动**；仅允许新增只读 hook（如基于 antd `Grid.useBreakpoint()` 的 `useBreakpoint`，不引新依赖）。改版前后各跑一次 `<Can>` 计数（应为 16）与 admin/rbacuser 按钮快照 | 硬约束，对应 G7 |

### P1 —— 体验补齐（强烈建议）

| ID | 页面 / 组件 | 改什么 | 为什么 |
| --- | --- | --- | --- |
| R12 | `Topbar` / `DraggableGrid` / 分页器 / `Switch` | 手机档下所有可点元素命中区 ≥ 44×44（`.dg-btn` 22px→44px，Topbar 图标按钮加 `::after` 扩大命中区），不改视觉尺寸 | 对应 G5，触屏误触率高 |
| R13 | 表格（`ResponsiveTable`） | 列显隐偏好持久化到 localStorage；提供"紧凑 / 舒适"两档行高密度切换 | 数据平台的核心诉求，运维偏紧凑、管理层偏舒适 |
| R14 | 表单与弹窗（`JobManagement`/`ClusterManagement`/`MultiTenantManagement`/`K8SManagement` 的 4 个 Modal） | `<768` 时 Modal 改为**全屏或底部抽屉**（antd 原生支持 `width="100vw"` + 自定义 `style.top`），表单单列，底部操作条 `position: sticky` 并带 `env(safe-area-inset-bottom)` | 现状弹窗在小屏高度溢出，确认按钮被顶出视口 |
| R15 | `src/pages/K8SManagement.tsx` | 消除内联魔法值：`227/229` 的 `flex: '1 1 200px'`、`263` 的 `display:flex; flexDirection:column` 换成栅格类 + 间距令牌 | 内联样式无法响应断点，是超宽屏/窄屏错乱的隐患源 |
| R16 | `src/components/States/` | `TableSkeleton` 的 `columns` 数在窄屏自动递减（现固定 6/7/8 列，375 屏溢出）；`EmptyState` 文案与插图在窄屏缩放；`ErrorState` 重试按钮命中区达标 | 骨架屏本身也会溢出，属于 G1 漏网点 |
| R17 | `src/components/CommandPalette/` | 移动端：宽度 `100vw`、列表项行高 ≥ 44px、`max-height` 改用 `dvh`、底部加 `env(safe-area-inset-bottom)` | 现状 `max-height: 70vh` 在移动端虚拟键盘弹出后会被压缩到不可用 |
| R18 | `src/pages/Login.tsx` | 卡片宽度改 `clamp()`（现固定 420px）；装饰光球在窄屏关闭/降采样（3 个 300–600px 的 radial-gradient 是移动端性能负担）；输入框高度 44→48；移除组件内 `<style>` 标签（现 `294-308` 行内联媒体查询），统一进 CSS 文件 | 现状靠组件内 `<style>` 做响应式，不可维护；375 下卡片边距靠 `calc(100% - 32px)` 硬凑 |
| R19 | `MonitoringAlert` | 趋势图与告警列表在 `<lg` 纵向堆叠、在 `≥2xl` 可考虑左右分栏（图左表右） | 充分利用大屏横向空间 |

### P2 —— 后续迭代

| ID | 内容 |
| --- | --- |
| R20 | 全局密度模式（紧凑 / 舒适 / 宽松）三档切换并持久化 |
| R21 | 表格操作列 `position: sticky` 右固定 + 大列表虚拟滚动 |
| R22 | 仪表盘卡片尺寸档位（S/M/L）与跨断点布局记忆 |
| R23 | 路由切换过渡动效、骨架→内容交叉淡入 |
| R24 | 无障碍增强：`Skip to content` 链接、表格 `<caption>`、焦点顺序梳理、WCAG AA 对比度全量核对 |
| R25 | 打印 / 导出优化视图（`@media print`） |

---

## 5. 技术约束与风险

| 类别 | 约束 / 风险 | 应对 |
| --- | --- | --- |
| 依赖 | 离线环境，**禁止新增 npm 依赖** | 只用 antd 5（`Grid.useBreakpoint`、`Grid.Row/Col`、`Drawer`）、原生 CSS（Grid/Flex/Container 查询渐进增强）、`ResizeObserver` |
| 类型 | TypeScript 严格模式，**禁止新增 `any`** | 断点常量用 `as const` 导出联合类型；`ResponsiveTable` 的 columns 沿用 antd `ColumnsType<T>`，卡片视图靠 `dataIndex` 泛型取值 |
| 构建 | `npm run build`（= `tsc && vite build`）必须通过 | 每个里程碑跑一次全量构建 |
| 权限 | `<Can>` 16 处、`hasPermission` 语义不得改动 | 改版前后各做一次计数快照 + admin/rbacuser 按钮可见性对比 |
| 主题 | 暗色主题必须继续支持 | 所有新增 CSS 变量必须在 `:root` 与 `html[data-theme='dark']` 双侧定义 |
| 兼容 | 不引入 CSS Container Query 作为唯一依赖（目标浏览器可能不支持） | 用媒体查询为主，Container Query 作为渐进增强 |
| 风险 | `Sidebar.css` 大量 `!important` 与 antd 内部样式耦合，重写易回归 | 分两步：先合并宽度真相源并加遮罩（R3-a），再清理 `!important`（R3-b），每步单独验证 |
| 风险 | `Layout.tsx` 改 CSS 驱动后，侧边栏折叠动画可能变化 | 用 `grid-template-columns` 的 `transition` 替代 `margin-left` 过渡，视觉保持一致 |

---

## 6. UI 设计方向：把"国际一流水准"落成可执行定义

"国际一流水准"不是审美形容词，本节把它拆成 **7 条可评审、可用 Lint 或肉眼判定的硬规则**。参照对象：Linear（克制、密度、动效）、Vercel（排版、留白、暗色）、Stripe（层次、对齐）、Grafana（数据密度）。

### 6.1 栅格系统

- **12 列主栅格**（`≥lg`），移动端降为 **4 列**（`xs/sm`）、平板降为 **8 列**（`md`）。
- 栅格 `gap` = `var(--mc-gap)`，`clamp(12px, 8px + 0.5vw, 24px)`。
- **内容永远从栅格线起，不从任意像素起** —— 卡片内的元素不允许出现 `margin-left: 13px` 这类魔法值（现状 `K8SManagement.tsx:227` 的 `flex: '1 1 200px'` 属此类）。
- 卡片内部遵循 **8pt 内边距节奏**：小卡 16px、标准卡 20px、大卡 24px（现状 antd Card `paddingLG: 20` 已合规，保持）。
- 所有 `min-width: 0` 已在 `.mc-page` / `.dg-item` 落实，改版时**每个新增 flex/grid 子项都必须带 `min-width: 0`**，防止长文本撑破容器（这是 G1 最常见的失败原因）。

### 6.2 间距节奏（4pt 基准）

| Token | 值 | 用途 |
| --- | --- | --- |
| `--mc-sp-1` | 4px | 图标与文字间隙 |
| `--mc-sp-2` | 8px | 按钮组间隙、标签间隙 |
| `--mc-sp-3` | 12px | 表单项内部、卡片内小间距 |
| `--mc-sp-4` | 16px | 卡片内边距（小卡）、栅格 gap 下限 |
| `--mc-sp-5` | 20px | 卡片内边距（标准卡） |
| `--mc-sp-6` | 24px | 卡片内边距（大卡）、区块间距下限 |
| `--mc-sp-8` | 32px | 区块间距（大屏） |
| `--mc-gutter` | `clamp(16,8+1.2vw,48)` | 页面左右安全边距 |
| `--mc-section` | `clamp(16,12+0.7vw,32)` | 区块之间纵向节奏 |

**规则**：间距只允许取上表值；相邻区块的间距必须 ≥ 区块内部间距（否则层次塌陷）。现状 `MonitoringAlert.tsx:72` 的 `marginBottom: 16` 内联值应改为 `--mc-section`。

### 6.3 字号阶梯（流式排版，全部 `clamp()`）

| Token | 值 | 用途 | 375 → 1440 → 2560 实际渲染 |
| --- | --- | --- | --- |
| `--mc-fs-display` | `clamp(28px, 24px + 1.2vw, 44px)` | 登录页品牌名 / 大屏 Hero 数字 | 28 → 41 → 44 |
| `--mc-fs-metric` | `clamp(22px, 19px + 0.55vw, 32px)` | KPI 数值（现状固定 26px） | 22 → 27 → 32 |
| `--mc-fs-h1` | `clamp(19px, 17.6px + 0.35vw, 26px)` | 页面标题（现状固定 22px） | 19 → 22.6 → 26 |
| `--mc-fs-h2` | `clamp(15px, 14.2px + 0.16vw, 17px)` | 卡片标题 / 区块标题 | 15 → 16.5 → 17 |
| `--mc-fs-h3` | `clamp(14px, 13.6px + 0.1vw, 15px)` | 表格表头 / 小标题 | 14 → 14.8 → 15 |
| `--mc-fs-body` | `clamp(13.5px, 13.2px + 0.08vw, 14.5px)` | 表格正文 / 表单值 | 13.5 → 14.4 → 14.5 |
| `--mc-fs-sm` | `clamp(12.5px, 12.3px + 0.06vw, 13.5px)` | 页面描述 / 辅助文字（现状固定 13px） | 12.5 → 13.2 → 13.5 |
| `--mc-fs-xs` | `11.5px`（固定） | 角标 / 分组标题 / 徽标 | 固定 |

**规则**：
- 行高：标题 `1.3`，正文 `1.6`，表格 `1.5`，数值 `1.1`（数值行高过大会破坏 KPI 卡的视觉重量）。
- 字重：标题 `600–660`，正文 `400–500`，数值 `680`（沿用现状），**同一页面字重种类 ≤ 3 种**。
- 数字一律 `font-variant-numeric: tabular-nums`（现状 `.mc-num` 已实现，改版时不得丢弃）。
- 中英文混排：中文字符 `letter-spacing: 0`，西文标题 `letter-spacing: -0.3px`（现状已做，保持）。

### 6.4 信息密度

- **表格行高**：舒适档 `48px`（现状 antd `cellPaddingBlock: 12` ≈ 46px，保持）；紧凑档 `40px`（P1/R13 提供）。
- **首屏信息量下限**：1920×1080 下，列表页首屏可见数据行 ≥ 15 行（紧凑档 ≥ 20 行）。
- **密度不随宽度膨胀**：超宽屏增加的是"每行可承载的列数"，不是"元素尺寸"。卡片、按钮、输入框尺寸在 `≥1440` 后**保持恒定**（现状 KPI 图标 44px、按钮 controlHeight 34px 已合规，不得随 vw 放大）。
- **渐进披露**：次要信息（创建时间、错误信息、详细描述）默认隐藏或折叠，通过行展开 / 列设置 / Tooltip 获取，不一股脑平铺。

### 6.5 留白

- **页面外边距 = `--mc-gutter`**（16 → 48px 随视口），页面内容永不贴边。
- **超宽屏留白是特性不是浪费**：≥2560 时主内容居中，两侧留白对称，留白区域可用于固定侧栏工具（P2）。
- **卡片之间留白 = 卡片内部留白 × 0.75~1**（即 16/20/24 卡内边距对应 12/16/24 卡间距），现状 `DraggableGrid` gap 16 对 Card padding 20，合规。
- **空态留白**：空状态区域最小高度 ≥ 240px，避免空态塌陷成一条（现状 `States.css` 需复核）。

### 6.6 对齐与层次

- **对齐**：页面上所有卡片的左边缘、右边缘必须落在同一栅格线上；表格首列与卡片标题左对齐；数值列一律**右对齐**（现状 ResourceManagement 的 total/used/available 未明确右对齐，改版时统一）。
- **层次三手段，按优先级使用**：① 留白与分组 → ② 字号与字重 → ③ 分割线 → ④ 阴影 → ⑤ 背景色差。
  - **禁止用阴影承担主要层次**（现状 `Login.tsx:153` 用了 `0 25px 50px rgba(0,0,0,.25)` 的重阴影，属于过度使用）。
  - 分割线只用于 1px 弱线（`--mc-line`），不用 `line-strong`。
- **Elevation 四级**：`e0` 页面底（无阴影，靠 1px 边框）→ `e1` 卡片（`--mc-shadow-1`）→ `e2` 悬浮/下拉（`--mc-shadow-3`）→ `e3` 模态（`--mc-shadow-4`）。现状令牌已分级，只需保证不越级使用。
- **圆角一致性**：卡片 12px、按钮/输入 8px、标签 6px、徽标 pill。现状已合规。

### 6.7 动效准则

- **时长**：沿用现状 `--mc-t-fast: 120ms`（hover/按下）、`--mc-t-base: 180ms`（展开/折叠/显隐）、`--mc-t-slow: 280ms`（布局级位移，如侧边栏折叠）。**新增动效不得突破这三档**。
- **缓动**：统一 `cubic-bezier(.2,.8,.2,1)`（`--mc-ease`），禁止 `linear` 与 `ease-in-out` 混用。
- **只动 `transform` 与 `opacity`**；`width` / `margin` / `top` / `left` 的过渡只允许出现在侧边栏折叠这一处，且必须用 `grid-template-columns` 或 `transform: translateX` 替代（后者性能更好）。
- **resize 期间禁止触发任何过渡**（R2 落地后天然满足）：窗口拖拽时布局应立即跟随，不能有缓动尾随。
- **`prefers-reduced-motion` 必须继续支持**（`index.css:127-136` 已实现，新增动效自动纳入）。
- **禁止的动效**：现状 `Sidebar.css` 的 `gradient-shift` / `glow`（logo 渐变流动 + 发光）、`Login.tsx` 的 3 个 `float` 光球 —— 这类"装饰性持续动效"与 Linear/Vercel 的克制取向相悖，建议降级为静态或仅在登录页保留（见 Q8）。
- **数据刷新反馈**：轮询/重取时不要用整块骨架闪烁，用局部 `opacity` 或角标提示（P2/R23）。

### 6.8 一句话自检

> 把页面缩到 375 和放大到 3440，如果两档下都"像专门为该尺寸设计过"而不是"被拉伸/被压缩的同一张设计稿"，就达标了。

---

## 7. 待确认问题（需拍板）

| # | 问题 | 选项 | 影响 | PM 建议 |
| --- | --- | --- | --- | --- |
| **Q1** | **手机端的定位**：是"只读查看告警/状态"，还是"完整可操作"（能在手机上新建作业、创建集群、启停套件）？ | A. 只读 + 紧急操作（取消作业 / 确认告警）<br>B. 完整操作（含所有创建/编辑表单） | 直接决定 R6 卡片视图与 R14 弹窗适配的工作量（A ≈ 少 40% 工作量） | **A**。数据平台的移动端主场景是"看"，复杂表单在手机上转化率极低 |
| **Q2** | **超宽屏（≥2560）策略**：内容居中留白，还是铺满并利用多出的宽度做分栏（如图左表右、双栏卡片流）？ | A. 居中留白（max-width 2400）<br>B. 铺满 + 部分页面多栏 | 影响 R9 与 R19 的实现方式 | **A**（P0）+ **B 中"监控页左右分栏"**（P1/R19）。居中留白实现成本低、风险小，先落地 |
| **Q3** | **平板断点侧边栏形态**：1024–1279 用 64px 常驻图标轨，还是与 <1024 一样用浮层？ | A. 常驻图标轨 64px<br>B. 浮层 + 遮罩 | 影响 R3 与内容区可用宽度 | **A**。1024 横屏下 64px 轨比浮层更高效，且内容区仍有 936px |
| **Q4** | **表格在 <768 的呈现**：自动卡片化，还是保留横向滚动（现状做法，仅优化滚动体验）？ | A. 卡片化<br>B. 保留横向滚动 | 这是本次改版**工作量最大的单点**，直接决定排期 | **A**。用户明确要求"自适应不同分辨率"，横向滚动条是最典型的"不自适应"表现 |
| **Q5** | **Tailwind 的处置**：接入（补 `@tailwind` 指令 + 扩展 MDS 断点/色板），还是彻底移除？ | A. 接入<br>B. 移除 | 影响 R10 方向与后续开发范式 | **A**。既已付费（构建配置已存在），接入后可让断点类名与 TS 常量同源，且 antd 覆盖不到的细粒度布局很好用。**但无论选哪条都必须先修 `PrivateRoute.tsx:57`** |
| **Q6** | **是否需要"紧凑/舒适"密度切换**（P1/R13）？ | A. 需要（持久化到用户偏好）<br>B. 不需要，固定当前密度 | 约 1–2 天工作量 | **A，但放 P1**。运维视角的"一屏多看几行"与管理者视角的"看得清"诉求冲突，切换是标准解法 |
| **Q7** | **仪表盘的拖拽自定义布局在移动端是否保留**？HTML5 Drag & Drop 在触屏不可用，现有"前移/后移"按钮可兜底但体验一般 | A. 移动端禁用拖拽，仅保留显示/隐藏<br>B. 保留前移/后移按钮<br>C. 移动端完全隐藏自定义入口 | 影响 R7 | **A**。移动端隐藏拖拽，保留"显示/隐藏卡"即可 |
| **Q8** | **视觉风格幅度**：是否要从当前的"渐变 + 光晕 + 彩色图标"转向更克制的"单色图标 + 数据密度优先"？现状侧边栏有 `gradient-shift`/`glow` 持续动效，登录页有 3 个浮动光球，与 Linear/Vercel 的克制取向有差距 | A. 大幅克制化（去渐变、去光晕、统一单色描边图标）<br>B. 小幅收敛（仅去掉持续动效，保留品牌渐变）<br>C. 保持现状，只做响应式 | **这是本次改版调性最大的分歧点** | **B**。品牌色 Quantum Blue 是资产，全去会失去辨识度；但持续动效与大重阴影建议去掉，这已足以让观感"上一个台阶" |
| **Q9** | **是否需要支持浏览器缩放 200% / Windows 125–150% 缩放**（WCAG 1.4.4 文本缩放）？ | A. 需要，纳入验收清单<br>B. 不需要 | 影响验收视口清单与 R1 断点取值（若需支持，断点应更多用 `em`/`rem` 而非 `px`） | **A**。企业内网环境放大缩放很常见，且成本很低（断点用 `rem` 即可） |
| **Q10** | **验收方式**：由我（PM）产出 10 档视口的截图基线供比对，还是由前端自测 + 我抽检？ | A. PM 出基线<br>B. 前端自测 + PM 抽检 | 影响排期与质量兜底 | **B + 关键 4 档（375/1366/1920/3440）PM 复核**。全量 10 档 × 10 页 = 100 张基线成本过高 |

---

## 8. 里程碑（建议，待 Q1–Q8 拍板后排期）

| 阶段 | 内容 | 产出 |
| --- | --- | --- |
| M1 · 地基 | R1 / R2 / R3 / R10 | 断点体系 + CSS 驱动布局 + 侧边栏浮层化；G2/G3/G4 达标 |
| M2 · 页面模板与表格 | R4 / R5 / R6（首批 6 张表）/ R9 | G1（表格页）/ G6 达标 |
| M3 · 仪表盘与图表 | R7 / R8 | G1（仪表盘）达标 |
| M4 · 补齐与验收 | R11 回归核验 + P1 中 R12/R16/R18 + 10 档视口走查 | G5/G7/G8 达标，交付 |

每个里程碑结束必须跑：`npm run build` + `<Can>` 计数（=16）+ admin/rbacuser 按钮快照 + 明暗双主题走查。

---

## 附录 A：现状关键代码索引

| 关注点 | 文件:行 |
| --- | --- |
| 设计令牌（CSS） | `src/index.css:5-88` |
| 全局唯一断点 | `src/index.css:232-236` |
| JS 驱动布局 | `src/components/Layout.tsx:14-19, 28-42, 45-49, 54-61` |
| 侧边栏宽度双真相 | `src/theme/sidebar.ts:1-31` ↔ `src/components/Sidebar/Sidebar.css:309-437` |
| 侧边栏遗留 orientation 判断 | `src/components/Sidebar/Sidebar.tsx:285` |
| 表格固定横向滚动（9 处） | `JobManagement.tsx:193`、`K8SManagement.tsx:148/176/248`、`ClusterManagement.tsx:114`、`ResourceManagement.tsx:51`、`MultiTenantManagement.tsx:115`、`MonitoringAlert.tsx:83`、`SecurityManagement.tsx:69`、`AccelerationSuiteManagement.tsx:71` |
| 图表硬编码高度 | `Dashboard.tsx:349/366`、`MonitoringAlert.tsx:73` |
| Tailwind 零产出 + 失效类名 | `tailwind.config.js`、`src/components/PrivateRoute.tsx:57` |
| 权限资产（16 处） | `JobManagement`、`ClusterManagement`、`MultiTenantManagement`、`AccelerationSuiteManagement`、`SecurityManagement`、`K8SManagement` 中的 `<Can perm=...>` |
| K8S 内联魔法值 | `src/pages/K8SManagement.tsx:227/229/263` |
| 登录页内联响应式 | `src/pages/Login.tsx:294-308` |
