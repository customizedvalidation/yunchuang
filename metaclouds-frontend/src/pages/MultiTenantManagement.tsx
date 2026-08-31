import React from 'react';
import { Card, Table, Button, Space, Tag, message, Modal, Form, Input, InputNumber } from 'antd';
import { useGetTenantsQuery, useCreateTenantMutation, useDeleteTenantMutation } from '../store/api';
import { extractArrayData } from '../utils/api';

const MultiTenantManagement: React.FC = () => {
  const { data: tenants, isLoading, refetch } = useGetTenantsQuery(undefined);
  const tenantsData = extractArrayData(tenants);
  const [createTenant] = useCreateTenantMutation();
  const [deleteTenant] = useDeleteTenantMutation();
  const [isModalVisible, setIsModalVisible] = React.useState(false);
  const [form] = Form.useForm();

  const handleCreate = async (values: any) => {
    try {
      await createTenant(values).unwrap();
      message.success('租户创建成功');
      setIsModalVisible(false);
      form.resetFields();
      refetch();
    } catch (error) {
      message.error('租户创建失败');
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await deleteTenant(id).unwrap();
      message.success('租户删除成功');
      refetch();
    } catch (error) {
      message.error('租户删除失败');
    }
  };

  const columns = [
    { title: 'ID', dataIndex: 'id', key: 'id' },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '描述', dataIndex: 'description', key: 'description' },
    { title: '状态', dataIndex: 'status', key: 'status', render: (status: string) => <Tag color={status === 'active' ? 'green' : 'red'}>{status}</Tag> },
    { title: 'GPU配额', dataIndex: 'gpu_quota', key: 'gpu_quota' },
    { title: 'CPU配额', dataIndex: 'cpu_quota', key: 'cpu_quota' },
    { title: '内存配额(GB)', dataIndex: 'memory_quota', key: 'memory_quota' },
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
        title="多租户管理"
        extra={
          <Button type="primary" onClick={() => setIsModalVisible(true)}>
            创建租户
          </Button>
        }
      >
        <Table
          columns={columns}
          dataSource={tenantsData}
          loading={isLoading}
          rowKey="id"
        />
      </Card>

      <Modal
        title="创建租户"
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
          <Form.Item name="gpu_quota" label="GPU配额" rules={[{ required: true }]}>
            <InputNumber min={0} />
          </Form.Item>
          <Form.Item name="cpu_quota" label="CPU配额" rules={[{ required: true }]}>
            <InputNumber min={0} />
          </Form.Item>
          <Form.Item name="memory_quota" label="内存配额(GB)" rules={[{ required: true }]}>
            <InputNumber min={0} />
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

export default MultiTenantManagement;