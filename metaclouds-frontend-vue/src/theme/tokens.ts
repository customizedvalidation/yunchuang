/**
 * ============================================================
 * Metaclouds Design System (MDS) v2.1 — TypeScript Design Tokens
 * ============================================================
 *
 * 【强制同步约定 · FORCE-SYNC】
 * 本文件是 CSS 自定义属性（`src/styles/index.css` 中 `:root { --mc-* }`）的 TS 镜像，
 * 仅服务于无法直接消费 CSS 变量的 JS/TS 场景（如 ECharts option、业务常量、文档）。
 *
 * 权威源（single source of truth）：`src/styles/index.css` 的 `:root`。
 * 工作纪律：
 *   1. 任何颜色 / 间距 / 圆角 / 动效 / 字号的新增或改值，必须「先改 index.css `:root`，
 *      再同步本文件」，二者键名与取值逐字一致。
 *   2. 禁止在本文件引入 index.css 中不存在的新色值；新令牌先入 `:root`。
 *   3. 组件内联样式请优先用 `var(--mc-*)`，本文件导出仅给 ECharts / 图表 / 纯逻辑使用。
 *
 * 配色基调：科技蓝主色，语义色成对（solid + -fg 可读文字版），中性灰阶分层。
 * ============================================================
 */

/* ------------------------------------------------------------
 * 颜色令牌
 * ---------------------------------------------------------- */
export interface ColorTokens {
  /** 品牌色板 scale（主色 = brand，深档 600/700/800，浅档 400/100/50） */
  brand: {
    /** 主色科技蓝（EP --el-color-primary 映射值） */
    base: string;
    '50': string;
    '100': string;
    '400': string;
    '600': string;
    /** v2.1 增补深档 */
    '700': string;
    /** v2.1 增补深档 */
    '800': string;
    /** 品牌渐变（主按钮背景） */
    grad: string;
    /** 主色可读文字版（用于浅底上的品牌文字） */
    fg: string;
  };
  /** 语义色：每个状态成对提供 solid（色块）与 fg（软底上的可读文字） */
  semantic: {
    success: string;
    successSoft: string;
    successFg: string;
    warning: string;
    warningSoft: string;
    warningFg: string;
    danger: string;
    dangerSoft: string;
    dangerFg: string;
    /** v2.1 增补 info 语义（solid / soft / fg 成对） */
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
    lineStrong: string;
    surface: string;
    surface2: string;
    surface3: string;
    bg: string;
  };
  /** 焦点环颜色（v2.1 增补） */
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
    grad: 'linear-gradient(135deg, #2f6bff 0%, #5b3fd9 100%)',
    fg: '#1e50e0',
  },
  semantic: {
    success: '#16c784',
    successSoft: 'rgba(22, 199, 132, 0.12)',
    successFg: '#0b7a54',
    warning: '#ffb020',
    warningSoft: 'rgba(255, 176, 32, 0.12)',
    warningFg: '#8a5a00',
    danger: '#ff5c7a',
    dangerSoft: 'rgba(255, 92, 122, 0.12)',
    dangerFg: '#c2185b',
    info: '#55647e',
    infoSoft: '#eef4ff',
    infoFg: '#3a4a63',
    teal: '#00b8a9',
  },
  neutral: {
    text1: '#0e1726',
    text2: '#4a5b75',
    text3: '#647189',
    line: '#e4eaf3',
    lineStrong: '#d2dceb',
    surface: '#ffffff',
    surface2: '#fafcfe',
    surface3: '#f4f7fc',
    bg: '#f5f7fc',
  },
  focusRing: '#2f6bff',
};

/* ------------------------------------------------------------
 * 布局令牌：间距 / 圆角 / 阴影 / 动效 / 字号
 * ---------------------------------------------------------- */
export interface LayoutTokens {
  radius: { xs: string; sm: string; md: string; lg: string; xl: string; pill: string };
  shadow: { 1: string; 2: string; 3: string; raisedSm: string; raised: string };
  motion: { ease: string; fast: string; base: string; slow: string };
  spacing: { gutter: string; gap: string; siderW: string; siderWCollapsed: string };
  fontSize: { h1: string; h2: string; body: string; sm: string };
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
    1: '0 1px 2px rgba(14, 23, 38, 0.06)',
    2: '0 2px 6px rgba(14, 23, 38, 0.06)',
    3: '0 8px 24px rgba(14, 23, 38, 0.08)',
    raisedSm:
      '6px 6px 14px rgba(14, 23, 38, 0.06), -6px -6px 14px rgba(255, 255, 255, 0.9)',
    raised:
      '10px 10px 24px rgba(14, 23, 38, 0.07), -10px -10px 24px rgba(255, 255, 255, 0.9)',
  },
  motion: {
    ease: 'cubic-bezier(0.2, 0.8, 0.2, 1)',
    fast: '120ms',
    base: '180ms',
    slow: '280ms',
  },
  spacing: {
    gutter: 'clamp(16px, 8px + 1.2vw, 48px)',
    gap: 'clamp(12px, 8px + 0.5vw, 24px)',
    siderW: '240px',
    siderWCollapsed: '64px',
  },
  fontSize: {
    h1: 'clamp(19px, 17.6px + 0.35vw, 26px)',
    h2: 'clamp(15px, 14.2px + 0.16vw, 17px)',
    body: 'clamp(13.5px, 13.2px + 0.08vw, 14.5px)',
    sm: 'clamp(12.5px, 12.3px + 0.06vw, 13.5px)',
  },
};

/* ------------------------------------------------------------
 * Element Plus 变量映射（MDS → --el-*）
 * 与 index.css 中「Element Plus 核心变量覆盖段」一一对应，
 * 供文档 / 后续按主题切换时程序化读取。EP 2.8 直接在 :root 覆盖即可全局生效。
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
}

export const elTheme: ElTheme = {
  colorPrimary: {
    // EP primary 取 brand-fg 深档（填充/文字双向可读）；CTA 按钮另叠加亮蓝渐变
    base: colorTokens.brand.fg,
    light3: '#6285e9',
    light5: '#8fa8f0',
    light7: '#bccbf6',
    light8: '#d2dcf9',
    light9: '#e9eefc',
    dark2: '#1840b3',
  },
  colorSuccess: {
    // EP 实心/按钮用 -fg 深档（白字可读），浅档由深档派生
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
    // EP error 与 danger 同源（-fg 深档）
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
    disabled: '#b6c0d0',
  },
  border: {
    base: colorTokens.neutral.line,
    light: colorTokens.neutral.line,
    lighter: '#eef2f8',
    extraLight: '#f4f7fc',
    dark: colorTokens.neutral.lineStrong,
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
};

/* ------------------------------------------------------------
 * ECharts 调色板（chartPalette）
 * 顺序 = 系列取色顺序：品牌蓝打头，语义色穿插，避免大面积高饱和蓝。
 * 与 MDS 色板同源，供 Dashboard 图表使用。
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
  '#69b1ff', // 9 侧栏亮蓝
  colorTokens.neutral.text3, // 10 中性灰
];

export default colorTokens;
