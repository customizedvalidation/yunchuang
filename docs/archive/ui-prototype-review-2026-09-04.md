# UI 原型复盘 · metaclouds-ui-prototype（2026-09-04）

> 复盘对象：`.workbuddy/tmp/metaclouds-ui-prototype.html` + `metaclouds-ui-design-spec.md` + 4 张渲染截图
> 复盘方式：四张截图逐张目检 + 633 行源码逐段核对 + 对比度逐组实算 + 前端令牌 grep 验证
> 结论先行：**视觉与令牌层达成度高，但"可访问性/键盘友好"的宣称有 3 处不实，存在 2 个功能性 bug，建议修复后再进入 React 移植。**

---

## 一、总评

| 维度 | 评价 |
|---|---|
| 视觉完成度 | ★★★★☆ 浅深双主题渲染均成立，新拟态语言统一，截图可直接当视觉基线 |
| 设计系统严谨度 | ★★★★☆ 令牌、圆角阶、间距阶、阴影三件套全部落地且与规格书一致 |
| 可访问性 | ★★☆☆☆ 宣称 ≥AA / 键盘友好，实测 **text-3 全系不达标、导航不可键盘操作、hidden 被覆盖** |
| 交付一致性 | ★★★☆☆ 截图与源码数据不同步（侧栏 badge 842 vs 342） |

---

## 二、核实通过的部分（不是自夸，是验过）

1. **令牌与规格书 100% 一致**：`--brand/#2F6BFF`、`--violet/#7C5CFF`、渐变、圆角 12/16/22/28、间距 4→48、KPI 30px/750/-1px，全部与 spec 第 1 节吻合。
2. **与现有前端对齐宣称属实**（grep 验证）：`src/theme/tokens.ts` 确有 `500: '#2F6BFF'`、`violet: '#7C5CFF'`；`index.css` 的 `--mc-brand` 同值。"按令牌搬运"可行。
3. **环形图数学正确**：4 段 dasharray 173/83/79/42 对 46/22/21/11%（周长 2π×60≈377），offset 依次衔接无误。
4. **主题持久化健壮**：`localStorage('mc-theme')` 读写均包 try/catch，隐私模式不炸。
5. **表单基础好**：真 `<form>` submit、`autocomplete="username"/"current-password"`（spec 没吹但做了）、密码显隐即点即切。
6. **图表零依赖**：纯 SVG 自适应 viewBox，深浅主题下均清晰。
7. **响应式断点与 spec 一致**：1100px KPI 2 列、820px 侧栏抽屉化均落地。

---

## 三、问题清单

### P1 — 宣称与实际不符（复盘核心）

| # | 问题 | 证据 |
|---|---|---|
| 1 | **侧边导航键盘不可达**。spec 称"键盘友好：Tab 顺序合理"，但 8 个 `nav-item` 全是 `<div>`（L380-387）：无 tabindex、无 role、无 aria-current，Tab 直接跳过，Enter 无法激活 | L380-387, L591 |
| 2 | **对比度宣称 ≥AA 不成立**。宣称只对 text-1 成立；辅助文字 text-3 两主题都不达标（实测见下表）。KPI 标签"在线 GPU 卡数"、面包屑、表头、节点百分比全在用 text-3 | L38, L55 |
| 3 | **`[hidden]` 被作者样式覆盖**。`.app { display:flex }`（L164）优先级高于 UA 的 `[hidden]{display:none}`，登录前控制台 DOM 实际已渲染（只是被推到视口外 + body overflow:hidden 挡住）。屏幕阅读器会读到，Tab 会焦点逃逸进不可见区域 | L164, L372 |

### P1 — 功能性 bug

| # | 问题 | 证据 |
|---|---|---|
| 4 | **主题 FAB 遮挡头像**。`theme-fab` 是 `position:fixed; top:22px; right:22px; z-index:5`（L153-154），登录后未隐藏，正好压在 topbar 的 avatar（"A"）上——4 张仪表盘截图里头像消失就是这个原因 | L153, 截图右上角 |
| 5 | **顶栏主题按钮图标不随主题切换**。`#topTheme` 的月亮 SVG 写死（L409），`applyTheme()` 只更新 FAB 的 `#themeIcon`（L555）。浅色下出现"两个并排月亮"，深色下月亮+太阳并排，用户会以为按钮坏了 | L409, L555, 截图右上角 |

### P2 — 设计/视觉缺陷

| # | 问题 | 说明 |
|---|---|---|
| 6 | **作业数 badge 用危险红**（`--danger`，L185）。中性计数 342 配红色，第一语义是"842 个失败"。应改中性色或品牌蓝；红留给真正的告警数 | L185 |
| 7 | **移动端双汉堡**。≤820px 时 `back-btn`（L310, L316）与 `menuBtn`（L402）同时显示，两个一模一样的汉堡并排，一个开一个关；且 >820px 时 menuBtn 也常驻显示但点击无效 | L310, L401-402 |
| 8 | **浅色"已完成"徽章对比度 3.85:1**。`st.done` 品牌蓝对 12% 蓝软底，12px 字号低于 AA 4.5:1；且"完成"用品牌蓝而成功绿 `#16C784` 定义了却没用，状态语义自相矛盾 | L284, L21 |
| 9 | **环形图中心"总卡数"压线**。截图可见 11px 小字被 16px 弧带下缘裁切，中心文字块应整体上收或缩小环内径 | L490, 截图 |
| 10 | **小屏登录卡溢出不可滚**。body `overflow:hidden`（L72）+ login-screen `min-height:100vh`，<600px 高的竖屏下登录按钮可能被裁切且无法滚动 | L72, L82-86 |

### P3 — 小项

- 错误提示复用对勾图标变红（L625），语义错位，应换叉/叹号图标。
- sparkline 颜色硬编码 hex（L432 等），不随主题，深色下恰好可读但不可维护。
- spec 宣称"aria-label/title 提示"：顶栏按钮有 title ✓，但导航、avatar、segmented 无任何 aria ✗。

### 交付一致性

- 截图侧栏 badge 显示 **842**，源码是 **342**（L381）——截图来自旧版本，交付包内证据与产物不同步，复现/验收时会困惑。

---

## 四、对比度实测（WCAG 4.5:1 正文 / 3:1 大字）

| 前景 | 背景 | 实测 | 判定 |
|---|---|---|---|
| 浅色 text-3 `#8A97AD` | 输入框底 `#F4F7FC` | **2.73:1** | ✗ 差 1.65 倍 |
| 浅色 text-3 | 白卡 | **2.96:1** | ✗ |
| 深色 text-3 `#6E7C93` | 卡面 `#141C2B` | **4.05:1** | ✗（差临门一脚） |
| 浅色 st.done 品牌蓝 | 蓝色软底 | **≈3.85:1**（12px） | ✗ |
| 导航 badge 白字 | `--danger` 红 | **2.97:1**（11px） | ✗ |
| 深色 text-1 `#EAF0F9` | `#141C2B` | ≈14:1 | ✓（spec 这条属实） |

**修法建议**：浅色 text-3 提到 `#6B7A93`（≈4.5）或把用途收窄为纯装饰；深色 text-3 提到 `#7E8CA4`；badge 换底色或加深文字。

---

## 五、规格书宣称核对表

| spec 宣称 | 实际 |
|---|---|
| 令牌/圆角/间距/阴影三件套 | ✅ 全部属实 |
| 图表零依赖、环形图中心无遮挡 | ✅ / ⚠️ 中心小字压线 |
| 主题切换 localStorage 持久化 | ✅ |
| 响应式 1100/820 | ✅（但双汉堡 bug） |
| "文字主色对背景均 ≥AA" | ⚠️ 仅 text-1 成立，text-3 两主题不达标 |
| "所有交互元素有 aria-label/title" | ❌ 导航/avatar/seg 完全没有 |
| "键盘友好：Tab 顺序合理" | ❌ 导航不可达 + hidden 焦点逃逸 |

---

## 六、修复优先级与建议

**进入 React 移植前必须修（P1）**：
1. nav-item 改 `<button>` + `aria-current="page"`；`#app` 显隐改 class 控制（`.app[data-show=false]{display:none}`）而非裸 hidden。
2. FAB 登录后隐藏（进入 app 时 `themeFab.style.display='none'`），主题切换只保留顶栏一个入口，且 `applyTheme` 同步更新两处图标。
3. text-3 两主题各提亮至 ≥4.5:1（见上表数值）。

**P2 随第一批组件化一起修**：badge 换中性色、移动端删 back-btn 只留 menuBtn、done 状态统一用成功绿、环形图中心字号微调。

**流程改进**：交付包中的截图必须从最终版 HTML 重新生成（本次 842/342 不同步即因此）；"可访问性"宣称下次先跑 axe 再写进 spec。

---

## 七、v1.2 修复闭合（后续追加）

### 7.1 P2/P3 收尾

| # | 原问题 | 修复方式 |
|---|---|---|
| 11 | 非交互卡片 hover 上移（误导可点击） | 去掉无 href/click 卡的 hover transform，仅保留阴影过渡 |
| 12 | 触摸目标过小 | 保证 `.icon-btn` / `.avatar` / `.nav-item` / `.pg` 均 ≥24px；图标按钮统一 42px |
| 13 | 主图表 `preserveAspectRatio="none"` 拉伸文字 | 重构为 `.chart-wrap`：SVG 仅折线+网格，`Y`/`X` 轴用 HTML 绝对定位渲染 |
| 14 | 表格小屏溢出 | 加 `.table-wrap{overflow-x:auto}`，次要列 `.col-opt` 在 ≤640px 隐藏 |
| 15 | 进度条/环形图缺少无障碍语义 | 所有 `.track` 加 `role="progressbar"`+`aria-valuenow/min/max`；安全环加 `aria-label` |
| 16 | 登录错误提示只有 Toast | 新增 `<div class="login-err" role="alert">` 内联错误位 |
| 17 | 装饰 sparkline 被读屏 | 加 `aria-hidden="true"` |
| 18 | 搜索框无 `aria-label`、无快捷键 | 加 `aria-label`；实现 `Ctrl/Cmd+K` 聚焦 |
| 19 | 数字未等宽，表格列跳动 | 加 `.tabular-nums` / `font-variant-numeric: tabular-nums` |
| 20 | 错误 Toast 时长与图标 | 错误态用专用叹号图标并延长至 3.2s |
| 21 | 缺少 Skip Link | 页面首个 Tab 项出现「跳转到主内容」，跳转 `#main` |
| 22 | 移动抽屉无焦点陷阱 | Tab 焦点在抽屉内循环，Esc/遮罩/关闭后归还 `.menuBtn` |

### 7.2 7 个模块页面补齐

已全部按同一设计系统实现：

- **作业管理**：状态分段筛选 + 时间范围 + 表格 + 分页
- **资源管理**：GPU/CPU/存储三池卡片 + 算力分配堆叠条 + 租户配额进度
- **集群**：节点卡片网格 + GPU/CPU/内存三段进度 + 在线/低负载/隔离状态徽章
- **监控**：外置轴折线图 + 关键指标卡片 + 告警时间线
- **租户**：配额总览 + 成员分布 + 租户列表
- **安全**：安全评分环形图 + 策略开关 + 审计日志时间线
- **设置**：外观/通知/调度策略三卡片，含主题选择、开关、下拉框

### 7.3 浏览器内无障碍回归审计

axe-core 离线不可用，自研 `ui-a11y-audit.mjs`（CDP / 真实 Chromium），覆盖：

- `lang` / `<title>` / 地标（main/nav/banner）/ 标题层级
- 所有可交互元素的可访问名称
- 所有表单控件关联标签
- 所有 `img` / `role=img` 文本替代
- 可见文本逐节点对比度（WCAG AA）
- 触摸目标 ≥24px
- `aria-hidden` + 可聚焦冲突

**最终报告：总计 0 项 · 错误 0 · 警告 0**（8 个页面分别扫描，exit code 0）

### 7.4 规格书同步

`metaclouds-ui-design-spec.md` 已升级到 **v1.2**，并修正：

- `--brand-grad` 渐变尾由 `#7C5CFF` 调深至 `#5B3FD9`，解决白字在渐变浅端约 4.29:1 的临界风险
- 主内容区加 `role="main"`
- 新增所有模块级组件的文档条目

### 7.5 验证截图

`ui-login-light.png` / `ui-login-dark.png` / `ui-dashboard-light.png` / `ui-dashboard-dark.png` 已从最终版重生成；另补充 `mod-*.png` 7 张模块页面截图用于目检。

---

## 七、v1.1 修复状态（2026-09-04 已全部落地）

按上表优先级完成修复，产物 `metaclouds-ui-prototype.html` 升级至 v1.1，规格书同步至 v1.1 并补修订记录。

| 编号 | 问题 | 状态 |
|---|---|---|
| P1-1 | 导航键盘不可达 | ✅ 8 项改语义 `<button>` + `aria-current`，nav 加 `aria-label` |
| P1-2 | 对比度不达标 | ✅ 引入 `--*-fg` 语义文字版令牌；text-3 浅 `#647189`(5.0:1) / 深 `#7E8CA4`(5.0:1) |
| P1-3 | `[hidden]` 被覆盖 | ✅ 全局 `[hidden]{display:none!important}` |
| P1-4 | FAB 遮挡头像 | ✅ 登录后 `$('#themeFab').style.display='none'`，焦点移交主标题 |
| P1-5 | 顶栏主题图标写死 | ✅ 新增 `#topThemeIcon`，`applyTheme` 同步两处图标 |
| 焦点不可见 | — | ✅ 补 `:focus-visible` 焦点环 |
| P2-6 | badge 误导用红 | ✅ 改品牌蓝 `#1E50E0`（白字 6.4:1） |
| P2-7 | 移动端双汉堡 | ✅ 桌面隐藏 menuBtn；展开后汉堡→X；新增遮罩 + Esc 关闭 |
| P2-8 | done 语义/对比度 | ✅ 改成功绿 `--success-fg` |
| P2-9 | 中心字压弧带 | ✅ 字号 26→24px，补显式 line-height 与 padding 微调 |
| P2-10 | 小屏不可滚 | ✅ `overflow:hidden` → `overflow-x:hidden` |
| P3 | 错误图标复用对勾 | ✅ 新增叹号圈图标 `ICON_ERR` |
| P3 | sparkline 硬编码 | ✅ 令牌化（**踩坑**：`stroke="var(--x)"` 属性形式不解析，须用 `style="stroke:var(--x)"`） |
| 一致性 | 截图与源码不同步 | ✅ 用 CDP 脚本从最终版重新生成 4 张截图，运行时 0 错误 |

**验证方式**：`ui-fix-smoke.cjs` —— JS 语法 OK、5 类标签全部配平、20 项修复落地核查全绿、无旧值残留；`login-ui-verify.mjs` 重跑 `PAGE_ERRORS=[]`。
**仍未验证**：axe 自动审计（离线无 axe-core），留待 React 实现后补跑。
