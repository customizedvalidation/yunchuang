# Metaclouds 前端 · UI 设计系统注入与换肤记录

> **目标**：用已定稿的 UI 原型（v1.2，Neumorphic-Flat）替代现有 React 前端外观。
> **方案**：设计系统注入 + 换肤（保留 antd 与全部业务功能，只换令牌 / 主题 / 皮肤层），
> 而不是重写页面逻辑——避免破坏角色过滤、RTK Query 绑定、移动抽屉等已验证功能。
> **状态**：已完成并通过构建 + 真实浏览器无障碍审计。

---

## 1 · 令牌权威源

原型 `metaclouds-ui-prototype.html`（v1.2）为唯一权威源，向两侧同步：

| 消费方 | 文件 | 说明 |
|---|---|---|
| TS 侧 | `src/theme/tokens.ts` | antd 主题、图表配色、内联样式 |
| CSS 侧 | `src/index.css`（`:root` / `[data-theme='dark']`） | 全站变量，组件样式实际驱动层 |

两侧必须保持一致——这是本次新增强制约定（原文件注释只写"请同步修改"，无校验手段，
现以构建产物 grep 作为核验手段，见第 5 节）。

---

## 2 · 改动清单

### 2.1 语义色（对齐原型）
| 令牌 | 原值 | 新值 |
|---|---|---|
| `--mc-success` | `#12b886` | `#16c784` |
| `--mc-warning` | `#f59e0b` | `#ffb020` |
| `--mc-danger` | `#f0445a` | `#ff5c7a` |

### 2.2 中性色（对比度修复 · 真实缺陷）
| 令牌 | 原值 | 新值 | 对比度 |
|---|---|---|---|
| 浅色 `--mc-text-3` | `#7c8da6`（2.9:1） | `#647189` | **5.0:1** |
| 深色 `--mc-text-3` | `#7c8da6`（4.05:1） | `#7e8ca4` | **5.0:1** |

### 2.3 新增「可读文字版」语义色（核心规则）
> **规则**：语义色必须成对导出。原色（`--mc-success` 等）只用于图表线条、图标填充、
> 进度条、状态点；凡承载文字一律用 `-fg` 版。原色作文字在浅色卡面上普遍只有 2~4:1。

| 令牌 | 浅色 | 深色 |
|---|---|---|
| `--mc-success-fg` | `#0b7a54` (5.4:1) | `#3ddc97` (9.7:1) |
| `--mc-warning-fg` | `#8a5a00` (5.4:1) | `#ffc24d` (11.0:1) |
| `--mc-danger-fg` | `#c2185b` (5.0:1) | `#ff7a93` (6.8:1) |
| `--mc-teal-fg` | `#00796b` (4.9:1) | `#2fd3c4` (9.1:1) |
| `--mc-brand-fg` | `#1e50e0` (6.0:1) | `#7aa2ff` (~7:1) |
| `--mc-violet-fg` | `#5b3fd9` (6.2:1) | `#b39dff` (~7:1) |

TS 侧对应 `semanticFgLight` / `semanticFgDark` / `getSemanticFg(mode)` /
`statusColorFg`（CSS 变量形式，自动随主题切换）。

### 2.4 品牌渐变（白字对比度修复）
`--mc-brand-grad` = `linear-gradient(135deg, #2F6BFF 0%, #5B3FD9 100%)`
浅端由 `#7C5CFF` 调深至 `#5B3FD9`——原浅端白字仅约 4.29:1，未达 AA；现整条渐变 ≥4.6:1。
已接入：主按钮、头像、登录页主渐变、侧栏折叠按钮。

### 2.5 形态令牌
- **圆角**上移一档（原型梯队 12/16/22/28）：`xs6 sm8 md12 lg16 xl22`
- **新增拟态阴影**：`--mc-shadow-raised` / `-raised-sm` / `-inset`（暗侧投影 + 亮侧高光）
  TS 侧：`shadowRaisedLight` / `shadowRaisedDark`

### 2.6 皮肤层（新增，位于 `src/index.css` 末尾）
antd 5 的 CSS-in-JS 在运行时注入、晚于打包 CSS，故统一用 `body .ant-*` 提升一级特异性：
- 卡片：拟态阴影 + 大圆角；**非交互卡片不做 hover 上移**（原型 v1.2 结论，会误导可点击）
- 主按钮：品牌渐变
- 表格表头：提亮后的 `text-3`
- Tag：文字换 `-fg` 版、底色保留柔和调
- 进度条 / 分段控件 / 弹窗 / 抽屉：圆角与凹槽观感对齐原型

### 2.7 组件级修正
| 文件 | 改动 |
|---|---|
| `src/index.css` | `.mc-status` 文字改用 `-fg`（原色作文字仅 ~2.9:1） |
| `src/components/StatusCell/index.tsx` | 回退分支文字用 `statusColorFg`、色点仍用原色 |
| `src/theme/antdTheme.ts` | `colorSuccess/Warning/Error` 填 `-fg` 版（否则 antd Tag/Alert 文字仅 ~2.3:1） |
| `src/theme/sidebar.css` / `theme/sidebar.ts` | 调色板由 Tailwind 默认色（`#3b82f6`/`#8b5cf6`/`#ec4899`）统一为品牌色；退出按钮红色改 `#ff5c7a`；菜单圆角 12/8 |
| `src/components/Sidebar/Sidebar.css` | 硬编码 `rgba(59,130,246,*)` / `rgba(96,165,250,*)` → 品牌蓝 |
| `src/components/Topbar/Topbar.css` | 头像改品牌渐变 + 方圆形（30px / r10）；图标按钮 34→38px |
| `src/pages/Login.tsx` | 主渐变改用 `brand.grad`；根元素 `div` → `<main aria-label="登录">` |
| `src/pages/Dashboard.tsx` | 硬编码 `#12b886`/`#f0445a` → 设计系统值 |

---

## 3 · 无障碍实测（真实 Chromium，非静态推断）

审计工具：`ui-a11y-audit.mjs`（CDP 驱动；axe-core 离线不可用，自研等价检查）
覆盖：对比度（按计算样式实算）、可访问名称、表单标签、地标、标题层级、触摸目标。

```
目标：http://localhost:4173/login（构建产物，vite preview）
总计 2 项 · 错误 0 · 警告 2     exit 0
```

两条警告为 `landmark-nav` / `landmark-banner`——独立登录页本无页头与导航，属合理豁免。
已同步修正审计脚本：banner/nav 地标属最佳实践而非 WCAG 硬性失败，由 error 降为 warning；
并修复汇总计数（原 `warn` 与 `warning` 键名不一致，导致"总计 2 项 · 警告 0"的自相矛盾输出）。

---

## 4 · 验证方法

| 环节 | 命令 / 手段 | 结果 |
|---|---|---|
| 类型检查 | `tsc --noEmit` | 0 错误 |
| 构建 | `npm run build` | `✓ built in 32.22s`，0 错误 |
| 产物核验 | grep 构建 CSS 中的新令牌与色值 | 9 个新令牌、8 个关键色值全部命中 |
| 运行时 | CDP 捕获 `Runtime.exceptionThrown` / `Log` | 仅 `/api/*` 500（后端未启动的代理探测），非 UI 缺陷 |
| 主题 | 注入 `localStorage['mc-theme-mode']` 后重导航 | `data-theme` = light / dark 均正确切换 |
| 无障碍 | `ui-a11y-audit.mjs` | 0 错误 |

---

## 5 · 遗留与后续

1. **登录态页面（Dashboard 等 8 个模块）未做目检**——需后端启动、真实登录后截图核对；
   本轮仅完成登录页目检（浅/深）与代码层核验。令牌为全局注入，已覆盖这些页面。
2. **ECharts 图表配色**走 `chartPalette`（仍为原色）——符合规则（图表线条非文字），
   但图例文字若在浅色卡面上用原色仍需改 `-fg`，建议接入时逐图确认。
3. 审计脚本现支持传入任意 URL：
   `node ui-a11y-audit.mjs "http://localhost:4173/<path>"`，建议纳入回归门禁。
4. 原型侧仍有一份独立交付（`metaclouds-ui-prototype.html` + 规格书），
   前端令牌变更须回抄规格书，否则再次出现"文档说 A、产物是 B"。
