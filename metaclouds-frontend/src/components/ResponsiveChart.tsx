import React, { useEffect, useRef } from 'react';
// 使用 echarts-for-react 的 core 入口：不默认导入全量 echarts，
// 必须通过 echarts prop 传入按需注册的实例，从而实现真正的按需打包。
import ReactEChartsCore from 'echarts-for-react/lib/core';
// 按需引入的 echarts 实例（仅注册饼图/折线图 + 必要组件），
// 替代默认全量引入，echarts chunk 体积由 ~1MB 降至 ~300KB。
import echarts from '../utils/echarts';
import './ResponsiveChart.css';

interface ResponsiveChartProps {
  option: any;
  /** 高度令牌档位：sm=200 / md=200→260 / lg=200→260→320 */
  size?: 'sm' | 'md' | 'lg';
  className?: string;
}

/**
 * 响应式 ECharts 容器：
 * - 高度走 --mc-chart-h-* 设计令牌（随断点变化），不再硬编码 260px
 * - 用原生 ResizeObserver 监听容器尺寸变化（侧边栏折叠、窗口缩放），
 *   主动调用 echartsInstance.resize()，避免图表被裁切或留白
 */
const ResponsiveChart: React.FC<ResponsiveChartProps> = ({ option, size = 'md', className }) => {
  const chartRef = useRef<ReactEChartsCore>(null);
  const wrapRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = wrapRef.current;
    if (!el || typeof ResizeObserver === 'undefined') return;
    const ro = new ResizeObserver(() => {
      chartRef.current?.getEchartsInstance().resize();
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  return (
    <div
      ref={wrapRef}
      className={`mc-chart mc-chart-${size}${className ? ` ${className}` : ''}`}
    >
      <ReactEChartsCore
        ref={chartRef}
        echarts={echarts}
        option={option}
        notMerge
        style={{ height: '100%', width: '100%' }}
      />
    </div>
  );
};

export default ResponsiveChart;
