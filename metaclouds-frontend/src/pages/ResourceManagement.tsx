import React, { useMemo, useState } from 'react';
import { Card, Progress, Tag, Select, Space } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import { useGetResourcesQuery } from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { Resource, GPUVendor } from '../types';

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

const ResourceManagement: React.FC = () => {
  const [vendorFilter, setVendorFilter] = useState<string | undefined>(undefined);

  const { data: resources, isLoading, error, refetch } = useGetResourcesQuery(undefined);
  const resourcesData = extractArrayData<Resource>(resources);

  // 按厂商筛选
  const filteredData = useMemo(() => {
    if (!vendorFilter) return resourcesData;
    return resourcesData.filter((r) => r.vendor === vendorFilter);
  }, [resourcesData, vendorFilter]);

  // 列配置用 useMemo 缓存：不随渲染变化，避免 Table 因 columns 引用变化而深比较重渲染
  const columns = useMemo(() => [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 80, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag>{type}</Tag> },
    {
      title: '厂商',
      dataIndex: 'vendor',
      key: 'vendor',
      width: 100,
      render: (v: GPUVendor) => (v ? <Tag color={VENDOR_COLOR[v]}>{v}</Tag> : '-'),
    },
    { title: 'GPU型号', dataIndex: 'gpu_model', key: 'gpu_model', width: 120 },
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
      title: '超发比率',
      dataIndex: 'vram_oversubscription_ratio',
      key: 'vram_oversubscription_ratio',
      width: 100,
      render: (v: number) => (v != null ? <span className="mc-num">{v}x</span> : '-'),
    },
    {
      title: 'MIG',
      dataIndex: 'mig_enabled',
      key: 'mig_enabled',
      width: 80,
      render: (v: boolean) => (v ? <Tag color="blue">开启</Tag> : <Tag>关闭</Tag>),
    },
    { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (status: string) => <StatusCell status={status} /> },
    { title: '总量', dataIndex: 'total', key: 'total', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '已用', dataIndex: 'used', key: 'used', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '可用', dataIndex: 'available', key: 'available', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '利用率', dataIndex: 'utilization', key: 'utilization', width: 140, render: (util: number) => <Progress percent={util} size="small" /> },
  ], []);

  const state = renderState({
    isLoading,
    error,
    isEmpty: filteredData.length === 0,
    onRetry: refetch,
    skeletonRows: 6,
    skeletonColumns: 8,
    empty: (
      <EmptyState title="暂无资源" description="还没有登记任何算力资源，新增节点后将在此汇总总量、已用与利用率。" />
    ),
  });

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">资源管理</h1>
          <p className="mc-page-desc">算力资源总量、已用与利用率总览</p>
        </div>
      </div>

      <Card style={{ marginBottom: 16 }}>
        <Space>
          <Select
            placeholder="厂商筛选"
            allowClear
            style={{ width: 180 }}
            value={vendorFilter}
            onChange={(v) => setVendorFilter(v)}
            options={VENDOR_OPTIONS}
          />
        </Space>
      </Card>

      <Card>
        {state ?? (
          <ResponsiveTable
            columns={columns}
            dataSource={filteredData}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 1500, y: 520 }}
            // 资源列表可能包含大量 GPU/CPU 节点，启用虚拟滚动
            virtual
          />
        )}
      </Card>
    </div>
  );
};

export default ResourceManagement;
