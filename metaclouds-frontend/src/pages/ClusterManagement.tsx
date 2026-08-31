import React from 'react';
import { Card, Table, Button, Space, message, Modal, Form, Input } from 'antd';
import { useGetClustersQuery, useCreateClusterMutation, useDeleteClusterMutation } from '../store/api';
import { extractArrayData } from '../utils/api';

const ClusterManagement: React.FC = () => {
  const { data: clusters, isLoading, refetch } = useGetClustersQuery(undefined);
  const clustersData = extractArrayData(clusters);
  const [createCluster] = useCreateClusterMutation();
  const [deleteCluster] = useDeleteClusterMutation();
  const [isModalVisible, setIsModalVisible] = React.useState(false);
  const [form] = Form.useForm();

  const handleCreate = async (values: any) => {
    try {
      await createCluster(values).unwrap();
      message.success('集群创建成功');
      setIsModalVisible(false);
      form.resetFields();
      refetch();
    } catch (error) {
      message.error('集群创建失败');
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await deleteCluster(id).unwrap();
      message.success('集群删除成功');
      refetch();
    } catch (error) {
      message.error('集群删除失败');
    }
  };

  const columns = [
    { title: 'ID', dataIndex: 'id', key: 'id' },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '描述', dataIndex: 'description', key: 'description' },
    { title: '状态', dataIndex: 'status', key: 'status' },
    { title: '节点数', dataIndex: 'nodes', key: 'nodes' },
    { title: 'GPU数', dataIndex: 'gpus', key: 'gpus' },
    {
      title: '操作',
      key: 'action',
      render: (_: any, record: any) => (
        <Space>
          <Button type="link" onClick={() => handleDelete(record.id)}>删除</Button>
        </Space>
      ),
    },
  ];

  return (
    <div className="p-6">
      <Card
        title="集群管理"
        extra={
          <Button type="primary" onClick={() => setIsModalVisible(true)}>
            创建集群
          </Button>
        }
      >
        <Table
          columns={columns}
          dataSource={clustersData}
          loading={isLoading}
          rowKey="id"
        />
      </Card>

      <Modal
        title="创建集群"
        open={isModalVisible}
        onCancel={() => setIsModalVisible(false)}
        footer={null}
      >
        <Form form={form} onFinish={handleCreate} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true }]}>
            <Input />
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea />
          </Form.Item>
          <Form.Item name="nodes" label="节点数" rules={[{ required: true }]}>
            <Input type="number" />
          </Form.Item>
          <Form.Item name="gpus" label="GPU数" rules={[{ required: true }]}>
            <Input type="number" />
          </Form.Item>
          <Form.Item>
            <Button type="primary" htmlType="submit">
              创建
            </Button>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default ClusterManagement;