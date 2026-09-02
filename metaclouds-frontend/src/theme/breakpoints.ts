/**
 * 响应式断点单一真相源（PRD R1）。
 *
 * 三处必须同步（改一处须同步另外两处）：
 *   1. 本文件（TS 常量，供 JS 逻辑与类型使用）
 *   2. tailwind.config.js 的 `theme.screens`
 *   3. src/index.css 的 `@media` 查询 + `--mc-sider-w*` 变量
 *
 * 采用 min-width 移动优先（mobile-first）策略。
 */
export const BREAKPOINTS = {
  xs: 0,
  sm: 576,
  md: 768,
  lg: 1024,
  xl: 1280,
  '2xl': 1440,
  '3xl': 1920,
  '4xl': 2560,
} as const;

export type BreakpointToken = keyof typeof BREAKPOINTS;

/**
 * 移动端上限：< 1024px 视为移动端，侧边栏以浮层 Drawer 呈现；
 * ≥ 1024px 侧边栏常驻（lg 为 64px 图标轨，xl+ 为 240px 展开）。
 * 与 PRD 决策 Q3（A）一致。
 */
export const MOBILE_MAX = 1023;

/** 侧边栏宽度令牌，与 src/index.css 的 --mc-sider-w* 同步 */
export const SIDER_WIDTH = {
  expanded: 240,
  collapsed: 64,
  rail: 64, // lg(1024-1279) 图标轨
  drawer: 280, // <1024 浮层宽度
} as const;
