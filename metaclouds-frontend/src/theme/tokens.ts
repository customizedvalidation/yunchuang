/**
 * Metaclouds Design System (MDS) v2.0 — Design Tokens
 *
 * 说明：
 * - 这里存放「TS 侧」需要的令牌（antd 主题、图表配色、内联样式）。
 * - 「CSS 侧」的同名变量定义在 src/index.css 的 :root 与 [data-theme='dark'] 中，
 *   两者必须保持一致，改色时请同步修改。
 */

export type ThemeMode = 'light' | 'dark';

/** 品牌色 —— Quantum Blue 量子蓝 */
export const brand = {
  700: '#1B47C4',
  600: '#1F52E0',
  500: '#2F6BFF',
  400: '#5B8BFF',
  100: '#DCE8FF',
  50: '#EEF4FF',
  /** 品牌渐变（原型 v1.2 将浅端由 #7C5CFF 调深至 #5B3FD9，使白字在整条渐变上稳过 AA） */
  grad: 'linear-gradient(135deg, #2F6BFF 0%, #5B3FD9 100%)',
} as const;

/** 语义色（对齐 UI 原型 v1.2 色值） */
export const semantic = {
  success: '#16C784',
  warning: '#FFB020',
  danger: '#FF5C7A',
  teal: '#00B8A9',
  violet: '#7C5CFF',
  muted: '#7C8DA6',
} as const;

/**
 * 语义色「可读文字版」—— 原色仅够做图表/图标填充/进度条，
 * 承载文字时落在浅色卡面上普遍只有 2~4:1（不达标），故为徽章、状态、delta 等
 * 小字单独定义（浅色面均 ≥4.8:1、深色面 ≥6:1）。原色做文字属 a11y 缺陷，已修复。
 */
export const semanticFgLight = {
  success: '#0B7A54', // 5.4:1
  warning: '#8A5A00', // 5.4:1
  danger: '#C2185B', // 5.0:1
  teal: '#00796B', // 4.9:1
  brand: '#1E50E0', // 6.0:1
  violet: '#5B3FD9', // 6.2:1
} as const;

/** 深色面下的可读文字版（原型 v1.2 实测值，均为深色卡面上的实测对比度） */
export const semanticFgDark = {
  success: '#3DDC97', // 9.7:1
  warning: '#FFC24D', // 11.0:1
  danger: '#FF7A93', // 6.8:1
  teal: '#2FD3C4', // 9.1:1
  brand: '#7AA2FF', // ~7:1
  violet: '#B39DFF', // ~7:1
} as const;

/** 中性色（浅色模式） */
export const neutralLight = {
  text1: '#0E1726',
  text2: '#4A5B75',
  /** 原型 v1.2：由 #7C8DA6 提亮至 #647189，对卡面 5.0:1（原 2.9:1 不达 AA） */
  text3: '#647189',
  line: '#E4EAF3',
  lineStrong: '#D2DCEB',
  surface: '#FFFFFF',
  surface2: '#FAFCFE',
  surface3: '#F4F7FC',
  bg: '#F5F7FC',
} as const;

/** 中性色（深色模式） */
export const neutralDark = {
  text1: '#EAF0FA',
  text2: '#A9B8D0',
  /** 原型 v1.2：由 #7C8DA6 提亮至 #7E8CA4，对卡面 5.0:1（原 4.05:1 不达 AA） */
  text3: '#7E8CA4',
  line: '#22304A',
  lineStrong: '#2E3F5E',
  surface: '#111A2B',
  surface2: '#162135',
  surface3: '#0F1728',
  bg: '#0A1120',
} as const;

/**
 * 圆角 —— 对齐 UI 原型 v1.2（原型梯队：12 / 16 / 22 / 28）
 * 相比 v2.0 整体上调一档，呈现 Neumorphic-Flat 的柔和轮廓。
 */
export const radius = {
  xs: 6,
  sm: 8,
  md: 12,
  lg: 16,
  xl: 22,
  pill: 999,
} as const;

/** 间距（4pt 栅格） */
export const spacing = {
  1: 4,
  2: 8,
  3: 12,
  4: 16,
  5: 20,
  6: 24,
  8: 32,
  10: 40,
  12: 48,
} as const;

/**
 * 阴影
 * 1–4：通用层级阴影（v2.0 原有，保持兼容）
 * raised / raised-sm：原型 Neumorphic-Flat 的双侧阴影（暗侧投影 + 亮侧高光），
 *   用于卡片与容器；浅色的亮侧用白色、深色仅留极弱高光（避免脏）。
 */
export const shadow = {
  1: '0 1px 2px rgba(14,23,38,.06)',
  2: '0 2px 6px rgba(14,23,38,.06)',
  3: '0 8px 24px rgba(14,23,38,.08)',
  4: '0 18px 48px rgba(14,23,38,.16)',
} as const;

export const shadowRaisedLight = {
  raised: '10px 10px 24px rgba(14,23,38,.07), -10px -10px 24px rgba(255,255,255,.9)',
  raisedSm: '6px 6px 14px rgba(14,23,38,.06), -6px -6px 14px rgba(255,255,255,.9)',
  inset: 'inset 4px 4px 9px rgba(14,23,38,.05), inset -4px -4px 9px rgba(255,255,255,.75)',
} as const;

export const shadowRaisedDark = {
  raised: '12px 12px 28px rgba(0,0,0,.5), -10px -10px 24px rgba(255,255,255,.025)',
  raisedSm: '7px 7px 16px rgba(0,0,0,.45), -6px -6px 16px rgba(255,255,255,.025)',
  inset: 'inset 4px 4px 9px rgba(0,0,0,.45), inset -4px -4px 9px rgba(255,255,255,.03)',
} as const;

/** 动效 */
export const motion = {
  ease: 'cubic-bezier(.2,.8,.2,1)',
  fast: 120,
  base: 180,
  slow: 280,
} as const;

/** 图表配色（集中定义，避免散落硬编码） */
export const chartPalette = {
  gpu: brand[500],
  cpu: semantic.violet,
  success: semantic.success,
  warning: semantic.warning,
  danger: semantic.danger,
  teal: semantic.teal,
  grid: neutralLight.line,
  axis: neutralLight.text3,
} as const;

/** 按模式取可读文字版语义色（承载文字时必须用它，而非 semantic 原色） */
export const getSemanticFg = (mode: ThemeMode) =>
  mode === 'dark' ? semanticFgDark : semanticFgLight;

/** 按模式取中性色 */
export const getNeutral = (mode: ThemeMode) =>
  mode === 'dark' ? neutralDark : neutralLight;

/** 状态 → 语义色（用于 Tag / 状态点 / 文本） */
export const statusColor: Record<string, string> = {
  running: brand[500],
  pending: semantic.warning,
  completed: semantic.success,
  success: semantic.success,
  failed: semantic.danger,
  error: semantic.danger,
  critical: semantic.danger,
  cancelled: semantic.muted,
  warning: semantic.warning,
  info: brand[500],
  active: semantic.success,
  available: semantic.success,
  disabled: semantic.muted,
  inactive: semantic.muted,
  healthy: semantic.success,
  unhealthy: semantic.danger,
  paused: semantic.warning,
  scheduled: brand[500],
};

/**
 * 状态 → 可读文字色（CSS 变量形式，自动随深浅主题切换）
 * 承载文字时必须用它；statusColor 原色仅用于色点 / 图表 / 进度条。
 */
export const statusColorFg: Record<string, string> = {
  running: 'var(--mc-brand-fg)',
  pending: 'var(--mc-warning-fg)',
  completed: 'var(--mc-success-fg)',
  success: 'var(--mc-success-fg)',
  failed: 'var(--mc-danger-fg)',
  error: 'var(--mc-danger-fg)',
  critical: 'var(--mc-danger-fg)',
  cancelled: 'var(--mc-text-3)',
  warning: 'var(--mc-warning-fg)',
  info: 'var(--mc-brand-fg)',
  active: 'var(--mc-success-fg)',
  available: 'var(--mc-success-fg)',
  disabled: 'var(--mc-text-3)',
  inactive: 'var(--mc-text-3)',
  healthy: 'var(--mc-success-fg)',
  unhealthy: 'var(--mc-danger-fg)',
  paused: 'var(--mc-warning-fg)',
  scheduled: 'var(--mc-brand-fg)',
};

/** 状态 → 中文文案（禁止仅用颜色表意，必须色点 + 文字） */
export const statusText: Record<string, string> = {
  running: '运行中',
  pending: '排队中',
  completed: '已完成',
  success: '健康',
  failed: '失败',
  error: '失败',
  critical: '严重',
  cancelled: '已取消',
  warning: '警告',
  info: '提示',
  active: '已启用',
  available: '可用',
  disabled: '已停用',
  inactive: '未启用',
  healthy: '健康',
  unhealthy: '异常',
  paused: '已暂停',
  scheduled: '已调度',
};
