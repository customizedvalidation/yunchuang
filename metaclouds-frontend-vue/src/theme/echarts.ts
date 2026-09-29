/**
 * ============================================================
 * Metaclouds Design System (MDS) v2.1 — ECharts 主题
 * ============================================================
 *
 * 注册名为 `mc-light` 的 ECharts 主题：
 *   - 取色 `color` 来自 chartPalette（与 MDS 同源，品牌蓝 + 语义 + 中性）；
 *   - 文字 / 轴线 / 分割线 / 图例统一使用 MDS 中性色阶；
 *   - 背景透明，卡片/页面底色由容器提供。
 *
 * 使用方式（后续 Dashboard 接入，本任务不主动挂载）：
 *   ```ts
 *   import { registerMcLightTheme } from '@/theme/echarts';
 *   registerMcLightTheme();
 *   const chart = echarts.init(el, 'mc-light');
 *   ```
 *
 * 【FORCE-SYNC】颜色取值须与 tokens.ts / index.css `:root` 保持同步。
 * 不新增 npm 依赖（echarts 已在 dependencies）。
 * ============================================================
 */
import * as echarts from 'echarts';
import { chartPalette, colorTokens, layoutTokens } from './tokens';

/** ECharts 系列取色板（导出供业务 option 直接引用，保证与主题一致） */
export const palette = chartPalette;

const { neutral, semantic } = colorTokens;

/** mc-light 主题配置对象（供注册，也可被业务 option 展开参考） */
export const mcLightTheme = {
  // 系列主色循环
  color: chartPalette,
  backgroundColor: 'transparent',

  textStyle: {
    color: neutral.text2,
    fontFamily:
      "'Inter', 'HarmonyOS Sans SC', 'PingFang SC', 'Microsoft YaHei', sans-serif",
  },

  title: {
    left: 'auto',
    top: 'auto',
    textStyle: {
      color: neutral.text1,
      fontSize: 15,
      fontWeight: 650,
    },
    subtextStyle: {
      color: neutral.text3,
      fontSize: 12,
    },
  },

  legend: {
    icon: 'roundRect',
    itemWidth: 12,
    itemHeight: 8,
    itemGap: 16,
    textStyle: {
      color: neutral.text2,
    },
  },

  tooltip: {
    backgroundColor: neutral.surface,
    borderColor: neutral.line,
    borderWidth: 1,
    padding: [10, 12],
    textStyle: {
      color: neutral.text1,
      fontSize: 12,
    },
    extraCssText: `box-shadow: ${layoutTokens.shadow[3]}; border-radius: ${layoutTokens.radius.md};`,
  },

  // 类目轴（X）：有轴线、无分割线
  categoryAxis: {
    axisLine: {
      show: true,
      lineStyle: { color: neutral.line },
    },
    axisTick: {
      show: true,
      lineStyle: { color: neutral.line },
    },
    axisLabel: {
      color: neutral.text3,
      fontSize: 11,
    },
    splitLine: { show: false },
    splitArea: { show: false },
  },

  // 数值轴（Y）：无轴线、浅分割线
  valueAxis: {
    axisLine: { show: false },
    axisTick: { show: false },
    axisLabel: {
      color: neutral.text3,
      fontSize: 11,
    },
    splitLine: {
      show: true,
      lineStyle: { color: '#eef2f8', width: 1 },
    },
    splitArea: { show: false },
  },

  line: {
    smooth: false,
    symbol: 'circle',
    symbolSize: 6,
    lineStyle: { width: 2 },
    itemStyle: { borderColor: neutral.surface, borderWidth: 2 },
  },

  bar: {
    itemStyle: {
      borderRadius: [4, 4, 0, 0],
    },
  },

  pie: {
    itemStyle: {
      borderColor: neutral.surface,
      borderWidth: 2,
    },
    label: {
      color: neutral.text2,
    },
  },

  // 标记线 / 标记点状态色与 MDS 语义对齐
  markPoint: {
    label: { color: neutral.surface },
  },
  markLine: {
    symbol: 'none',
    lineStyle: { color: semantic.danger, type: 'dashed' },
    label: { color: semantic.dangerFg },
  },
};

/**
 * 注册 `mc-light` 主题。可重复调用（ECharts 内部按名覆盖，幂等）。
 * 必须在 `echarts.init(el, 'mc-light')` 之前调用。
 */
export function registerMcLightTheme(): void {
  echarts.registerTheme('mc-light', mcLightTheme as object);
}

export default registerMcLightTheme;
