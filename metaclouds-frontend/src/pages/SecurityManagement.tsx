import { Can } from '../components/Can';
import React from 'react';
import { Card, Tag, Switch, App } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import { useGetSecurityPoliciesQuery, useUpdateSecurityPolicyMutation } from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { SecurityPolicy } from '../types';

const SecurityManagement: React.FC = () => {
  const { message } = App.useApp();
  const { data: policies, isLoading, error, refetch } = useGetSecurityPoliciesQuery(undefined);
  const policiesData = extractArrayData<SecurityPolicy>(policies);
  const [updatePolicy] = useUpdateSecurityPolicyMutation();

  const handleToggle = async (id: number, enabled: boolean) => {
    try {
      await updatePolicy({ id, enabled }).unwrap();
      message.success(`安全策略${enabled ? '启用' : '禁用'}成功`);
      refetch();
    } catch {
      message.error('操作失败，请稍后重试');
    }
  };

  const columns: ColumnsType<SecurityPolicy> = [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 80, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag>{type}</Tag> },
    { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (status: string) => <StatusCell status={status} /> },
    { title: '描述', dataIndex: 'description', key: 'description' },
    {
      title: '启用', key: 'enabled', width: 90,
      render: (_: React.ReactNode, record: SecurityPolicy) => (
        <Can perm="security:write">
          <Switch checked={record.enabled} onChange={(checked) => handleToggle(record.id, checked)} />
        </Can>
      ),
    },
  ];

  const state = renderState({
    isLoading,
    error,
    isEmpty: policiesData.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 6,
    empty: <EmptyState title="暂无安全策略" description="还没有配置任何安全策略。" />,
  });

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">安全管理</h1>
          <p className="mc-page-desc">共 {policiesData.length} 条安全策略 · 访问控制、网络与数据防护</p>
        </div>
      </div>

      <Card>
        {state ?? (
          <ResponsiveTable
            columns={columns}
            dataSource={policiesData}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 880 }}
          />
        )}
      </Card>
    </div>
  );
};

export default SecurityManagement;
