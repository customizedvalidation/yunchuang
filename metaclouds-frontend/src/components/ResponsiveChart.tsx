import React, { useEffect, useRef } from 'react';
import ReactECharts from 'echarts-for-react';
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
  const chartRef = useRef<ReactECharts>(null);
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
      <ReactECharts ref={chartRef} option={option} notMerge style={{ height: '100%', width: '100%' }} />
    </div>
  );
};

export default ResponsiveChart;
