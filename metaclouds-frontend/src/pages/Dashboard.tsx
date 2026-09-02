import React, { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Card,
  Statistic,
  Progress,
  List,
  Button,
  Space,
  Segmented,
  Skeleton,
  Tooltip,
} from 'antd';
import ResponsiveChart from '../components/ResponsiveChart';
import {
  DatabaseOutlined,
  CloudOutlined,
  ClockCircleOutlined,
  BellOutlined,
  ReloadOutlined,
} from '@ant-design/icons';
import {
  useGetClustersQuery,
  useGetResourcesQuery,
  useGetJobsQuery,
  useGetAlertsQuery,
} from '../store/api';
import { extractArrayData } from '../utils/api';
import { ErrorState, EmptyState } from '../components/States';
import DraggableGrid, { type GridItem } from '../components/DraggableGrid';
import { useThemeMode } from '../theme/ThemeModeContext';
import { chartPalette, getNeutral, statusColor, statusText } from '../theme/tokens';

const STORAGE_KEY = 'mc-dashboard-layout-v1';

const DEFAULT_ORDER = [
  'kpi-cluster',
  'kpi-util',
  'kpi-running',
  'kpi-alerts',
  'chart-resource',
  'chart-job',
  'alerts',
];

interface Layout {
  order: string[];
  hidden: string[];
}

const readLayout = (): Layout => {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { order: DEFAULT_ORDER, hidden: [] };
    const parsed = JSON.parse(raw) as Partial<Layout>;
    const savedOrder = Array.isArray(parsed.order) ? parsed.order : [];
    // 过滤已不存在的卡片，并补齐新增卡片，保证向前兼容
    const order = [
      ...savedOrder.filter((id) => DEFAULT_ORDER.includes(id)),
      ...DEFAULT_ORDER.filter((id) => !savedOrder.includes(id)),
    ];
    const hidden = Array.isArray(parsed.hidden)
      ? parsed.hidden.filter((id) => DEFAULT_ORDER.includes(id))
      : [];
    return { order, hidden };
  } catch {
    return { order: DEFAULT_ORDER, hidden: [] };
  }
};

const Dashboard: React.FC = () => {
  const { mode } = useThemeMode();
  const colors = useMemo(() => getNeutral(mode), [mode]);

  const { data: clusters, isLoading: clustersLoading, error: clustersError, refetch: refetchClusters } =
    useGetClustersQuery(undefined);
  const { data: resources, isLoading: resourcesLoading, error: resourcesError, refetch: refetchResources } =
    useGetResourcesQuery(undefined);
  const { data: jobs, isLoading: jobsLoading, error: jobsError, refetch: refetchJobs } =
    useGetJobsQuery(undefined);
  const { data: alerts, isLoading: alertsLoading, error: alertsError, refetch: refetchAlerts } =
    useGetAlertsQuery(undefined);

  const clustersData = extractArrayData(clusters);
  const resourcesData = extractArrayData(resources);
  const jobsData = extractArrayData(jobs);
  const alertsData = extractArrayData(alerts);

  const [layout, setLayout] = useState<Layout>(readLayout);
  const [editable, setEditable] = useState(false);
  const [range, setRange] = useState('近 7 天');

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(layout));
    } catch {
      /* 忽略写入失败 */
    }
  }, [layout]);

  // 真实指标：GPU 利用率取自资源 used/total，而非硬编码
  const gpuUsed = useMemo(
    () => resourcesData.reduce((s: number, r: { used?: number }) => s + (Number(r.used) || 0), 0),
    [resourcesData],
  );
  const gpuTotal = useMemo(
    () => resourcesData.reduce((s: number, r: { total?: number }) => s + (Number(r.total) || 0), 0),
    [resourcesData],
  );
  const utilization = gpuTotal > 0 ? Math.round((gpuUsed / gpuTotal) * 100) : 0;

  const clusterCount = clustersData.length;
  const runningJobs = jobsData.filter((j: { status: string }) => j.status === 'running').length;
  const pendingJobs = jobsData.filter((j: { status: string }) => j.status === 'pending').length;
  const totalJobs = jobsData.length;
  const alertCount = alertsData.length;

  const hasError = clustersError || resourcesError || jobsError || alertsError;
  const isLoading = clustersLoading || resourcesLoading || jobsLoading || alertsLoading;

  const refetchAll = useCallback(() => {
    refetchClusters();
    refetchResources();
    refetchJobs();
    refetchAlerts();
  }, [refetchClusters, refetchResources, refetchJobs, refetchAlerts]);

  const jobStatusData = useMemo(() => {
    const groups: Record<string, number> = {};
    jobsData.forEach((j: { status?: string }) => {
      const key = j.status ?? 'unknown';
      groups[key] = (groups[key] ?? 0) + 1;
    });
    return Object.entries(groups).map(([status, value]) => ({
      value,
      name: statusText[status] ?? status,
      itemStyle: { color: statusColor[status] ?? colors.text3 },
    }));
  }, [jobsData, colors.text3]);

  const resourceOption = useMemo(
    () => ({
      tooltip: { trigger: 'item' as const, formatter: '{b}: {c} ({d}%)' },
      legend: {
        bottom: 0,
        itemWidth: 10,
        itemHeight: 10,
        textStyle: { color: colors.text3, fontSize: 12 },
      },
      series: [
        {
          type: 'pie' as const,
          radius: ['52%', '74%'],
          center: ['50%', '44%'],
          avoidLabelOverlap: true,
          itemStyle: { borderRadius: 8, borderColor: colors.surface, borderWidth: 2 },
          label: { show: true, color: colors.text3, fontSize: 12, formatter: '{d}%' },
          data: [
            { value: gpuUsed, name: '已使用', itemStyle: { color: chartPalette.gpu } },
            {
              value: Math.max(gpuTotal - gpuUsed, 0),
              name: '可分配',
              itemStyle: { color: colors.line },
            },
          ],
        },
      ],
    }),
    [gpuUsed, gpuTotal, colors],
  );

  const jobOption = useMemo(
    () => ({
      tooltip: { trigger: 'item' as const, formatter: '{b}: {c} ({d}%)' },
      legend: {
        bottom: 0,
        itemWidth: 10,
        itemHeight: 10,
        textStyle: { color: colors.text3, fontSize: 12 },
      },
      series: [
        {
          type: 'pie' as const,
          radius: ['52%', '74%'],
          center: ['50%', '44%'],
          avoidLabelOverlap: true,
          itemStyle: { borderRadius: 8, borderColor: colors.surface, borderWidth: 2 },
          label: { show: true, color: colors.text3, fontSize: 12, formatter: '{d}%' },
          data: jobStatusData,
        },
      ],
    }),
    [jobStatusData, colors],
  );

  const handleReorder = (ids: string[]) => setLayout((prev) => ({ ...prev, order: ids }));

  const handleMove = (id: string, dir: -1 | 1) =>
    setLayout((prev) => {
      const order = [...prev.order];
      const from = order.indexOf(id);
      const to = from + dir;
      if (from < 0 || to < 0 || to >= order.length) return prev;
      [order[from], order[to]] = [order[to], order[from]];
      return { ...prev, order };
    });

  const handleToggleHidden = (id: string) =>
    setLayout((prev) => ({
      ...prev,
      hidden: prev.hidden.includes(id)
        ? prev.hidden.filter((h) => h !== id)
        : [...prev.hidden, id],
    }));

  const handleReset = () => {
    setLayout({ order: DEFAULT_ORDER, hidden: [] });
    try {
      localStorage.removeItem(STORAGE_KEY);
    } catch {
      /* 忽略 */
    }
  };

  /**
   * 统一的 KPI 卡。
   * 注意：antd Statistic 的 value 仅接受 string | number（传入 ReactNode 会被字符串化），
   * 因此单位一律通过 suffix 传入。
   */
  const statCard = (
    icon: React.ReactNode,
    bg: string,
    title: string,
    value: string | number,
    suffix: React.ReactNode,
    footer: React.ReactNode,
  ) => (
    <Card styles={{ body: { padding: 20 } }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
        <div
          style={{
            width: 44,
            height: 44,
            borderRadius: 'var(--mc-radius-lg)',
            background: bg,
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            color: '#fff',
            fontSize: 20,
            flexShrink: 0,
          }}
        >
          {icon}
        </div>
        <div style={{ flex: 1, minWidth: 0 }}>
          <Statistic
            className="mc-num"
            title={title}
            value={value}
            suffix={suffix}
            valueStyle={{ fontSize: 'var(--mc-fs-metric)', fontWeight: 680, letterSpacing: '-0.5px' }}
          />
          <div style={{ marginTop: 4, minHeight: 20 }}>{footer}</div>
        </div>
      </div>
    </Card>
  );

  const cards: GridItem[] = [
    {
      id: 'kpi-cluster',
      span: { xs: 12, sm: 12, md: 6, lg: 6, xl: 3, '2xl': 3 },
      title: '集群数量',
      node: statCard(
        <DatabaseOutlined />,
        'linear-gradient(135deg, #2f6bff, #5b8bff)',
        '集群数量',
        clusterCount,
        <span style={{ fontSize: 14, fontWeight: 400, color: 'var(--mc-text-3)' }}> 个</span>,
        <span style={{ fontSize: 12.5, color: 'var(--mc-text-3)' }}>统一纳管的计算集群</span>,
      ),
    },
    {
      id: 'kpi-util',
      span: { xs: 12, sm: 12, md: 6, lg: 6, xl: 3, '2xl': 3 },
      title: 'GPU 利用率',
      node: statCard(
        <CloudOutlined />,
        'linear-gradient(135deg, #00b8a9, #22d3c5)',
        'GPU 利用率',
        utilization,
        '%',
        <Progress
          percent={utilization}
          size="small"
          status={utilization > 80 ? 'exception' : 'normal'}
          strokeColor={utilization > 80 ? statusColor.failed : chartPalette.teal}
          showInfo={false}
        />,
      ),
    },
    {
      id: 'kpi-running',
      span: { xs: 12, sm: 12, md: 6, lg: 6, xl: 3, '2xl': 3 },
      title: '运行中作业',
      node: statCard(
        <ClockCircleOutlined />,
        'linear-gradient(135deg, #7c5cff, #9b84ff)',
        '运行中作业',
        runningJobs,
        <span style={{ fontSize: 14, fontWeight: 400, color: 'var(--mc-text-3)' }}>
          {' '}
          / {totalJobs}
        </span>,
        <span style={{ fontSize: 12.5, color: 'var(--mc-text-3)' }}>
          排队中 <b className="mc-num">{pendingJobs}</b> 个
        </span>,
      ),
    },
    {
      id: 'kpi-alerts',
      span: { xs: 12, sm: 12, md: 6, lg: 6, xl: 3, '2xl': 3 },
      title: '活跃告警',
      node: statCard(
        <BellOutlined />,
        alertCount > 0
          ? 'linear-gradient(135deg, #f0445a, #ff616e)'
          : 'linear-gradient(135deg, #12b886, #2bd9a6)',
        '活跃告警',
        alertCount,
        ' 条',
        <span
          style={{
            fontSize: 12.5,
            color: alertCount > 0 ? 'var(--mc-danger)' : 'var(--mc-success)',
          }}
        >
          {alertCount > 0 ? '需要关注' : '运行正常'}
        </span>,
      ),
    },
    {
      id: 'chart-resource',
      span: { xs: 12, sm: 12, md: 12, lg: 6, xl: 6, '2xl': 6 },
      title: '资源分布',
      node: (
        <Card title="资源分布" extra={<span style={{ fontSize: 12, color: 'var(--mc-text-3)' }}>GPU 卡</span>}>
          {gpuTotal > 0 ? (
            <ResponsiveChart option={resourceOption} size="md" />
          ) : (
            <EmptyState
              title="暂无资源数据"
              description="接入 GPU 资源后，这里会展示已使用与可分配的占比。"
            />
          )}
        </Card>
      ),
    },
    {
      id: 'chart-job',
      span: { xs: 12, sm: 12, md: 12, lg: 6, xl: 6, '2xl': 6 },
      title: '作业状态分布',
      node: (
        <Card title="作业状态分布" extra={<span style={{ fontSize: 12, color: 'var(--mc-text-3)' }}>共 {totalJobs} 个</span>}>
          {jobStatusData.length > 0 ? (
            <ResponsiveChart option={jobOption} size="md" />
          ) : (
            <EmptyState
              title="还没有作业"
              description="提交第一个作业后，这里会按状态展示分布。"
            />
          )}
        </Card>
      ),
    },
    {
      id: 'alerts',
      span: { xs: 12, sm: 12, md: 12, lg: 12, xl: 12, '2xl': 12 },
      title: '最近告警',
      node: (
        <Card title="最近告警" extra={<span style={{ fontSize: 12, color: 'var(--mc-text-3)' }}>按严重程度排序</span>}>
          {alertsData.length > 0 ? (
            <List
              dataSource={alertsData.slice(0, 6)}
              renderItem={(item: Record<string, unknown>) => {
                const level = String(item.level ?? 'info');
                const color = statusColor[level] ?? colors.text3;
                return (
                  <List.Item style={{ padding: '12px 0' }}>
                    <List.Item.Meta
                      title={
                        <Space>
                          <span className={`mc-status ${level === 'critical' ? 'failed' : 'pending'}`}>
                            <i className="mc-status-dot" style={{ background: color }} />
                            {statusText[level] ?? level}
                          </span>
                          <span>{String(item.message ?? item.title ?? '未知告警')}</span>
                        </Space>
                      }
                      description={
                        <span style={{ fontSize: 12.5, color: 'var(--mc-text-3)' }}>
                          {String(item.details ?? item.description ?? '暂无详情')}
                        </span>
                      }
                    />
                  </List.Item>
                );
              }}
            />
          ) : (
            <EmptyState title="暂无告警" description="系统运行正常，有新的告警会第一时间出现在这里。" />
          )}
        </Card>
      ),
    },
  ];

  // 编辑模式下渲染全部卡片（含已隐藏），便于重新显示与调整顺序
  const orderedCards = useMemo(() => {
    const map = new Map(cards.map((c) => [c.id, c]));
    const ordered = layout.order
      .map((id) => map.get(id))
      .filter((c): c is GridItem => Boolean(c));
    return editable ? ordered : ordered.filter((c) => !layout.hidden.includes(c.id));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [layout, editable, clusterCount, utilization, runningJobs, alertCount, totalJobs, pendingJobs, gpuUsed, gpuTotal, jobStatusData, resourceOption, jobOption, alertsData]);

  const skeletonCards = (
    <div className="dg">
      {[0, 1, 2, 3].map((i) => (
        <div key={i} className="dg-item dg-span-xs-12 dg-span-md-6 dg-span-xl-3">
          <Card>
            <Skeleton active paragraph={{ rows: 2 }} />
          </Card>
        </div>
      ))}
      <div className="dg-item dg-span-xs-12 dg-span-lg-6">
        <Card>
          <Skeleton active paragraph={{ rows: 7 }} />
        </Card>
      </div>
      <div className="dg-item dg-span-xs-12">
        <Card>
          <Skeleton active paragraph={{ rows: 4 }} />
        </Card>
      </div>
    </div>
  );

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">算力总览</h1>
          <p className="mc-page-desc">
            {range} · 实时掌握全平台算力水位、作业吞吐与风险告警
          </p>
        </div>
        <div className="mc-page-head-extra">
          <Segmented
            value={range}
            onChange={(v) => setRange(String(v))}
            options={['今日', '近 7 天', '近 30 天']}
          />
          <Tooltip title="刷新数据">
            <Button icon={<ReloadOutlined />} onClick={refetchAll}>
              刷新
            </Button>
          </Tooltip>
          <Button type={editable ? 'primary' : 'default'} onClick={() => setEditable((v) => !v)}>
            {editable ? '完成布局' : '自定义布局'}
          </Button>
          {editable && <Button onClick={handleReset}>恢复默认</Button>}
        </div>
      </div>

      {hasError ? (
        <ErrorState
          title="仪表盘数据加载失败"
          description="部分接口请求失败，可尝试重新加载；若持续失败请联系平台管理员。"
          onRetry={refetchAll}
        />
      ) : isLoading ? (
        skeletonCards
      ) : (
        <DraggableGrid
          items={orderedCards}
          editable={editable}
          hiddenIds={layout.hidden}
          onReorder={handleReorder}
          onMove={handleMove}
          onToggleHidden={handleToggleHidden}
        />
      )}
    </div>
  );
};

export default Dashboard;
