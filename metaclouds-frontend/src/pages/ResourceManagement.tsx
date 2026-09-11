import React, { useMemo, useState } from 'react';
import { Card, Progress, Tag, Select, Space, Input, Row, Col, Statistic, Tabs, Typography } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import { useGetResourcesQuery } from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { Resource, GPUVendor } from '../types';
import { brand } from '../theme/tokens';

const { Text } = Typography;

const VENDOR_OPTIONS: { label: string; value: GPUVendor }[] = [
  { label: 'NVIDIA', value: 'nvidia' },
  { label: '燧原', value: 'enflame' },
  { label: '摩尔线程', value: 'moore_threads' },
  { label: '国产 X', value: 'domestic_x' },
];

const VENDOR_COLOR: Record<GPUVendor, string> = {
  nvidia: 'green',
  enflame: 'blue',
  moore_threads: 'purple',
  domestic_x: 'orange',
};

const VENDOR_LABEL: Record<GPUVendor, string> = {
  nvidia: 'NVIDIA',
  enflame: '燧原',
  moore_threads: '摩尔线程',
  domestic_x: '国产X',
};

/** skill.md 4.1 — AI存储资源五类 */
const STORAGE_CAPABILITIES = [
  { key: 'capacity', name: '海量存储', desc: '大容量数据存储，用于存储大规模数据集' },
  { key: 'parallel', name: '并行文件存储', desc: 'Lustre/BeeGFS 并行文件系统，支持高并发数据访问' },
  { key: 'dense', name: '高密度存储', desc: '高密度存储方案，提高存储密度与性价比' },
  { key: 'allflash', name: '全闪存存储', desc: '全闪存高性能存储，低延迟、高吞吐' },
  { key: 'object', name: '对象存储', desc: '分布式对象存储，存放非结构化数据与模型文件' },
];

/** skill.md 4.1 — AI网络资源五类 */
const NETWORK_CAPABILITIES = [
  { key: 'roce', name: 'RoCE网络', desc: '基于以太网的 RDMA，高速节点间通信' },
  { key: 'ib', name: 'IB网络', desc: 'InfiniBand 高速网络，低延迟高带宽' },
  { key: 'lb', name: '负载均衡', desc: '网络流量负载均衡，优化流量分布' },
  { key: 'mgmt', name: '管理网', desc: '集群管理网络，承载节点间管理通信' },
  { key: 'storage_net', name: '存储网', desc: '存储专用网络，服务存储系统间通信' },
];

/** 能力卡：科技蓝渐变标题条 */
const CapabilityCard: React.FC<{ name: string; desc: string }> = ({ name, desc }) => (
  <Card size="small" style={{ height: '100%' }}>
    <div
      style={{
        display: 'inline-block',
        padding: '2px 10px',
        borderRadius: 999,
        background: brand.grad,
        color: '#fff',
        fontSize: 12,
        marginBottom: 8,
      }}
    >
      {name}
    </div>
    <Text type="secondary" style={{ fontSize: 12 }}>{desc}</Text>
  </Card>
);

const ResourceManagement: React.FC = () => {
  const [activeTab, setActiveTab] = useState<string>('compute');
  const [vendorFilter, setVendorFilter] = useState<string | undefined>(undefined);
  const [keyword, setKeyword] = useState<string>('');

  const { data: resources, isLoading, error, refetch } = useGetResourcesQuery(undefined);
  const resourcesData = extractArrayData<Resource>(resources);

  // 名称模糊搜索
  const searchedData = useMemo(() => {
    if (!keyword.trim()) return resourcesData;
    const kw = keyword.trim().toLowerCase();
    return resourcesData.filter(
      (r) => r.name.toLowerCase().includes(kw) || (r.gpu_model ?? '').toLowerCase().includes(kw),
    );
  }, [resourcesData, keyword]);

  // 计算资源（gpu / cpu）
  const computeData = useMemo(() => {
    const list = searchedData.filter((r) => r.type === 'gpu' || r.type === 'cpu');
    if (!vendorFilter) return list;
    return list.filter((r) => r.vendor === vendorFilter);
  }, [searchedData, vendorFilter]);

  // 存储 / 网络资源
  const storageData = useMemo(() => searchedData.filter((r) => r.type === 'storage' || r.type === 'memory'), [searchedData]);
  const networkData = useMemo(() => searchedData.filter((r) => r.type === 'network'), [searchedData]);

  // 计算资源分厂商统计
  const vendorStats = useMemo(() => {
    const gpuList = resourcesData.filter((r) => r.type === 'gpu');
    const byVendor = (v: GPUVendor) => gpuList.filter((r) => r.vendor === v).length;
    const cpuCount = resourcesData.filter((r) => r.type === 'cpu').reduce((s, r) => s + (r.total ?? 0), 0);
    return {
      nvidia: byVendor('nvidia'),
      enflame: byVendor('enflame'),
      moore_threads: byVendor('moore_threads'),
      domestic_x: byVendor('domestic_x'),
      cpu: cpuCount,
    };
  }, [resourcesData]);

  // 列配置：计算资源
  const computeColumns = useMemo<ColumnsType<Resource>>(() => [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 80, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag color={type === 'cpu' ? 'geekblue' : 'blue'}>{type === 'cpu' ? 'CPU' : 'GPU'}</Tag> },
    {
      title: '厂商',
      dataIndex: 'vendor',
      key: 'vendor',
      width: 100,
      render: (v?: GPUVendor) => (v ? <Tag color={VENDOR_COLOR[v]}>{VENDOR_LABEL[v]}</Tag> : <Tag>通用</Tag>),
    },
    { title: 'GPU型号', dataIndex: 'gpu_model', key: 'gpu_model', width: 120, render: (v: React.ReactNode) => v ?? '-' },
    {
      title: '显存总量(MB)',
      dataIndex: 'vram_total_mb',
      key: 'vram_total_mb',
      width: 120,
      render: (v: React.ReactNode) => <span className="mc-num">{v ?? '-'}</span>,
    },
    {
      title: '显存已用(MB)',
      dataIndex: 'vram_used_mb',
      key: 'vram_used_mb',
      width: 120,
      render: (v: React.ReactNode) => <span className="mc-num">{v ?? '-'}</span>,
    },
    {
      title: '显存超发比率',
      dataIndex: 'vram_oversubscription_ratio',
      key: 'vram_oversubscription_ratio',
      width: 110,
      render: (v: number) => (v != null ? <Tag color="purple">{v}x</Tag> : '-'),
    },
    {
      title: 'MIG',
      dataIndex: 'mig_enabled',
      key: 'mig_enabled',
      width: 80,
      render: (v: boolean) => (v ? <Tag color="blue">开启</Tag> : <Tag>关闭</Tag>),
    },
    { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (status: string) => <StatusCell status={status} /> },
    { title: '总量', dataIndex: 'total', key: 'total', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v ?? '-'}</span> },
    { title: '已用', dataIndex: 'used', key: 'used', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v ?? '-'}</span> },
    { title: '可用', dataIndex: 'available', key: 'available', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v ?? '-'}</span> },
    {
      title: '利用率',
      dataIndex: 'utilization',
      key: 'utilization',
      width: 140,
      render: (util: number) => <Progress percent={util ?? 0} size="small" />,
    },
  ], []);

  // 通用资源列（存储 / 网络）
  const genericColumns = useMemo<ColumnsType<Resource>>(() => [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 80, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'type', key: 'type', width: 110, render: (type: string) => <Tag color="cyan">{type}</Tag> },
    { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (status: string) => <StatusCell status={status} /> },
    { title: '总量', dataIndex: 'total', key: 'total', width: 100, render: (v: React.ReactNode) => <span className="mc-num">{v ?? '-'}</span> },
    { title: '已用', dataIndex: 'used', key: 'used', width: 100, render: (v: React.ReactNode) => <span className="mc-num">{v ?? '-'}</span> },
    { title: '可用', dataIndex: 'available', key: 'available', width: 100, render: (v: React.ReactNode) => <span className="mc-num">{v ?? '-'}</span> },
    {
      title: '利用率',
      dataIndex: 'utilization',
      key: 'utilization',
      width: 140,
      render: (util: number) => <Progress percent={util ?? 0} size="small" />,
    },
    { title: '详情', dataIndex: 'details', key: 'details', render: (v: React.ReactNode) => v ?? '-' },
  ], []);

  const state = renderState({
    isLoading,
    error,
    isEmpty: resourcesData.length === 0,
    onRetry: refetch,
    skeletonRows: 6,
    skeletonColumns: 8,
    empty: (
      <EmptyState title="暂无资源" description="还没有登记任何算力 / 存储 / 网络资源，新增节点后将在此分类汇总。" />
    ),
  });

  const renderTable = (data: Resource[], columns: ColumnsType<Resource>, emptyText: string) => {
    if (state) return state;
    if (data.length === 0) {
      return <EmptyState title={emptyText} description="当前分类下暂无登记资源。" />;
    }
    return (
      <ResponsiveTable
        columns={columns}
        dataSource={data}
        rowKey="id"
        pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
        scroll={{ x: 1500, y: 480 }}
        virtual
      />
    );
  };

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">基础资源管理</h1>
          <p className="mc-page-desc">AI 计算资源池 · AI 存储资源 · AI 网络资源 三大基础层总览</p>
        </div>
      </div>

      <Card style={{ marginBottom: 16 }}>
        <Space wrap>
          <Input.Search
            placeholder="搜索资源名称 / GPU 型号"
            allowClear
            style={{ width: 260 }}
            value={keyword}
            onChange={(e) => setKeyword(e.target.value)}
            aria-label="搜索资源"
          />
          {activeTab === 'compute' && (
            <Select
              placeholder="厂商筛选"
              allowClear
              style={{ width: 180 }}
              value={vendorFilter}
              onChange={(v) => setVendorFilter(v)}
              options={VENDOR_OPTIONS}
            />
          )}
        </Space>
      </Card>

      <Tabs
        activeKey={activeTab}
        onChange={setActiveTab}
        items={[
          {
            key: 'compute',
            label: 'AI 计算资源池',
            children: (
              <div>
                {/* 多厂商 GPU + CPU 统计卡 */}
                <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
                  <Col xs={12} sm={8} md={5} lg={4}>
                    <Card><Statistic title="NVIDIA" value={vendorStats.nvidia} suffix="台" /></Card>
                  </Col>
                  <Col xs={12} sm={8} md={5} lg={4}>
                    <Card><Statistic title="燧原 Enflame" value={vendorStats.enflame} suffix="台" /></Card>
                  </Col>
                  <Col xs={12} sm={8} md={5} lg={4}>
                    <Card><Statistic title="摩尔线程" value={vendorStats.moore_threads} suffix="台" /></Card>
                  </Col>
                  <Col xs={12} sm={8} md={5} lg={4}>
                    <Card><Statistic title="国产 X" value={vendorStats.domestic_x} suffix="台" /></Card>
                  </Col>
                  <Col xs={12} sm={8} md={5} lg={4}>
                    <Card><Statistic title="CPU 资源" value={vendorStats.cpu} suffix="核" /></Card>
                  </Col>
                </Row>
                <Card>
                  {renderTable(computeData, computeColumns, '暂无计算资源')}
                </Card>
              </div>
            ),
          },
          {
            key: 'storage',
            label: 'AI 存储资源',
            children: (
              <div>
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: 16, marginBottom: 16 }}>
                  {STORAGE_CAPABILITIES.map((c) => (
                    <div key={c.key} style={{ flex: '1 1 200px', minWidth: 200 }}>
                      <CapabilityCard name={c.name} desc={c.desc} />
                    </div>
                  ))}
                </div>
                <Card>
                  {renderTable(storageData, genericColumns, '暂无存储资源')}
                </Card>
              </div>
            ),
          },
          {
            key: 'network',
            label: 'AI 网络资源',
            children: (
              <div>
                <div style={{ display: 'flex', flexWrap: 'wrap', gap: 16, marginBottom: 16 }}>
                  {NETWORK_CAPABILITIES.map((c) => (
                    <div key={c.key} style={{ flex: '1 1 200px', minWidth: 200 }}>
                      <CapabilityCard name={c.name} desc={c.desc} />
                    </div>
                  ))}
                </div>
                <Card>
                  {renderTable(networkData, genericColumns, '暂无网络资源')}
                </Card>
              </div>
            ),
          },
        ]}
      />
    </div>
  );
};

export default ResourceManagement;
