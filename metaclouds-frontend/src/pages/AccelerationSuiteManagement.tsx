import { Can } from '../components/Can';
import React from 'react';
import { Card } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import { Tag, Switch, App } from 'antd';
import { useGetAccelerationSuitesQuery, useUpdateAccelerationSuiteMutation } from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { AccelerationSuite } from '../types';

const AccelerationSuiteManagement: React.FC = () => {
  const { message } = App.useApp();
  const { data: suites, isLoading, error, refetch } = useGetAccelerationSuitesQuery(undefined);
  const suitesData = extractArrayData<AccelerationSuite>(suites);
  const [updateSuite] = useUpdateAccelerationSuiteMutation();

  const handleToggle = async (id: number, enabled: boolean) => {
    try {
      await updateSuite({ id, enabled }).unwrap();
      message.success(`加速套件${enabled ? '启用' : '禁用'}成功`);
      refetch();
    } catch {
      message.error('操作失败，请稍后重试');
    }
  };

  const columns: ColumnsType<AccelerationSuite> = [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 80, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag>{type}</Tag> },
    { title: '版本', dataIndex: 'version', key: 'version', width: 100 },
    { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (status: string) => <StatusCell status={status} /> },
    { title: '描述', dataIndex: 'description', key: 'description' },
    {
      title: '启用', key: 'enabled', width: 90,
      render: (_: unknown, record: AccelerationSuite) => (
        <Can perm="acceleration:write">
          <Switch checked={record.enabled} onChange={(checked) => handleToggle(record.id, checked)} />
        </Can>
      ),
    },
  ];

  const state = renderState({
    isLoading,
    error,
    isEmpty: suitesData.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 7,
    empty: <EmptyState title="暂无加速套件" description="还没有登记任何加速套件。" />,
  });

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">加速套件管理</h1>
          <p className="mc-page-desc">共 {suitesData.length} 个加速套件 · 启停推理/训练加速能力</p>
        </div>
      </div>

      <Card>
        {state ?? (
          <ResponsiveTable
            columns={columns}
            dataSource={suitesData}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 900 }}
          />
        )}
      </Card>
    </div>
  );
};

export default AccelerationSuiteManagement;
