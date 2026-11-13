# Metaclouds UI 全面复盘与优化报告（2026-11-13）

> 交付：设计系统 **MDS v3.0** + 全站视觉升级 + 真实浏览器核验。
> 范围：`metaclouds-frontend-vue`（Vue 3.5 / Element Plus 2.8 / ECharts 5.5），15 个业务页 + 登录页 + 8 个通用组件。
> 原则：**不放宽任何门禁、不伪装通过、不把「未做」写成「已完成」**。

---

## 0. 结论摘要

| 项 | 结果 |
|---|---|
| 改造前定性 | 功能完整、工程纪律好，但视觉语言停留在 2018–2020 年「科技蓝管理后台」：三套蓝、紫渐变主按钮、发光 logo、拟物双向阴影、正文 14.5px、非标准字重 |
| 改造后定性 | 单一主色真相、细边框 + 极轻单向阴影、8pt 网格、完整排版阶梯、WCAG AA 内建、克制动效；对齐 Linear / Vercel / Stripe 一类 B 端产品的设计语言 |
| 落地范围 | 全局设计系统（覆盖全部 15 页）+ 全站框架 + 登录页 + 仪表盘 + 图表主题 + 基础资产（favicon / meta） |
| 门禁 | `type-check` 0 错误；`vitest` **61/61 通过**（Login 由 6 条增至 7 条，新增安全性与可访问性回归）；`vite build` 成功 |
| 真实渲染核验 | Chrome 无头截图登录页（1440×900）、仪表盘（1600×1000）、集群管理（1600×1000），逐项目视核验并修复 3 类截图暴露的缺陷 |
| 明确未做 | 暗色主题、14 个业务页表格列宽重构、轮询逻辑统一、自托管字体、i18n、自动化视觉回归基线、git commit（用户未要求） |

---

## 1. 审计范围与方法

- **只读盘点**：逐文件阅读 24 个 `.vue`、31 个 `.ts`、2 个 `.css`，以及路由 / 构建配置 / 文档，形成页面、组件、路由、令牌、Element Plus、ECharts、可访问性、静态资产八类清单。
- **标杆对照**：视觉层级、排版尺度、色彩语义、间距节奏、组件状态、响应式、WCAG AA、动效克制、品牌一致性九个维度。
- **真实运行验证**：本地后端 `:8001` + 前端 `:3000`，Chrome 无头真实渲染截图（而非只看代码或单测）。
- **门禁回归**：类型检查 / 单元测试 / 生产构建，全部在改动后重跑。

---

## 2. 复盘：改造前的七类差距

| # | 维度 | 改造前事实 | 国际一流基准 | 严重度 |
|---|---|---|---|---|
| 1 | 视觉语言 | 深蓝渐变侧栏 + 发光 logo、蓝紫渐变主按钮、登录页紫色光晕与渐变文字、`.el-card` 使用拟物双向阴影（`6px 6px` 深 + `-6px -6px` 白） | 中性底、单色层级、1px 细边框、极轻单向阴影、无发光无渐变按钮 | 高 |
| 2 | 主色真相分裂 | `--mc-brand #2f6bff`、`--el-color-primary #1e50e0`、按钮渐变收在 `#5b3fd9` 紫、侧栏另用 `#1677ff/#4096ff/#0958d9`、登录左栏又一套 `#0b1f4d→#123a8f` | 一套品牌色板，强调色全局唯一 | 高 |
| 3 | 排版体系 | 正文 `13.5–14.5px`；字重出现 `650/660/680` 非标准值（系统字体回退）；无行高规范；标题负字距只在一处 | 15–16px 正文基线、标准字重阶梯、统一行高与标题字距 | 高 |
| 4 | 空间与信息密度 | `20/16/12.5px` 混用；`.el-card__body` 固定 20px；7 张 KPI 卡全用同一渐变图标底；饼图 9 类厂商的 `{d}%` 标签互相重叠 | 8pt 网格、主次指标有层级、图表标签不与数据打架 | 中高 |
| 5 | 可访问性 | 无 skip-link；`:focus-visible` 用 `box-shadow` 双环（会覆盖组件自身阴影）；表格文字按钮触控目标 32px / 44px 页面间不一致；Topbar「个人信息」是点击无响应的死链；登录页明文展示默认账号 `admin` | WCAG AA 4.5:1、键盘可达、44px 触控目标、不泄露凭据 | 高 |
| 6 | 组件状态 | loading 只有一个转圈图标 +「加载中…」；空态用 Element Plus 默认插画；错误态文案裸奔；无骨架屏 | Skeleton 保持布局稳定、空态有引导、错误态有恢复路径 | 中 |
| 7 | 基础资产 | `index.html` 引用不存在的 `/vite.svg`；无 `public/`；全项目 0 个图片 / 字体 / logo 资源；无 `description` / `theme-color` | 品牌 favicon、完整 meta、可控字体栈 | 中 |

---

## 3. MDS v3.0 设计规范

### 3.1 五条原则
1. **单一主色真相**：品牌蓝只承担「强调」；同色相渐变仅用于徽标与登录装饰，不再用于按钮。
2. **去装饰化**：层级由「细边框 + 极轻单向阴影 + 留白 + 字重」表达，删除拟物高光、发光、彩色顶条。
3. **8pt 网格 + 完整排版阶梯**：正文提升到 14–15.5px，标准字重 400/500/600/700，四级行高。
4. **可访问性内建**：AA 文本对比、2px 焦点环、44px 触控目标、skip-link、reduced-motion、prefers-contrast、forced-colors。
5. **只交付一套做到极致的 light 主题**（`color-scheme: light`）。

### 3.2 令牌（权威源 `src/styles/index.css :root`，镜像 `src/theme/tokens.ts`）

| 类别 | 关键值 |
|---|---|
| 品牌 | `base #2f6bff` / `fg #1e50e0`（白底 5.6:1，同时是 `--el-color-primary`）/ `subtle #eff4ff` / 深档 600–800 / 同色相渐变 `#3b7bff→#1f52e0` |
| 语义 | success `#16c784` / fg `#0b7a54`；warning `#ffb020` / fg `#8a5a00`；danger `#ff5c7a` / fg `#c2185b`；info / teal；每色均有 soft 底 |
| 中性 | text1 `#0e1726`、text2 `#44536c`、text3 `#5b6880`（白底 5.4:1，AA 正文）、line `#e3e8f0`、surface 三级、bg `#f6f8fc` |
| 字号 | xs 11.5 / sm 12.5–13.5 / body 14–15.5 / h2 15–17 / h1 22–30 / md 16 / lg 18 / xl 22 / 2xl 28 / 3xl 36 |
| 行高 / 字重 | 1.25 / 1.45 / 1.6 / 1.75；400 / 500 / 600 / 700 |
| 间距 | 4 / 8 / 12 / 16 / 20 / 24 / 32 / 40 / 48 / 64；流式 gutter / gap |
| 圆角 | 6 / 8 / 12 / 16 / 22 / 999 |
| 阴影 | 四级单向极轻阴影；`raisedSm/raised` 保留为兼容别名，值已替换为干净阴影 |
| 动效 | ease `cubic-bezier(0.2,0.8,0.2,1)`；120 / 180 / 280ms |
| 其他 | 内容最大宽 1600px、z-index 刻度、品牌聚焦环 `rgba(47,107,255,.16)`、图表第 9 系列色 |

### 3.3 组件规范（全局生效，一次覆盖全部 15 页）
- **卡片**：1px 细边框 + `--mc-shadow-1`，radius 12，header 细分隔线；不做 hover 上浮。
- **按钮**：primary 纯色 `#1e50e0`（白字 5.6:1）、hover 深档、active 下压 1px；次级按钮白底细边框；表格文字按钮统一 44px。
- **输入 / 选择**：1px 描边、hover 加深、聚焦品牌环；large 最小高 44px。
- **表格**：表头浅底 + 小字 600、行 hover、去竖向边线、表头允许换行；固定右列取消脏渐变阴影，改 1px 分隔线。
- **标签 / 弹层 / 分页 / Tabs / 下拉 / 消息**：统一 soft 底 + fg 字、radius、分层阴影与细分隔线。
- **三态**：loading 用骨架屏（保持布局尺寸、避免 CLS）；错误态 `role="alert"` + 恢复提示；空态保留可读描述。
- **图表**：全站统一 `mc-light` 主题；饼图关闭常驻外部百分比标签，改为「图例 + tooltip + hover 环心详情」。

### 3.4 可访问性与响应式
- skip-link → `#mc-main`（登录页与业务页均有锚点）；`:focus-visible` 改 2px `outline` + 2px 偏移。
- `role="status"` / `role="alert"`、`aria-live`、`aria-current="page"`、折叠态 `title`、徽标 `aria-label`、装饰 SVG `aria-hidden`。
- `prefers-reduced-motion`（含骨架 shimmer 关闭）、`prefers-contrast: more`、`forced-colors: active`。
- 断点：≥1401 / 1024–1400 / ≤1023（侧栏浮层）/ ≤900（登录页单栏）/ ≤767（弹层收敛）/ ≤640（页头堆叠）/ ≤480。

---

## 4. 本轮落地清单

| 文件 | 改动 |
|---|---|
| `src/styles/index.css` | **重写**：v3 令牌、排版基础、8pt 网格、全量 Element Plus 皮肤、骨架屏、skip-link、焦点环、对比度/动效/响应式兜底 |
| `src/theme/tokens.ts` | 同步 FORCE-SYNC 镜像：新增 sidebar / chartAccent / warningSurface、排版与层级令牌；`chartPalette` 不再含孤立硬编码色 |
| `src/styles/sidebar.css` | 重写：中性墨色单底、细分隔线、低饱和激活态 + 2px 指示条、子菜单竖线、克制徽标与退出按钮 |
| `src/App.vue` | 增加键盘 skip-link |
| `src/components/Layout.vue` | 内容区 `id="mc-main" tabindex="-1"`；路由过渡改 120ms fade + 4px 位移 |
| `src/components/Topbar.vue` | 修掉「个人信息」死链 → 真实只读账户信息弹层；用户区改为可键盘聚焦的 `button`；面包屑 `aria`；60px 顶高与克制视觉 |
| `src/components/Sidebar.vue` | 角色过滤改 **fail-closed**（与 router 守卫同口径）；折叠态 `title`；`aria-current`；徽标 `aria-label` |
| `src/components/ModeBanner.vue` | token 化橙黄状态条、内联 SVG 图标、`aria-live`；文案与 `data-testid` 不变 |
| `src/components/PageState.vue` | loading 升级为骨架屏；错误态补 `role="alert"` 与恢复提示 |
| `src/pages/Login.vue` | 重构：抽象算力网格 SVG 品牌区、去光晕/彩条/blur/大阴影；8pt 卡片；行内 `role="alert"` 错误；**移除明文默认账号与初始密码线索**；`id="mc-main"` |
| `src/pages/Dashboard.vue` | KPI 卡浅底语义图标 + 细边框 + 28px tabular 数值；4/3/2/1 列网格且末张补齐末行；饼图标签重叠修复；告警列表等级色条；图表卡层级 |
| `src/pages/MonitoringAlert.vue` | 接入 `mc-light` 主题（此前未应用），与仪表盘图表视觉统一；标准字重 |
| `src/theme/echarts.ts` | 字体栈 / 轴分割线改为令牌引用；标准字重；更新过时注释 |
| `Cluster/GPU/Job/Partition/Resource/MonitoringAlert .vue` | 表格文字按钮触控目标统一 44px；`660/680` 非标准字重 → `600` |
| `public/favicon.svg` | **新建**：品牌 mark（M + 算力节点），修复此前指向不存在 `/vite.svg` 的 404 |
| `index.html` | favicon / description / theme-color / color-scheme |
| `src/pages/__tests__/Login.spec.ts` | 原「渲染默认账号提示」改为**更强的安全性回归**（断言不再出现默认账号/初始密码/`admin`，并新增 skip-link 与装饰 `aria-hidden` 断言） |

---

## 5. 验证结果

| 检查 | 结果 |
|---|---|
| `npm run type-check` | 0 错误 |
| `npm run test` | 12 个文件 / **61 个测试全部通过**（无跳过、无放宽） |
| `npm run build` | 成功；全局 CSS 9.66 kB → 18.96 kB（gzip 4.90 kB），favicon 就位，临时核验页未进入产物 |
| 真实浏览器：登录页 | 左右分栏、品牌网格装饰可见且不喧宾夺主；卡片细边框、表单 44px、按钮纯色；无错位/截断 |
| 真实浏览器：仪表盘 | 7 张 KPI 两行填满无空位；饼图标签不再重叠；侧栏分组/描述可读；面包屑无尾随斜杠 |
| 真实浏览器：集群管理 | 表头浅底、标签 soft 底、操作列边界干净、分页对齐 |

**截图暴露并已修复的三类缺陷**（纯代码检查无法发现）：
1. GPU 厂商分布 9 类小占比的 `{d}%` 标签糊成一团 → 关闭常驻标签，改图例 + tooltip + hover 环心详情。
2. 7 张 KPI 卡 `auto-fit` 留下末行右侧空位 → 显式 4/3/2/1 列 + 末张按断点补齐剩余列。
3. Element Plus 固定右操作列的灰渐变阴影形成脏带、表头中文被截断 → 阴影改 1px 分隔线，表头允许换行。

**一项诊断澄清**：无头 Chrome 截图中「活跃」标签一度不可见，DOM 核对确认元素存在且带 `el-zoom-in-center-enter-from` —— 是 `--virtual-time-budget` 冻结了 Vue 进入动画，非样式回归；以 `--force-prefers-reduced-motion=reduce` 重截后标签全部正常显示，同时验证了 reduced-motion 分支有效。

---

## 6. P2 待办收敛状态（v3.1 续作后更新）

> **复盘订正（2026-11-13 下午，本轮第一动作就是核验清单本身）**
>
> v3.0 报告里有两处判断未经实测，现予更正：
> 1. 「15 个业务页 `<style scoped>` 含大量硬编码色值」——**不成立**。精确统计（仅 `<style>` 段）为 **0 处**；
>    全站 `<style>` 内色值共 21 处，其中 17 处在登录页左侧「有意固定深色」品牌区，其余 4 处是
>    按钮白字 / 警示边框 / 蓝色投影。15 个业务页早已全量消费 `--mc-*` 令牌。
>    **「暗色主题必须先逐页迁色」是一个虚构的阻塞**——它让一个本可直接落地的能力被写成「有意不做」。
> 2. 「其余 10 页内联复制轮询」——实际是 **11 页**（3 页已用 composable + 11 页内联 = 14 个轮询页面）。
>
> 方法论教训：把「保守估计」写成「事实」会误伤后续决策。P2 清单每一项都应附可复现的计数口径。

| 项 | v3.0 | v3.1 状态 | 落地内容 / 仍剩余 |
|---|---|---|---|
| **暗色主题** | 🔴 有意不做 | **🟢 已落地** | 三态（浅/深/跟随系统）+ 防闪白内联脚本 + EP 官方暗色变量 + MDS 暗色令牌覆盖 + `mc-dark` 图表主题 + 顶栏/登录页切换器 + 三主题页双模式截图核验。**剩余**：逐页回归只覆盖登录 / 仪表盘 / 集群 3 页 |
| **表格列宽策略** | 🔴 | **🟡 部分落地** | 13 页 / **175 列** `width` → `min-width`（15 个 `fixed` 列保留确定宽度）；集群页加视口优先级隐藏 3 个次要列 + 「显示全部列」开关，实证修掉「多集」截断。**剩余**：其余 12 个含 `fixed` 列的宽表在紧凑视口仍可能出现半截列；`McDataTable` 未抽 |
| **轮询逻辑重复** | 🔴 | **🟢 已落地** | 11 页内联实现统一到 `useAutoRefresh`（新增 `resetCountdown`），共净减 **341 行**复制代码，生命周期/可见性行为保持一致。**剩余**：13 页的「自动刷新」提示模板仍各自重复，未抽成组件 |
| **字体资产** | 🔴 | **🟢 已落地（决策路径）** | 自托管 Inter woff2 在沙箱被代理 **502 阻断**，按 v3.0 给出的备选路径明确改为**纯系统 UI 栈**，去掉「声明了但机器没装」的虚假首位；数字等宽由 `tabular-nums` 保证，零请求 / 零 FOUT |
| **空态插画** | 🔴 | **🟢 已落地** | 新增 `McEmpty.vue`（点阵网格 + 空芯片托盘 + 低饱和品牌节点，全令牌配色），统一替换 **9 处** `el-empty` |
| **ECharts 初始化时序缺陷** | （未列入） | **🟢 新发现并修复** | 整页直接加载 `/dashboard` 时，数据 watch 早于 `v-if` 图表容器挂载，导致 `echarts.init` 永不执行、卡片全空；补容器 ref 的 `flush:'post'` 兜底 watch。修复后 DOM 实测 canvas 0 → 2 |
| **侧栏暗色可读性** | （未列入） | **🟢 新发现并修复** | 暗色下分组标题 / 菜单描述的白字透明度从 0.5/0.55 提到 0.8/0.74 |
| **自动化视觉回归** | 🔴 | **🔴 未做** | `playwright` / `@playwright/test` / `pixelmatch` / `pngjs` / `puppeteer` **均未安装**，CDN 拉取被代理 502 → 本轮无法接入基线。仍是「无头 Chrome 截图 + 人工目视」，截图已归档 |
| **i18n / 设计稿交付** | 🔴 | **🔴 未做** | 仅 zh-CN，无 Figma / 组件文档站；需对外交付时再补 |
| **数据真实性** | 后端缺口 | **不变** | 趋势为 `Math.sin` 合成、执行体为模拟、K8s 为 mock（`ModeBanner` 诚实提示）；非 UI 范畴，见 `docs/system-fixes-2026-11-09.md` |
| **git commit** | 未提交 | **未提交** | v3.0 + v3.1 全部改动均在工作区，等用户确认 |

---

## 7. 设计决策记录

1. **保留科技蓝品牌 identity**，不换色相：改造的是「用法与层级」，不是品牌识别；同时遵守既有的科技蓝约定。
2. ~~**单主题做到极致 > 草率双主题**：暗色列为 P2 并写明前置条件，避免半成品。~~
   **v3.1 撤销此决策**：它依赖「15 页大量硬编码色值」这一未经实测的假设（见 §6 订正）。
   实测后前置条件不存在，暗色主题已于同日下午完整落地。保留原条目是为了记录决策如何被证据推翻。
3. **测试随产品文案同步，不放宽门禁**：登录页移除默认账号明文后，把原断言替换为更强的安全性 + 可访问性回归，测试数 60 → 61。
4. **「测试绿」≠「视觉对」**：坚持真实浏览器截图，因此发现了 3 类纯静态检查遗漏的缺陷。

---

## 8. 复现方式

```bash
# 门禁
npm --prefix D:/YCYD/metaclouds-frontend-vue run type-check
npm --prefix D:/YCYD/metaclouds-frontend-vue run test
npm --prefix D:/YCYD/metaclouds-frontend-vue run build

# 本地实跑（后端需在其自身目录启动，否则 migrations 解析失败）
cd D:/YCYD/metaclouds-backend-rust && ./target/release/metaclouds-backend-rust.exe   # :8001
CODEBUDDY_SAFE_DELETE_ENABLED=0 VITE_API_PROXY_TARGET=http://127.0.0.1:8001 \
  npm --prefix D:/YCYD/metaclouds-frontend-vue run dev -- --host 127.0.0.1 --port 3000

# 打开 http://127.0.0.1:3000/login （本地演示账号见 gitignore 的环境配置；口令不入库）
```

核验截图位于 `.workbuddy/ui-review/`（login-v3-final.png、dashboard-v3-final.png、cluster-v3-reduced.png）。

---

## 9. v3.1 续作：P2 收敛记录（2026-11-13 下午）

### 9.1 暗色主题架构（本轮主体）

| 层 | 实现 | 关键约束 |
|---|---|---|
| 状态机 | `src/theme/useTheme.ts`：`light / dark / system` 三态 → `activeTheme` 二态 | DOM 唯一写入口：`data-theme` + `html.dark` class + `colorScheme` + 两个 meta |
| 防闪白 | `index.html` 同步内联脚本，位于任何样式表之前 | 与 `initTheme()` 取值口径一致，避免首帧白闪 |
| 样式 | `index.css` 新增 `html.dark[data-theme='dark']` 暗色令牌块 | 特异性 (0,3,1) 高于 EP 官方 `html.dark` (0,1,1)，因此**既保留** EP 对 `el-button`/`el-card`/`el-empty` 的组件级暗色处理，**又把** `--el-*` 主色/文字/描边/填充收敛回 MDS 品牌令牌 |
| 令牌镜像 | `tokens.ts` 新增 `darkColorTokens` / `darkShadowTokens` | FORCE-SYNC：ECharts 无法读 CSS 变量，JS 侧必须有一份逐字一致的镜像 |
| 图表 | `echarts.ts` 新增 `mc-dark` 主题、`chartThemeName`、`onChartThemeChange`、`activeColorTokens` / `activePalette` | ECharts 在 `init` 时固化主题色，`setOption` 无法热更新 → 两个图表页订阅主题变化后 `dispose` + 重建实例 |
| 交互 | `ThemeToggle.vue` 三态下拉，接入顶栏与登录页 | 图标按「已解析主题」显示，菜单标记「用户选择的模式」，两者分离 |

有意保留的「固定深色」区域：侧边栏、登录页左侧品牌区。这在两种主题下是同一套品牌导航，属一致性设计，不是半亮半暗的破面；暗色下额外补了一条细边界维持分栏。

### 9.2 真实缺陷修复（截图驱动，静态检查发现不了）

1. **ECharts 永不初始化**：`dashboard` 的图表容器由 `v-if` 控制，整页直接加载时 `PageState` 还在骨架屏，
   数据 watch（即使 `flush:'post'`）可能早于容器挂载触发 → `syncCharts()` 里 ref 为空 → 图表卡片全空白。
   v3.0 的核验走的是「登录页 → 守卫跳转」路径，多一次客户端路由周期，恰好掩盖了它。
   修复：新增 `watch([resourceChartRef, jobChartRef, vendorChartRef], …, { flush: 'post' })` 兜底补初始化。
   **验收口径是 DOM 实测**：`canvas` / `_echarts_instance_` 从 0 → 2，而不是「截图看起来差不多」。
2. **固定操作列遮挡倒数第二列**：`dump-dom` 读到 `<col width>` 显示 12 列合计 1560px，而 1600px 窗口内容区仅约
   1312px；EP 把 `min-width` 直接输出成固定 `width`，`fixed="right"` 操作列浮层盖住「多集群」列，
   表头被压成「多集」、数据标签不可见。修复：紧凑视口（`<1800px`）按优先级隐藏 GPU 厂商 / 高可用 /
   访问方式 3 列（合计 400px），并提供「显示全部列」开关保证信息不丢失。
   修复后截图核验：「多集群」表头完整、「未启用」标签可见、固定列边界干净。

### 9.3 验证

| 检查 | 结果 |
|---|---|
| `type-check` | 0 错误 |
| `vitest` | **71/71 通过**（v3.0 为 61；新增主题状态机 6 条、切换组件 3 条、登录页外观入口 1 条） |
| `vite build` | 成功；临时核验 helper 已删除，`dist/` 不含它 |
| 真实浏览器 | 登录页 / 仪表盘 / 集群管理 × light + dark 共 5 张最终截图；暗色图表扇区边框、图例、空态插画逐项核验 |

两处测试选择器加固（不是放宽门禁）：新增外观按钮改变了 `find('button')` 命中顺序，把 3 个登录用例改为
`find('button.login-submit')` 语义定位，并新增一条断言锁死「登录按钮必须有稳定类名」。

### 9.4 本轮明确未做（不伪装为已完成）

- **Playwright 视觉回归基线**：依赖全部缺失且 CDN 502，无法安装。当前核验命令可复现：
  `chrome.exe --headless=new --force-prefers-reduced-motion=reduce --virtual-time-budget=20000
  --screenshot=… URL`，但**没有像素断言、没有 CI 门禁**。联网环境装好依赖后按 §6 表格路径接入。
- **其余 12 个含 `fixed` 列的宽表**：列优先级机制（`useViewport`）已就绪，但只接入集群页；逐页需要业务判断哪些列可隐藏，未批量套用。
- **「自动刷新」提示组件 / `McDataTable`**：仍是 P2，未抽。
- **i18n、组件文档站、数据真实性、git commit**：状态同 §6。

### 9.5 最终截图

`.workbuddy/ui-review/` 下 `p2-login-dark2.png`、`p2-login-light2.png`、`p2-dashboard-dark3.png`、
`p2-dashboard-light3.png`、`p2-cluster-dark2.png`。
