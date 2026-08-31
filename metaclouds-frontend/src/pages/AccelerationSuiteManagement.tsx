import React from 'react';
import { Card, Table, Tag, Switch, message } from 'antd';
import { useGetAccelerationSuitesQuery, useUpdateAccelerationSuiteMutation } from '../store/api';
import { extractArrayData } from '../utils/api';

const AccelerationSuiteManagement: React.FC = () => {
  const { data: suites, isLoading, refetch } = useGetAccelerationSuitesQuery(undefined);
  const suitesData = extractArrayData(suites);
  const [updateSuite] = useUpdateAccelerationSuiteMutation();

  const handleToggle = async (id: number, enabled: boolean) => {
    try {
      await updateSuite({ id, enabled }).unwrap();
      message.success(`加速套件${enabled ? '启用' : '禁用'}成功`);
      refetch();
    } catch (error) {
      message.error('操作失败');
    }
  };

  const getTypeColor = (type: string) => {
    switch (type) {
      case 'data': return 'blue';
      case 'training': return 'green';
      case 'inference': return 'purple';
      default: return 'default';
    }
  };

  const columns = [
    { title: 'ID', dataIndex: 'id', key: 'id' },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag color={getTypeColor(type)}>{type}</Tag> },
    { title: '版本', dataIndex: 'version', key: 'version' },
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
      <Card title="加速套件管理">
        <Table
          columns={columns}
          dataSource={suitesData}
          loading={isLoading}
          rowKey="id"
        />
      </Card>
    </div>
  );
};

export default AccelerationSuiteManagement;