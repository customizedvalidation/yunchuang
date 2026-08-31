import React from 'react';
import { Card, Table, Progress, Tag } from 'antd';
import { useGetResourcesQuery } from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';

const ResourceManagement: React.FC = () => {
  const { data: resources, isLoading, error, refetch } = useGetResourcesQuery(undefined);
  const resourcesData = extractArrayData(resources);

  const columns = [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 80, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag>{type}</Tag> },
    { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (status: string) => <StatusCell status={status} /> },
    { title: '总量', dataIndex: 'total', key: 'total', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '已用', dataIndex: 'used', key: 'used', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '可用', dataIndex: 'available', key: 'available', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '利用率', dataIndex: 'utilization', key: 'utilization', width: 140, render: (util: number) => <Progress percent={util} size="small" /> },
  ];

  const state = renderState({
    isLoading,
    error,
    isEmpty: resourcesData.length === 0,
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

      <Card>
        {state ?? (
          <Table
            columns={columns}
            dataSource={resourcesData}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 900 }}
          />
        )}
      </Card>
    </div>
  );
};

export default ResourceManagement;
