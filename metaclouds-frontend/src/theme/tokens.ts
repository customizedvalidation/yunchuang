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
} as const;

/** 语义色 */
export const semantic = {
  success: '#12B886',
  warning: '#F59E0B',
  danger: '#F0445A',
  teal: '#00B8A9',
  violet: '#7C5CFF',
  muted: '#7C8DA6',
} as const;

/** 中性色（浅色模式） */
export const neutralLight = {
  text1: '#0E1726',
  text2: '#4A5B75',
  text3: '#7C8DA6',
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
  text3: '#7C8DA6',
  line: '#22304A',
  lineStrong: '#2E3F5E',
  surface: '#111A2B',
  surface2: '#162135',
  surface3: '#0F1728',
  bg: '#0A1120',
} as const;

/** 圆角 */
export const radius = {
  xs: 4,
  sm: 6,
  md: 8,
  lg: 12,
  xl: 16,
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

/** 阴影（由静到动） */
export const shadow = {
  1: '0 1px 2px rgba(14,23,38,.06)',
  2: '0 2px 6px rgba(14,23,38,.06)',
  3: '0 8px 24px rgba(14,23,38,.08)',
  4: '0 18px 48px rgba(14,23,38,.16)',
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
};
