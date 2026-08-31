import React from 'react';
import { Card, Table, Progress, Tag } from 'antd';
import { useGetResourcesQuery } from '../store/api';
import { extractArrayData } from '../utils/api';

const ResourceManagement: React.FC = () => {
  const { data: resources, isLoading } = useGetResourcesQuery(undefined);
  const resourcesData = extractArrayData(resources);

  const columns = [
    { title: 'ID', dataIndex: 'id', key: 'id' },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag>{type}</Tag> },
    { title: '状态', dataIndex: 'status', key: 'status', render: (status: string) => <Tag color={status === 'available' ? 'green' : 'red'}>{status}</Tag> },
    { title: '总量', dataIndex: 'total', key: 'total' },
    { title: '已用', dataIndex: 'used', key: 'used' },
    { title: '可用', dataIndex: 'available', key: 'available' },
    { title: '利用率', dataIndex: 'utilization', key: 'utilization', render: (util: number) => <Progress percent={util} size="small" /> },
  ];

  return (
    <div className="p-6">
      <Card title="资源管理">
        <Table
          columns={columns}
          dataSource={resourcesData}
          loading={isLoading}
          rowKey="id"
        />
      </Card>
    </div>
  );
};

export default ResourceManagement;