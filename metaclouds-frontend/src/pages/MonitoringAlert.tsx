import React from 'react';
import { Card, Table, Tag } from 'antd';
import { useGetAlertsQuery } from '../store/api';
import { extractArrayData } from '../utils/api';
import ReactECharts from 'echarts-for-react';

const MonitoringAlert: React.FC = () => {
  const { data: alerts, isLoading } = useGetAlertsQuery(undefined);
  const alertsData = extractArrayData(alerts);

  const getLevelColor = (level: string) => {
    switch (level) {
      case 'critical': return 'red';
      case 'error': return 'orange';
      case 'warning': return 'yellow';
      case 'info': return 'blue';
      default: return 'default';
    }
  };

  const chartOption = {
    title: { text: '资源使用趋势', left: 'center' },
    tooltip: { trigger: 'axis' },
    xAxis: { type: 'category', data: ['00:00', '04:00', '08:00', '12:00', '16:00', '20:00', '24:00'] },
    yAxis: { type: 'value' },
    series: [
      { name: 'CPU', type: 'line', data: [30, 45, 60, 55, 70, 65, 50] },
      { name: '内存', type: 'line', data: [40, 50, 55, 60, 65, 70, 60] },
      { name: 'GPU', type: 'line', data: [20, 30, 50, 70, 80, 75, 60] },
    ],
  };

  const columns = [
    { title: 'ID', dataIndex: 'id', key: 'id' },
    { title: '类型', dataIndex: 'type', key: 'type' },
    { title: '级别', dataIndex: 'level', key: 'level', render: (level: string) => <Tag color={getLevelColor(level)}>{level}</Tag> },
    { title: '消息', dataIndex: 'message', key: 'message' },
    { title: '状态', dataIndex: 'status', key: 'status', render: (status: string) => <Tag>{status}</Tag> },
    { title: '时间', dataIndex: 'created_at', key: 'created_at' },
  ];

  return (
    <div className="p-6">
      <Card title="监控告警">
        <ReactECharts option={chartOption} style={{ height: 300 }} />
      </Card>
      <Card title="告警列表" className="mt-4">
        <Table
          columns={columns}
          dataSource={alertsData}
          loading={isLoading}
          rowKey="id"
        />
      </Card>
    </div>
  );
};

export default MonitoringAlert;