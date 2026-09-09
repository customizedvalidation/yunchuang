/**
 * ECharts 按需引入模块
 *
 * 优化背景：
 *   默认 `import * as echarts from 'echarts'` 会打包全部图表类型（bar/line/pie/scatter/map/...）
 *   及全部组件，导致 echarts chunk 高达 ~1MB（gzip ~347KB）。
 *   本项目仅使用饼图（Dashboard）和折线图（MonitoringAlert），按需注册后体积可大幅下降。
 *
 * 使用方式：
 *   import echarts from './echarts';
 *   <ReactECharts echarts={echarts} option={...} />
 *
 * 新增图表类型时，请在此处注册对应模块并更新注释。
 */
import * as echarts from 'echarts/core';

// 图表类型：仅注册项目实际使用的
import { PieChart, LineChart, BarChart } from 'echarts/charts';

// 组件：Tooltip（提示框）、Legend（图例）、Title（标题）、Grid（直角坐标系网格）
import {
  TooltipComponent,
  LegendComponent,
  TitleComponent,
  GridComponent,
} from 'echarts/components';

// 渲染器：Canvas（项目全部图表均使用 Canvas 渲染）
import { CanvasRenderer } from 'echarts/renderers';

// 注册模块
echarts.use([
  PieChart,
  LineChart,
  BarChart,
  TooltipComponent,
  LegendComponent,
  TitleComponent,
  GridComponent,
  CanvasRenderer,
]);

export default echarts;
