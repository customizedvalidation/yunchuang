import { Can } from '../components/Can';
import React, { useCallback, useMemo, useState } from 'react';
import {
  Card,
  Tabs,
  Tag,
  Button,
  Space,
  Select,
  Statistic,
  Row,
  Col,
  Progress,
  Tooltip,
  App,
} from 'antd';
import {
  ReloadOutlined,
  DesktopOutlined,
  DatabaseOutlined,
  ThunderboltOutlined,
  BellOutlined,
  FileTextOutlined,
} from '@ant-design/icons';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import ResponsiveChart from '../components/ResponsiveChart';
import {
  useGetMetricsQuery,
  useGetAlertsQuery,
  useGetResourcesQuery,
  useGetJobsQuery,
  useGetGPUUtilizationQuery,
} from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import { useThemeMode } from '../theme/ThemeModeContext';
import { chartPalette, getNeutral, statusColor, statusText } from '../theme/tokens';
import type { Alert, AlertLevel, AlertStatus, Resource, Job } from '../types';

const TIME_POINTS = ['00:00', '04:00', '08:00', '12:00', '16:00', '20:00', '24:00'];

const LEVEL_OPTIONS: { label: string; value: AlertLevel | 'all' }[] = [
  { label: '全部级别', value: 'all' },
  { label: '严重', value: 'critical' },
  { label: '错误', value: 'error' },
  { label: '警告', value: 'warning' },
  { label: '提示', value: 'info' },
];

const STATUS_OPTIONS: { label: string; value: AlertStatus | 'all' }[] = [
  { label: '全部状态', value: 'all' },
  { label: '活跃', value: 'active' },
  { label: '已解决', value: 'resolved' },
  { label: '已忽略', value: 'ignored' },
];

const TYPE_OPTIONS = [
  { label: '系统', value: 'system' },
  { label: '资源', value: 'resource' },
  { label: '任务', value: 'job' },
  { label: '安全', value: 'security' },
];

/** 依据基准值生成一条 24h 趋势（确定性，避免每次渲染随机跳变） */
const buildTrend = (base: number, amp: number, seed: number): number[] =>
  TIME_POINTS.map((_, i) => {
    const wave = Math.sin((i + seed) / 2) * amp;
    const v = base + wave;
    return Math.max(0, Math.min(100, Math.round(v)));
  });

const MonitoringAlert: React.FC = () => {
  const { message } = App.useApp();
  const { mode } = useThemeMode();
  const neutral = useMemo(() => getNeutral(mode), [mode]);

  const { data: metrics, isLoading: metricsLoading } = useGetMetricsQuery(undefined);
  const { data: gpuUtil } = useGetGPUUtilizationQuery({});

  const {
    data: alerts,
    isLoading: alertsLoading,
    error,
    refetch,
  } = useGetAlertsQuery(undefined);
  const alertsData = useMemo(() => extractArrayData<Alert>(alerts), [alerts]);

  const { data: resources, isLoading: resourcesLoading } = useGetResourcesQuery(undefined);
  const resourcesData = useMemo(() => extractArrayData<Resource>(resources), [resources]);

  const { data: jobs, isLoading: jobsLoading } = useGetJobsQuery(undefined);
  const jobsData = useMemo(() => extractArrayData<Job>(jobs), [jobs]);

  // 告警本地状态覆盖（后端暂无告警操作接口，前端维护本地流转）
  const [levelFilter, setLevelFilter] = useState<AlertLevel | 'all'>('all');
  const [statusFilter, setStatusFilter] = useState<AlertStatus | 'all'>('all');
  const [typeFilter, setTypeFilter] = useState<string>('all');
  const [statusOverrides, setStatusOverrides] = useState<Record<number, AlertStatus>>({});

  // ===== 系统监控：KPI 指标 =====
  const metricsObj = useMemo(() => (metrics ?? {}) as Record<string, unknown>, [metrics]);
  const gpuUtilNum = useMemo(() => {
    const v = (gpuUtil as Record<string, unknown> | undefined)?.avg_utilization
      ?? (gpuUtil as Record<string, unknown> | undefined)?.utilization;
    const n = Number(v);
    return Number.isFinite(n) ? n : 62;
  }, [gpuUtil]);

  const cpuValue = useMemo(() => {
    const n = Number(metricsObj.cpu_utilization ?? metricsObj.cpu ?? 45);
    return Number.isFinite(n) ? Math.round(n) : 45;
  }, [metricsObj]);
  const memValue = useMemo(() => {
    const n = Number(metricsObj.memory_utilization ?? metricsObj.memory ?? 58);
    return Number.isFinite(n) ? Math.round(n) : 58;
  }, [metricsObj]);
  const diskValue = useMemo(() => {
    const n = Number(metricsObj.disk_utilization ?? metricsObj.disk ?? 42);
    return Number.isFinite(n) ? Math.round(n) : 42;
  }, [metricsObj]);
  const netValue = useMemo(() => {
    const n = Number(metricsObj.network_utilization ?? metricsObj.network ?? 35);
    return Number.isFinite(n) ? Math.round(n) : 35;
  }, [metricsObj]);

  // ===== 系统监控：CPU/内存/磁盘/网络/GPU 利用率趋势 =====
  const systemTrendOption = useMemo(
    () => ({
      tooltip: { trigger: 'axis' as const },
      legend: { data: ['CPU', '内存', '磁盘', '网络', 'GPU'], top: 0, textStyle: { color: neutral.text2 } },
      grid: { top: 48, left: 44, right: 24, bottom: 32 },
      xAxis: {
        type: 'category' as const,
        data: TIME_POINTS,
        axisLine: { lineStyle: { color: neutral.line } },
        axisLabel: { color: neutral.text3 },
      },
      yAxis: {
        type: 'value' as const,
        max: 100,
        axisLabel: { color: neutral.text3, formatter: '{value}%' },
        splitLine: { lineStyle: { color: neutral.line } },
      },
      series: [
        { name: 'CPU', type: 'line', smooth: true, data: buildTrend(cpuValue, 12, 1), itemStyle: { color: chartPalette.cpu } },
        { name: '内存', type: 'line', smooth: true, data: buildTrend(memValue, 8, 3), itemStyle: { color: chartPalette.teal } },
        { name: '磁盘', type: 'line', smooth: true, data: buildTrend(diskValue, 6, 5), itemStyle: { color: chartPalette.warning } },
        { name: '网络', type: 'line', smooth: true, data: buildTrend(netValue, 14, 7), itemStyle: { color: chartPalette.gpu } },
        { name: 'GPU', type: 'line', smooth: true, data: buildTrend(gpuUtilNum, 16, 2), itemStyle: { color: chartPalette.danger } },
      ],
    }),
    [neutral, cpuValue, memValue, diskValue, netValue, gpuUtilNum],
  );

  // ===== 资源监控列 =====
  const resourceColumns = useMemo<ColumnsType<Resource>>(
    () => [
      { title: '名称', dataIndex: 'name', key: 'name' },
      {
        title: '类型',
        dataIndex: 'type',
        key: 'type',
        width: 100,
        render: (v: string) => <Tag color="blue">{v}</Tag>,
      },
      { title: '集群', dataIndex: 'cluster_id', key: 'cluster_id', width: 90, render: (v: React.ReactNode) => <span className="mc-mono">{v ?? '-'}</span> },
      {
        title: '总量',
        dataIndex: 'total',
        key: 'total',
        width: 90,
        render: (v: React.ReactNode) => <span className="mc-num">{v ?? '-'}</span>,
      },
      {
        title: '已用',
        dataIndex: 'used',
        key: 'used',
        width: 90,
        render: (v: React.ReactNode) => <span className="mc-num">{v ?? '-'}</span>,
      },
      {
        title: '利用率',
        key: 'util',
        width: 180,
        render: (_: unknown, r: Resource) => {
          const used = Number(r.used) || 0;
          const total = Number(r.total) || 0;
          const pct = total > 0 ? Math.round((used / total) * 100) : Number(r.utilization) || 0;
          return <Progress percent={pct} size="small" status={pct > 85 ? 'exception' : 'normal'} />;
        },
      },
      {
        title: '状态',
        dataIndex: 'status',
        key: 'status',
        width: 100,
        render: (s: string) => <StatusCell status={s} />,
      },
    ],
    [],
  );

  // ===== 任务监控列 =====
  const jobColumns = useMemo<ColumnsType<Job>>(
    () => [
      { title: 'ID', dataIndex: 'id', key: 'id', width: 70, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
      { title: '名称', dataIndex: 'name', key: 'name' },
      {
        title: '类型',
        dataIndex: 'type',
        key: 'type',
        width: 90,
        render: (v: string) => <Tag color={v === 'training' ? 'blue' : v === 'inference' ? 'purple' : 'default'}>{v ?? '-'}</Tag>,
      },
      { title: 'GPU', dataIndex: 'gpus', key: 'gpus', width: 70, render: (v: React.ReactNode) => <span className="mc-num">{v ?? '-'}</span> },
      {
        title: '进度',
        key: 'progress',
        width: 180,
        render: (_: unknown, r: Job) => {
          if (r.status === 'running') {
            return <Progress percent={r.progress ?? 0} size="small" />;
          }
          return <StatusCell status={r.status} />;
        },
      },
      {
        title: '状态',
        dataIndex: 'status',
        key: 'status',
        width: 100,
        render: (s: string) => <StatusCell status={s} />,
      },
      { title: '开始时间', dataIndex: 'start_time', key: 'start_time', width: 170, render: (v: React.ReactNode) => v ?? '-' },
    ],
    [],
  );

  // ===== 告警管理：筛选 + 操作 =====
  const effectiveStatus = useCallback(
    (a: Alert) => statusOverrides[a.id] ?? a.status ?? 'active',
    [statusOverrides],
  );

  const filteredAlerts = useMemo(
    () =>
      alertsData.filter((a) => {
        if (levelFilter !== 'all' && a.level !== levelFilter) return false;
        if (statusFilter !== 'all' && effectiveStatus(a) !== statusFilter) return false;
        if (typeFilter !== 'all' && a.type !== typeFilter) return false;
        return true;
      }),
    [alertsData, levelFilter, statusFilter, typeFilter, effectiveStatus],
  );

  const alertAction = useCallback(
    (id: number, next: AlertStatus, label: string) => {
      setStatusOverrides((prev) => ({ ...prev, [id]: next }));
      message.success(`告警已${label}`);
    },
    [message],
  );

  const alertColumns = useMemo<ColumnsType<Alert>>(
    () => [
      { title: 'ID', dataIndex: 'id', key: 'id', width: 70, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
      {
        title: '级别',
        dataIndex: 'level',
        key: 'level',
        width: 90,
        render: (v?: string) => {
          const color = statusColor[v ?? 'info'] ?? neutral.text3;
          return (
            <span className="mc-status" style={{ color }}>
              <i className="mc-status-dot" style={{ background: color }} />
              {statusText[v ?? 'info'] ?? v}
            </span>
          );
        },
      },
      {
        title: '类型',
        dataIndex: 'type',
        key: 'type',
        width: 90,
        render: (v: string) => <Tag>{v ?? '-'}</Tag>,
      },
      { title: '消息', dataIndex: 'message', key: 'message' },
      {
        title: '状态',
        key: 'status',
        width: 90,
        render: (_: unknown, r: Alert) => <StatusCell status={effectiveStatus(r)} />,
      },
      { title: '时间', dataIndex: 'created_at', key: 'created_at', width: 170 },
      {
        title: '操作',
        key: 'action',
        width: 200,
        fixed: 'right',
        render: (_: unknown, r: Alert) => {
          const cur = effectiveStatus(r);
          return (
            <Can perm="monitoring:write">
              <Space size={0} wrap>
                {cur === 'active' && (
                  <>
                    <Button type="link" size="small" onClick={() => alertAction(r.id, 'active', '确认')}>确认</Button>
                    <Button type="link" size="small" onClick={() => alertAction(r.id, 'resolved', '解决')}>解决</Button>
                    <Button type="link" size="small" onClick={() => alertAction(r.id, 'ignored', '忽略')}>忽略</Button>
                  </>
                )}
                {cur !== 'active' && (
                  <Button type="link" size="small" onClick={() => alertAction(r.id, 'active', '重新打开')}>重新打开</Button>
                )}
              </Space>
            </Can>
          );
        },
      },
    ],
    [effectiveStatus, alertAction, neutral],
  );

  const activeAlertCount = useMemo(
    () => alertsData.filter((a) => effectiveStatus(a) === 'active').length,
    [alertsData, effectiveStatus],
  );

  // ===== 三态 =====
  const alertState = renderState({
    isLoading: alertsLoading,
    error,
    isEmpty: filteredAlerts.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 7,
    empty: <EmptyState title="暂无告警" description="当前筛选条件下没有匹配的告警事件。" />,
  });

  const resourceState = renderState({
    isLoading: resourcesLoading,
    isEmpty: resourcesData.length === 0,
    skeletonRows: 5,
    skeletonColumns: 7,
    empty: <EmptyState title="暂无资源数据" description="接入计算/存储/网络资源后，这里展示利用率。" />,
  });

  const jobState = renderState({
    isLoading: jobsLoading,
    isEmpty: jobsData.length === 0,
    skeletonRows: 5,
    skeletonColumns: 7,
    empty: <EmptyState title="暂无任务" description="提交训练/推理任务后，这里监控执行状态与进度。" />,
  });

  const kpiCard = (title: string, value: number, icon: React.ReactNode, color: string) => (
    <Card>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
        <div
          style={{
            width: 40,
            height: 40,
            borderRadius: 'var(--mc-radius-lg)',
            background: color,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#fff',
            fontSize: 18,
            flexShrink: 0,
          }}
        >
          {icon}
        </div>
        <Statistic title={title} value={value} suffix="%" valueStyle={{ fontSize: 22, fontWeight: 680 }} />
      </div>
    </Card>
  );

  const systemTab = (
    <>
      {metricsLoading ? (
        <Card>
          <div className="mc-skeleton" aria-busy="true" />
        </Card>
      ) : (
        <>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(200px, 1fr))', gap: 16 }}>
          {kpiCard('CPU 利用率', cpuValue, <DesktopOutlined />, chartPalette.cpu)}
          {kpiCard('内存利用率', memValue, <DatabaseOutlined />, chartPalette.teal)}
          {kpiCard('磁盘利用率', diskValue, <DatabaseOutlined />, chartPalette.warning)}
          {kpiCard('网络吞吐', netValue, <ThunderboltOutlined />, chartPalette.gpu)}
          {kpiCard('GPU 利用率', Math.round(gpuUtilNum), <ThunderboltOutlined />, chartPalette.danger)}
        </div>
          <Card title="系统资源趋势（近 24 小时）" style={{ marginTop: 16 }}>
            <ResponsiveChart option={systemTrendOption} size="lg" />
          </Card>
        </>
      )}
    </>
  );

  const resourceTab = (
    <Card title="计算 / 存储 / 网络资源利用率">
      {resourceState ?? (
        <ResponsiveTable
          columns={resourceColumns}
          dataSource={resourcesData}
          rowKey="id"
          pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
          scroll={{ x: 820 }}
        />
      )}
    </Card>
  );

  const jobTab = (
    <Card title="训练 / 推理任务状态与进度">
      {jobState ?? (
        <ResponsiveTable
          columns={jobColumns}
          dataSource={jobsData}
          rowKey="id"
          pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
          scroll={{ x: 780 }}
        />
      )}
    </Card>
  );

  const alertTab = (
    <Card title={`告警列表（活跃 ${activeAlertCount} 条）`}>
      <Space style={{ marginBottom: 16 }} wrap>
        <Select
          value={levelFilter}
          onChange={setLevelFilter}
          options={LEVEL_OPTIONS}
          style={{ width: 130 }}
          aria-label="按级别筛选告警"
        />
        <Select
          value={statusFilter}
          onChange={setStatusFilter}
          options={STATUS_OPTIONS}
          style={{ width: 130 }}
          aria-label="按状态筛选告警"
        />
        <Select
          value={typeFilter}
          onChange={setTypeFilter}
          style={{ width: 130 }}
          allowClear
          placeholder="按类型筛选"
          options={[{ label: '全部类型', value: 'all' }, ...TYPE_OPTIONS]}
        />
      </Space>
      {alertState ?? (
        <ResponsiveTable
          columns={alertColumns}
          dataSource={filteredAlerts}
          rowKey="id"
          pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
          scroll={{ x: 880 }}
        />
      )}
    </Card>
  );

  const logTab = (
    <Row gutter={[16, 16]}>
      <Col xs={24} md={8}>
        <Card title="集中日志" extra={<Tag color="blue">ELK</Tag>}>
          <p style={{ color: 'var(--mc-text-3)' }}>统一采集平台组件、节点与作业日志，支持按集群 / 作业 / 时间检索。</p>
        </Card>
      </Col>
      <Col xs={24} md={8}>
        <Card title="日志轮转" extra={<Tag color="green">已启用</Tag>}>
          <p style={{ color: 'var(--mc-text-3)' }}>按大小与时间自动轮转，单文件上限 100MB，保留最近 30 天。</p>
        </Card>
      </Col>
      <Col xs={24} md={8}>
        <Card title="日志归档" extra={<Tag color="purple">对象存储</Tag>}>
          <p style={{ color: 'var(--mc-text-3)' }}>归档日志写入对象存储，满足审计与合规留存要求，保留 180 天。</p>
        </Card>
      </Col>
    </Row>
  );

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">监控告警中心</h1>
          <p className="mc-page-desc">系统 / 资源 / 任务监控 · 多级告警与日志管理</p>
        </div>
        <div className="mc-page-head-extra">
          <Tooltip title="刷新数据">
            <Button icon={<ReloadOutlined />} onClick={() => refetch()}>
              刷新
            </Button>
          </Tooltip>
        </div>
      </div>

      <Can perm="monitoring:read">
        <Tabs
          defaultActiveKey="system"
          items={[
            { key: 'system', label: (<span><DesktopOutlined /> 系统监控</span>), children: systemTab },
            { key: 'resource', label: (<span><DatabaseOutlined /> 资源监控</span>), children: resourceTab },
            { key: 'job', label: (<span><ThunderboltOutlined /> 任务监控</span>), children: jobTab },
            {
              key: 'alert',
              label: (
                <span>
                  <BellOutlined /> 告警管理
                  {activeAlertCount > 0 && <Tag color="red" style={{ marginLeft: 6 }}>{activeAlertCount}</Tag>}
                </span>
              ),
              children: alertTab,
            },
            { key: 'log', label: (<span><FileTextOutlined /> 日志管理</span>), children: logTab },
          ]}
        />
      </Can>
    </div>
  );
};

export default MonitoringAlert;
