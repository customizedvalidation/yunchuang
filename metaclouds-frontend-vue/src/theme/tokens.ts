/**
 * ============================================================
 * Metaclouds Design System (MDS) v3.0 — TypeScript Design Tokens
 * ============================================================
 *
 * 【强制同步约定 · FORCE-SYNC】
 * 本文件是 CSS 自定义属性（`src/styles/index.css` 中 `:root`）的 TS 镜像，
 * 仅服务于无法直接消费 CSS 变量的 JS/TS 场景（ECharts option、业务常量、文档）。
 *
 * 权威源（single source of truth）：`src/styles/index.css` 的 `:root`（light）
 * 与 `html.dark[data-theme='dark']`（dark）。
 * 工作纪律：
 *   1. 任何颜色 / 间距 / 圆角 / 阴影 / 动效 / 字号的增改，必须「先改 index.css，
 *      再同步本文件」，二者键名与取值逐字一致；dark 同理镜像暗色块。
 *   2. 禁止在本文件引入 index.css 中不存在的新色值；新令牌先入 CSS。
 *   3. 组件内联样式优先用 `var(--mc-*)`，本文件导出仅给 ECharts / 图表 / 纯逻辑使用。
 *
 * v3.0：单一主色真相、去装饰化阴影、8pt 间距、完整排版阶梯。
 * v3.1：新增暗色主题。复盘实测 15 个业务页 <style scoped> 的硬编码色值为 0，
 *       已全量 token 化，故暗色只需覆盖 CSS 令牌；本文件新增 darkColorTokens
 *       镜像供 ECharts 在 JS 侧取色（ECharts 无法读取 CSS 变量）。
 *       EP 的 --el-* 暗色值仅在 CSS 中定义，TS 侧无消费者，故不镜像。
 * ============================================================
 */

/* ------------------------------------------------------------
 * 颜色令牌
 * ---------------------------------------------------------- */
export interface ColorTokens {
  /** 品牌色板 scale（主色 = base，深档 600/700/800，浅档 400/100/50） */
  brand: {
    /** 主色科技蓝（色块 / 图表 / 焦点环，非文字） */
    base: string;
    '50': string;
    '100': string;
    '400': string;
    '600': string;
    '700': string;
    '800': string;
    /** 同色系渐变，仅用于品牌徽标 / 登录装饰，不用于按钮 */
    grad: string;
    /** 主色可读文字版（白底 5.6:1，同时是 --el-color-primary） */
    fg: string;
    /** 文字别名，语义更明确 */
    text: string;
    /** 浅底高亮（选中 / 次级按钮 hover） */
    subtle: string;
    /** 聚焦环外扩柔光 */
    ring: string;
  };
  /** 语义色：每个状态成对提供 solid（色块）与 fg（软底上的可读文字） */
  semantic: {
    success: string;
    successSoft: string;
    successFg: string;
    warning: string;
    warningSoft: string;
    warningSurface: string;
    warningFg: string;
    danger: string;
    dangerSoft: string;
    dangerFg: string;
    dangerStrong: string;
    info: string;
    infoSoft: string;
    infoFg: string;
    teal: string;
  };
  /** 中性色阶：文字 / 描边 / 表面 / 背景 */
  neutral: {
    text1: string;
    text2: string;
    text3: string;
    line: string;
    lineSubtle: string;
    lineStrong: string;
    surface: string;
    surface2: string;
    surface3: string;
    bg: string;
  };
  /** 侧边栏专用科技蓝（v3 起与品牌蓝同色相） */
  sidebar: {
    primary: string;
    primaryLight: string;
    primaryDark: string;
    accent: string;
  };
  /** 图表第 9 系列色 */
  chartAccent: string;
  /** 焦点环颜色 */
  focusRing: string;
}

export const colorTokens: ColorTokens = {
  brand: {
    base: '#2f6bff',
    '50': '#eef4ff',
    '100': '#dce8ff',
    '400': '#5b8bff',
    '600': '#1f52e0',
    '700': '#1a44b8',
    '800': '#163694',
    grad: 'linear-gradient(135deg, #3b7bff 0%, #1f52e0 100%)',
    fg: '#1e50e0',
    text: 'var(--mc-brand-fg)',
    subtle: '#eff4ff',
    ring: 'rgba(47, 107, 255, 0.16)',
  },
  semantic: {
    success: '#16c784',
    successSoft: 'rgba(22, 199, 132, 0.12)',
    successFg: '#0b7a54',
    warning: '#ffb020',
    warningSoft: 'rgba(255, 176, 32, 0.12)',
    warningSurface: '#fff8ec',
    warningFg: '#8a5a00',
    danger: '#ff5c7a',
    dangerSoft: 'rgba(255, 92, 122, 0.12)',
    dangerFg: '#c2185b',
    dangerStrong: '#9b1349',
    info: '#55647e',
    infoSoft: '#eef4ff',
    infoFg: '#3a4a63',
    teal: '#00b8a9',
  },
  neutral: {
    text1: '#0e1726',
    text2: '#44536c',
    text3: '#5b6880',
    line: '#e3e8f0',
    lineSubtle: '#eef1f6',
    lineStrong: '#cbd5e5',
    surface: '#ffffff',
    surface2: '#fafbfe',
    surface3: '#f4f7fc',
    bg: '#f6f8fc',
  },
  sidebar: {
    primary: '#2f6bff',
    primaryLight: '#69b1ff',
    primaryDark: '#1f52e0',
    accent: '#8fc0ff',
  },
  chartAccent: '#69b1ff',
  focusRing: '#2f6bff',
};

/* ------------------------------------------------------------
 * 暗色令牌镜像 — 对应 index.css `html.dark[data-theme='dark']`
 * 仅供 ECharts / JS 逻辑取色，组件样式一律用 CSS 变量。
 * 【FORCE-SYNC】暗色块改色后必须同步本对象，键名逐字一致。
 * ---------------------------------------------------------- */
export const darkColorTokens: ColorTokens = {
  brand: {
    base: '#4f7dff',
    '50': '#16233c',
    '100': '#2a3f6b',
    '400': '#7aa2ff',
    '600': '#3d6fe8',
    '700': '#2f5fd6',
    '800': '#264bb0',
    grad: 'linear-gradient(135deg, #4f7dff 0%, #2f5fd6 100%)',
    fg: '#3d6fe8',
    text: '#8aabff',
    subtle: '#16294a',
    ring: 'rgba(91, 139, 255, 0.28)',
  },
  semantic: {
    success: '#22d394',
    successSoft: 'rgba(34, 211, 148, 0.16)',
    successFg: '#57dba6',
    warning: '#ffbe3d',
    warningSoft: 'rgba(255, 190, 61, 0.16)',
    warningSurface: '#2a2113',
    warningFg: '#ffcc70',
    danger: '#ff7091',
    dangerSoft: 'rgba(255, 112, 145, 0.16)',
    dangerFg: '#ff9ab0',
    dangerStrong: '#ffc0cd',
    info: '#8fa1bb',
    infoSoft: '#1c2a3f',
    infoFg: '#b6c6dc',
    teal: '#2fd0bf',
  },
  neutral: {
    text1: '#e9eff8',
    text2: '#b8c5d8',
    text3: '#93a3ba',
    line: '#2a3648',
    lineSubtle: '#202b3b',
    lineStrong: '#3a4a60',
    surface: '#141d2a',
    surface2: '#182231',
    surface3: '#1e2a3b',
    bg: '#0b111c',
  },
  sidebar: {
    // 侧边栏在两种主题下均为「有意固定深色」的品牌导航，令牌保持不变
    primary: '#2f6bff',
    primaryLight: '#69b1ff',
    primaryDark: '#1f52e0',
    accent: '#8fc0ff',
  },
  chartAccent: '#7aa2ff',
  focusRing: '#7aa2ff',
};

/** 暗色阴影（对应 index.css 暗色块 --mc-shadow-*），ECharts tooltip 使用 */
export const darkShadowTokens = {
  1: '0 1px 2px rgba(0, 0, 0, 0.36)',
  2: '0 1px 3px rgba(0, 0, 0, 0.44), 0 1px 2px rgba(0, 0, 0, 0.32)',
  3: '0 4px 12px rgba(0, 0, 0, 0.48), 0 2px 4px rgba(0, 0, 0, 0.32)',
  4: '0 12px 32px rgba(0, 0, 0, 0.56), 0 4px 8px rgba(0, 0, 0, 0.36)',
};

/* ------------------------------------------------------------
 * 布局令牌：间距 / 圆角 / 阴影 / 动效 / 字号 / 行高 / 字重 / 字体 / 层级
 * ---------------------------------------------------------- */
export interface LayoutTokens {
  radius: { xs: string; sm: string; md: string; lg: string; xl: string; pill: string };
  /** v3 单向极轻阴影；raisedSm/raised 为兼容旧引用的别名 */
  shadow: { 1: string; 2: string; 3: string; 4: string; raisedSm: string; raised: string };
  motion: { ease: string; easeOut: string; fast: string; base: string; slow: string };
  spacing: {
    space1: string; space2: string; space3: string; space4: string;
    space5: string; space6: string; space8: string; space10: string;
    space12: string; space16: string;
    gutter: string; gap: string; siderW: string; siderWCollapsed: string; contentMax: string;
  };
  fontSize: {
    xs: string; sm: string; body: string; h2: string; h1: string;
    md: string; lg: string; xl: string; '2xl': string; '3xl': string;
  };
  lineHeight: { tight: string; snug: string; normal: string; relaxed: string };
  fontWeight: { regular: string; medium: string; semibold: string; bold: string };
  font: { sans: string; mono: string };
  zIndex: { sticky: string; overlay: string; modal: string; toast: string; skip: string };
}

export const layoutTokens: LayoutTokens = {
  radius: {
    xs: '6px',
    sm: '8px',
    md: '12px',
    lg: '16px',
    xl: '22px',
    pill: '999px',
  },
  shadow: {
    1: '0 1px 2px rgba(16, 24, 40, 0.04)',
    2: '0 1px 3px rgba(16, 24, 40, 0.06), 0 1px 2px rgba(16, 24, 40, 0.04)',
    3: '0 4px 12px rgba(16, 24, 40, 0.06), 0 2px 4px rgba(16, 24, 40, 0.04)',
    4: '0 12px 32px rgba(16, 24, 40, 0.1), 0 4px 8px rgba(16, 24, 40, 0.04)',
    raisedSm: 'var(--mc-shadow-2)',
    raised: 'var(--mc-shadow-3)',
  },
  motion: {
    ease: 'cubic-bezier(0.2, 0.8, 0.2, 1)',
    easeOut: 'cubic-bezier(0.16, 1, 0.3, 1)',
    fast: '120ms',
    base: '180ms',
    slow: '280ms',
  },
  spacing: {
    space1: '4px',
    space2: '8px',
    space3: '12px',
    space4: '16px',
    space5: '20px',
    space6: '24px',
    space8: '32px',
    space10: '40px',
    space12: '48px',
    space16: '64px',
    gutter: 'clamp(16px, 8px + 1.2vw, 48px)',
    gap: 'clamp(12px, 8px + 0.5vw, 24px)',
    siderW: '240px',
    siderWCollapsed: '64px',
    contentMax: '1600px',
  },
  fontSize: {
    xs: '11.5px',
    sm: 'clamp(12.5px, 12.3px + 0.06vw, 13.5px)',
    body: 'clamp(14px, 13.6px + 0.12vw, 15.5px)',
    h2: 'clamp(15px, 14.2px + 0.16vw, 17px)',
    h1: 'clamp(22px, 20px + 0.5vw, 30px)',
    md: '16px',
    lg: '18px',
    xl: '22px',
    '2xl': '28px',
    '3xl': '36px',
  },
  lineHeight: {
    tight: '1.25',
    snug: '1.45',
    normal: '1.6',
    relaxed: '1.75',
  },
  fontWeight: {
    regular: '400',
    medium: '500',
    semibold: '600',
    bold: '700',
  },
  font: {
    // v3.1：纯系统 UI 栈（与 index.css --mc-font-sans / --mc-font-mono 逐字一致）。
    // 不托管 Inter / JetBrains Mono，避免「声明了但机器没装」造成的跨终端排版漂移。
    sans: "-apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', system-ui, 'HarmonyOS Sans SC', 'PingFang SC', 'Hiragino Sans GB', 'Microsoft YaHei', 'Noto Sans CJK SC', sans-serif",
    mono: "ui-monospace, 'SFMono-Regular', 'Cascadia Mono', 'Segoe UI Mono', Menlo, Monaco, Consolas, 'Liberation Mono', monospace",
  },
  zIndex: {
    sticky: '100',
    overlay: '1000',
    modal: '2000',
    toast: '3000',
    skip: '4000',
  },
};

/* ------------------------------------------------------------
 * Element Plus 变量映射（MDS → --el-*）
 * 与 index.css 中「Element Plus 核心变量映射」一一对应。
 * ---------------------------------------------------------- */
export interface ElTheme {
  colorPrimary: Record<string, string>;
  colorSuccess: Record<string, string>;
  colorWarning: Record<string, string>;
  colorDanger: Record<string, string>;
  colorError: Record<string, string>;
  colorInfo: Record<string, string>;
  text: Record<string, string>;
  border: Record<string, string>;
  fill: Record<string, string>;
  bg: Record<string, string>;
  radius: Record<string, string>;
  font: Record<string, string>;
}

export const elTheme: ElTheme = {
  colorPrimary: {
    base: colorTokens.brand.fg,
    light3: '#6285e9',
    light5: '#8fa8f0',
    light7: '#bccbf6',
    light8: '#d2dcf9',
    light9: '#e9eefc',
    dark2: '#1840b3',
  },
  colorSuccess: {
    base: colorTokens.semantic.successFg,
    light3: '#54a287',
    light5: '#85bdaa',
    light7: '#b6d7cc',
    light8: '#cee4dd',
    light9: '#e7f2ee',
    dark2: '#096243',
  },
  colorWarning: {
    base: colorTokens.semantic.warningFg,
    light3: '#ad8c4d',
    light5: '#c5ad80',
    light7: '#dcceb3',
    light8: '#e8decc',
    light9: '#f3efe6',
    dark2: '#6e4800',
  },
  colorDanger: {
    base: colorTokens.semantic.dangerFg,
    light3: '#d45d8c',
    light5: '#e18cad',
    light7: '#edbace',
    light8: '#f3d1de',
    light9: '#f9e8ef',
    dark2: '#9b1349',
  },
  colorError: {
    base: colorTokens.semantic.dangerFg,
    light3: '#d45d8c',
    light5: '#e18cad',
    light7: '#edbace',
    light8: '#f3d1de',
    light9: '#f9e8ef',
    dark2: '#9b1349',
  },
  colorInfo: {
    base: colorTokens.semantic.info,
    light3: '#8893a5',
    light5: '#aab2bf',
    light7: '#ccd1d8',
    light8: '#dde0e5',
    light9: '#eef0f2',
    dark2: '#445065',
  },
  text: {
    primary: colorTokens.neutral.text1,
    regular: colorTokens.neutral.text2,
    secondary: colorTokens.neutral.text3,
    placeholder: colorTokens.neutral.text3,
    disabled: '#a9b4c6',
  },
  border: {
    base: colorTokens.neutral.lineStrong,
    light: colorTokens.neutral.line,
    lighter: colorTokens.neutral.lineSubtle,
    extraLight: colorTokens.neutral.lineSubtle,
    dark: colorTokens.neutral.lineStrong,
    darker: '#aebbd0',
  },
  fill: {
    base: colorTokens.neutral.surface3,
    light: colorTokens.neutral.surface2,
    lighter: '#fafcfe',
    extraLight: '#f7f9fd',
    dark: '#e9eef6',
    blank: colorTokens.neutral.surface,
  },
  bg: {
    base: colorTokens.neutral.surface,
    page: colorTokens.neutral.bg,
    overlay: '#ffffff',
  },
  radius: {
    base: layoutTokens.radius.sm,
    small: layoutTokens.radius.xs,
    round: layoutTokens.radius.pill,
  },
  font: {
    family: 'var(--mc-font-sans)',
    sizeBase: 'var(--mc-fs-body)',
  },
};

/* ------------------------------------------------------------
 * ECharts 调色板
 * 顺序 = 系列取色顺序，全部来自 MDS 令牌（不在本文件引入孤立色值）。
 * ---------------------------------------------------------- */
export const chartPalette: string[] = [
  colorTokens.brand.base, // 1 品牌蓝
  colorTokens.semantic.teal, // 2 青
  colorTokens.semantic.success, // 3 成功绿
  colorTokens.semantic.warning, // 4 警告琥珀
  colorTokens.semantic.danger, // 5 危险玫红
  colorTokens.brand['400'], // 6 浅蓝
  colorTokens.semantic.info, // 7 info 石板蓝
  colorTokens.brand['600'], // 8 深蓝
  colorTokens.chartAccent, // 9 亮蓝
  colorTokens.neutral.text3, // 10 中性灰
];

export default colorTokens;
