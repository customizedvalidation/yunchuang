import React from 'react';
import { Card, Tag, Button } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import { useGetAlertsQuery } from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import { useThemeMode } from '../theme/ThemeModeContext';
import { getNeutral, chartPalette } from '../theme/tokens';
import ResponsiveChart from '../components/ResponsiveChart';

const MonitoringAlert: React.FC = () => {
  const { mode } = useThemeMode();
  const neutral = getNeutral(mode);
  const { data: alerts, isLoading, error, refetch } = useGetAlertsQuery(undefined);
  const alertsData = extractArrayData(alerts);

  const chartOption = {
    title: { text: '资源使用趋势', left: 'center', textStyle: { color: neutral.text1, fontSize: 14, fontWeight: 600 } },
    tooltip: { trigger: 'axis' },
    legend: { data: ['CPU', '内存', 'GPU'], top: 30, textStyle: { color: neutral.text2 } },
    grid: { top: 70, left: 44, right: 24, bottom: 32 },
    xAxis: {
      type: 'category',
      data: ['00:00', '04:00', '08:00', '12:00', '16:00', '20:00', '24:00'],
      axisLine: { lineStyle: { color: neutral.line } },
      axisLabel: { color: neutral.text3 },
    },
    yAxis: {
      type: 'value',
      axisLine: { lineStyle: { color: neutral.line } },
      splitLine: { lineStyle: { color: neutral.line } },
      axisLabel: { color: neutral.text3 },
    },
    series: [
      { name: 'CPU', type: 'line', smooth: true, data: [30, 45, 60, 55, 70, 65, 50], itemStyle: { color: chartPalette.cpu } },
      { name: '内存', type: 'line', smooth: true, data: [40, 50, 55, 60, 65, 70, 60], itemStyle: { color: chartPalette.teal } },
      { name: 'GPU', type: 'line', smooth: true, data: [20, 30, 50, 70, 80, 75, 60], itemStyle: { color: chartPalette.gpu } },
    ],
  };

  const columns = [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 80, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag>{type}</Tag> },
    { title: '级别', dataIndex: 'level', key: 'level', width: 100, render: (level: string) => <StatusCell status={level} /> },
    { title: '消息', dataIndex: 'message', key: 'message' },
    { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (status: string) => <StatusCell status={status} /> },
    { title: '时间', dataIndex: 'created_at', key: 'created_at', width: 180 },
  ];

  const state = renderState({
    isLoading,
    error,
    isEmpty: alertsData.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 6,
    empty: <EmptyState title="暂无告警" description="系统运行平稳，没有需要处理的告警。" />,
  });

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">监控告警</h1>
          <p className="mc-page-desc">资源使用趋势与告警事件总览</p>
        </div>
        <div className="mc-page-head-extra">
          <Button onClick={() => refetch()}>刷新</Button>
        </div>
      </div>

      <div className="mc-monitor-grid">
        <Card>
          <ResponsiveChart option={chartOption} size="lg" />
        </Card>

        <Card title="告警列表">
          {state ?? (
            <ResponsiveTable
              columns={columns}
              dataSource={alertsData}
              rowKey="id"
              pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
              scroll={{ x: 820 }}
            />
          )}
        </Card>
      </div>
    </div>
  );
};

export default MonitoringAlert;
