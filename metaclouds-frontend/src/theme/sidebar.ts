/**
 * 侧边栏宽度真相源统一（R3-b）。
 *
 * 历史版本在此定义了与以下两处矛盾的宽度常量，并含 getSidebarWidth(window.innerWidth)
 * —— 后者违反 R2「零 JS 布局」原则（外壳布局已由 CSS Grid + 媒体查询驱动，组件内不得监听 window.innerWidth）：
 *   - src/theme/breakpoints.ts 的 SIDER_WIDTH（被 Sidebar.tsx 的 antd <Sider> 直接消费，是实时真相源）
 *   - src/theme/sidebar.css 的 --sidebar-width-* 变量（被断点媒体查询消费，需 !important 覆盖 antd Sider 内联 width）
 *
 * 现将本文件收敛为单一真相源的薄别名：宽度一律以 breakpoints.ts SIDER_WIDTH 与
 * --sidebar-width-* 变量为准；不再保留任何独立的宽度常量或 JS 宽度计算。
 * 仅保留色彩常量（若后续需要可在此扩展）。
 */
export { SIDER_WIDTH } from './breakpoints';

export const SIDEBAR_COLORS = {
  PRIMARY: '#3b82f6',
  PRIMARY_LIGHT: '#60a5fa',
  PRIMARY_DARK: '#2563eb',
  SECONDARY: '#8b5cf6',
  SECONDARY_LIGHT: '#a78bfa',
  ACCENT: '#ec4899',
};
