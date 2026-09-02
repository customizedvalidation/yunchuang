import { theme, type ThemeConfig } from 'antd';
import { brand, semantic, neutralLight, neutralDark, radius, shadow, spacing, type ThemeMode } from './tokens';

/**
 * 生成 antd 5.x 主题配置。
 * 深色模式使用 antd 的 darkAlgorithm 自动派生对比色，
 * 再叠加品牌色与组件级令牌，保证与自定义 CSS 变量一致。
 */
export const getThemeConfig = (mode: ThemeMode, density: DensityMode = 'comfortable'): ThemeConfig => {
  const n = mode === 'dark' ? neutralDark : neutralLight;
  const isDark = mode === 'dark';

  const modeAlgo = isDark ? theme.darkAlgorithm : theme.defaultAlgorithm;
  const densityAlgo = density === 'compact' ? theme.compactAlgorithm : null;

  return {
    algorithm: densityAlgo ? [modeAlgo, densityAlgo] : modeAlgo,
    token: {
      colorPrimary: isDark ? brand[400] : brand[500],
      colorSuccess: semantic.success,
      colorWarning: semantic.warning,
      colorError: semantic.danger,
      colorInfo: isDark ? brand[400] : brand[500],

      colorTextBase: n.text1,
      colorBgLayout: n.bg,
      colorBgContainer: n.surface,
      colorBgElevated: n.surface,
      colorBorder: n.line,
      colorBorderSecondary: n.line,

      borderRadius: radius.md,
      borderRadiusLG: radius.lg,
      borderRadiusSM: radius.sm,

      fontSize: 14,
      fontFamily:
        'Inter, "HarmonyOS Sans SC", "PingFang SC", "Microsoft YaHei", -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',

      controlHeight: 34,
      controlHeightLG: 42,
      controlHeightSM: 28,

      boxShadowSecondary: shadow[3],
      boxShadowTertiary: shadow[1],

      motionDurationFast: '0.12s',
      motionDurationMid: '0.18s',
      motionDurationSlow: '0.28s',
      motionEaseInOut: 'cubic-bezier(.2,.8,.2,1)',

      wireframe: false,
    },
    components: {
      Table: {
        headerBg: n.surface2,
        headerColor: n.text3,
        headerSplitColor: 'transparent',
        headerBorderRadius: 0,
        rowHoverBg: n.surface3,
        borderColor: n.line,
        cellPaddingBlock: 12,
        cellPaddingInline: 16,
        cellFontSize: 13.5,
        rowSelectedBg: isDark ? 'rgba(91,139,255,.10)' : brand[50],
        rowSelectedHoverBg: isDark ? 'rgba(91,139,255,.14)' : brand[50],
      },
      Card: {
        borderRadiusLG: radius.lg,
        paddingLG: spacing[5],
        boxShadowTertiary: shadow[1],
        colorBorderSecondary: n.line,
      },
      Layout: {
        siderBg: '#0A1020',
        headerBg: n.surface,
        headerHeight: 56,
        headerPadding: '0 24px',
        bodyBg: n.bg,
      },
      Menu: {
        darkItemBg: 'transparent',
        darkSubMenuItemBg: 'transparent',
        darkItemSelectedBg: 'rgba(47,107,255,.18)',
        darkItemHoverBg: 'rgba(255,255,255,.07)',
        darkItemColor: '#A9B8D0',
        darkItemSelectedColor: '#FFFFFF',
        itemBorderRadius: radius.md,
        itemHeight: 40,
        itemMarginInline: 4,
        groupTitleColor: '#6F7F9B',
        groupTitleFontSize: 11,
      },
      Button: {
        primaryShadow: 'none',
        fontWeight: 500,
        paddingInline: 16,
      },
      Statistic: {
        contentFontSize: 28,
        titleFontSize: 13,
      },
      Tag: {
        borderRadiusSM: radius.sm,
        defaultBg: n.surface3,
      },
      Tabs: {
        horizontalItemGutter: 24,
        titleFontSize: 14,
        cardGutter: 2,
      },
      Modal: {
        borderRadiusLG: radius.xl,
        boxShadowTertiary: shadow[4],
      },
      Dropdown: {
        borderRadiusLG: radius.lg,
        paddingBlock: 4,
      },
      Input: {
        activeShadow: `0 0 0 3px ${isDark ? 'rgba(91,139,255,.18)' : brand[50]}`,
      },
      Select: {
        optionSelectedBg: isDark ? 'rgba(91,139,255,.16)' : brand[50],
      },
    },
  };
};
