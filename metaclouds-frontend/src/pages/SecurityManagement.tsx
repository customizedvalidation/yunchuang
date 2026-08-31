import React from 'react';
import { Card, Table, Tag, Switch, message } from 'antd';
import { useGetSecurityPoliciesQuery, useUpdateSecurityPolicyMutation } from '../store/api';
import { extractArrayData } from '../utils/api';

const SecurityManagement: React.FC = () => {
  const { data: policies, isLoading, refetch } = useGetSecurityPoliciesQuery(undefined);
  const policiesData = extractArrayData(policies);
  const [updatePolicy] = useUpdateSecurityPolicyMutation();

  const handleToggle = async (id: number, enabled: boolean) => {
    try {
      await updatePolicy({ id, enabled }).unwrap();
      message.success(`安全策略${enabled ? '启用' : '禁用'}成功`);
      refetch();
    } catch (error) {
      message.error('操作失败');
    }
  };

  const getTypeColor = (type: string) => {
    switch (type) {
      case 'access': return 'blue';
      case 'network': return 'green';
      case 'data': return 'purple';
      case 'system': return 'orange';
      default: return 'default';
    }
  };

  const columns = [
    { title: 'ID', dataIndex: 'id', key: 'id' },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag color={getTypeColor(type)}>{type}</Tag> },
    { title: '状态', dataIndex: 'status', key: 'status', render: (status: string) => <Tag color={status === 'active' ? 'green' : 'red'}>{status}</Tag> },
    { title: '描述', dataIndex: 'description', key: 'description' },
    {
      title: '启用',
      key: 'enabled',
      render: (_: any, record: any) => (
        <Switch
          checked={record.enabled}
          onChange={(checked) => handleToggle(record.id, checked)}
        />
      ),
    },
  ];

  return (
    <div className="p-6">
      <Card title="安全管理">
        <Table
          columns={columns}
          dataSource={policiesData}
          loading={isLoading}
          rowKey="id"
        />
      </Card>
    </div>
  );
};

export default SecurityManagement;