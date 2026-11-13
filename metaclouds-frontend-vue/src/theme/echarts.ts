/**
 * ============================================================
 * Metaclouds Design System (MDS) v3.1 — ECharts 主题
 * ============================================================
 *
 * 注册两套 ECharts 主题：`mc-light` / `mc-dark`，取色分别来自
 * tokens.ts 的 colorTokens 与 darkColorTokens（与 index.css 两套令牌同源）。
 * 背景透明，卡片/页面底色由容器提供。
 *
 * 使用方式：
 *   ```ts
 *   import { registerMcThemes, chartThemeName, onChartThemeChange, echarts } from '@/theme/echarts';
 *   registerMcThemes();
 *   const chart = echarts.init(el, chartThemeName.value);
 *   // 主题切换时 ECharts 主题不会热更新，必须重建实例：
 *   const stop = onChartThemeChange(() => { chart.dispose(); chart = echarts.init(el, chartThemeName.value); ... });
 *   onBeforeUnmount(stop);
 *   ```
 *
 * 【FORCE-SYNC】颜色取值须与 tokens.ts / index.css 两套令牌保持同步。
 * 不新增 npm 依赖（echarts 已在 dependencies）。
 *
 * 【按需引入】本模块是全项目唯一的 echarts 入口：
 *   - 从 `echarts/core` 引入并集中 `use()` 注册本项目用到的图表与组件；
 *   - 全量 `import 'echarts'` 约 1 MB（gzip 343 KB），按需后仅打包
 *     饼图/折线/柱状 + 基础组件，业务页请 `import { echarts } from '@/theme/echarts'`，
 *     不要直接从 'echarts' 或 'echarts/core' 引入（会绕过注册或重复打包）。
 * ============================================================
 */
import { computed, watch } from 'vue'
import * as echarts from 'echarts/core'
import { PieChart, LineChart, BarChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import {
  chartPalette,
  colorTokens,
  darkColorTokens,
  darkShadowTokens,
  layoutTokens,
  type ColorTokens,
} from './tokens';
import { activeTheme } from './useTheme';

// 按需注册当前全部页面用到的图表类型 / 组件 / 渲染器（幂等，可安全多次调用）。
echarts.use([
  PieChart,
  LineChart,
  BarChart,
  GridComponent,
  TooltipComponent,
  LegendComponent,
  CanvasRenderer,
])

export { echarts }

/** ECharts 系列取色板（浅色，导出供业务 option 直接引用） */
export const palette = chartPalette;

/** 当前应使用的 ECharts 主题名，随 activeTheme 响应式变化 */
export const chartThemeName = computed(() =>
  activeTheme.value === 'dark' ? 'mc-dark' : 'mc-light',
);

/**
 * 订阅主题切换。返回取消订阅函数，组件须在 onBeforeUnmount 中调用。
 * 页面收到回调后应 dispose 旧实例并用 chartThemeName.value 重建，
 * 因为 ECharts 在 init 时就把主题色固化进实例，setOption 无法热更新。
 */
export function onChartThemeChange(cb: () => void): () => void {
  return watch(activeTheme, () => cb());
}

/**
 * 当前主题的颜色令牌（响应式）。业务 option 若在 computed 中读取
 * `activeColorTokens.value.*`，主题切换时 option 会自动重算。
 * 系列语义色两主题共用同一套高饱和色；只有中性色（文字/轴线/表面/边框）
 * 随主题切换，避免暗底下出现浅色文字或白色扇区边框。
 */
export const activeColorTokens = computed(() =>
  activeTheme.value === 'dark' ? darkColorTokens : colorTokens,
);

/** 当前主题的系列取色板（响应式） */
export const activePalette = computed(() =>
  activeTheme.value === 'dark' ? darkPalette : chartPalette,
);

/** 由一套颜色令牌构造 ECharts 主题对象，保证 light / dark 结构完全一致 */
function buildTheme(c: ColorTokens, paletteColors: string[], shadow3: string) {
  const { neutral, semantic } = c;
  return {
    color: paletteColors,
    backgroundColor: 'transparent',

    textStyle: {
      color: neutral.text2,
      fontFamily: layoutTokens.font.sans,
    },

    title: {
      left: 'auto',
      top: 'auto',
      textStyle: {
        color: neutral.text1,
        fontSize: 15,
        fontWeight: 600,
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
      textStyle: { color: neutral.text2 },
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
      extraCssText: `box-shadow: ${shadow3}; border-radius: ${layoutTokens.radius.md};`,
    },

    categoryAxis: {
      axisLine: { show: true, lineStyle: { color: neutral.line } },
      axisTick: { show: true, lineStyle: { color: neutral.line } },
      axisLabel: { color: neutral.text3, fontSize: 11 },
      splitLine: { show: false },
      splitArea: { show: false },
    },

    valueAxis: {
      axisLine: { show: false },
      axisTick: { show: false },
      axisLabel: { color: neutral.text3, fontSize: 11 },
      splitLine: { show: true, lineStyle: { color: neutral.lineSubtle, width: 1 } },
      splitArea: { show: false },
    },

    line: {
      smooth: false,
      symbol: 'circle',
      symbolSize: 6,
      lineStyle: { width: 2 },
      itemStyle: { borderColor: neutral.surface, borderWidth: 2 },
    },

    bar: { itemStyle: { borderRadius: [4, 4, 0, 0] } },

    pie: {
      itemStyle: { borderColor: neutral.surface, borderWidth: 2 },
      label: { color: neutral.text2 },
    },

    markPoint: { label: { color: neutral.surface } },
    markLine: {
      symbol: 'none',
      lineStyle: { color: semantic.danger, type: 'dashed' },
      label: { color: semantic.dangerFg },
    },
  };
}

/** 暗色系列取色板：与 chartPalette 同一顺序，换成暗色令牌 */
export const darkPalette: string[] = [
  darkColorTokens.brand.base,
  darkColorTokens.semantic.teal,
  darkColorTokens.semantic.success,
  darkColorTokens.semantic.warning,
  darkColorTokens.semantic.danger,
  darkColorTokens.brand['400'],
  darkColorTokens.semantic.info,
  darkColorTokens.brand['600'],
  darkColorTokens.chartAccent,
  darkColorTokens.neutral.text3,
];

/** mc-light 主题配置对象（供注册，也可被业务 option 展开参考） */
export const mcLightTheme = buildTheme(colorTokens, chartPalette, layoutTokens.shadow[3]);

/** mc-dark 主题配置对象 */
export const mcDarkTheme = buildTheme(darkColorTokens, darkPalette, darkShadowTokens[3]);

/** 安全注册（测试环境若 mock 掉 registerTheme 则静默降级） */
function safeRegister(name: string, theme: object): void {
  try {
    echarts.registerTheme(name, theme)
  } catch {
    /* 测试环境 echarts 被 mock 时无 registerTheme，静默降级 */
  }
}

/**
 * 注册 `mc-light` 主题（v3.0 兼容入口，幂等）。
 * 新代码建议直接用 registerMcThemes()。
 */
export function registerMcLightTheme(): void {
  safeRegister('mc-light', mcLightTheme)
}

/** 注册全部主题（幂等）。必须在 echarts.init(el, chartThemeName.value) 之前调用。 */
export function registerMcThemes(): void {
  safeRegister('mc-light', mcLightTheme)
  safeRegister('mc-dark', mcDarkTheme)
}

export default registerMcThemes;
