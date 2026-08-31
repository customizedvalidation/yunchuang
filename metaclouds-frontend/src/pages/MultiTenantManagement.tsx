import React from 'react';
import { Card, Table, Button, Space, App, Modal, Form, Input, InputNumber, Popconfirm } from 'antd';
import { useGetTenantsQuery, useCreateTenantMutation, useDeleteTenantMutation } from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';

const MultiTenantManagement: React.FC = () => {
  const { message } = App.useApp();
  const { data: tenants, isLoading, error, refetch } = useGetTenantsQuery(undefined);
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
    } catch {
      message.error('租户创建失败，请检查必填项后重试');
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await deleteTenant(id).unwrap();
      message.success('租户删除成功');
      refetch();
    } catch {
      message.error('租户删除失败，请稍后重试');
    }
  };

  const columns = [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 80, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '描述', dataIndex: 'description', key: 'description' },
    { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (status: string) => <StatusCell status={status} /> },
    { title: 'GPU配额', dataIndex: 'gpu_quota', key: 'gpu_quota', width: 100, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: 'CPU配额', dataIndex: 'cpu_quota', key: 'cpu_quota', width: 100, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '内存配额(GB)', dataIndex: 'memory_quota', key: 'memory_quota', width: 120, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    {
      title: '操作', key: 'action', width: 90,
      render: (_: any, record: any) => (
        <Space>
          <Popconfirm
            title="删除该租户？"
            description="租户下的配额与关联资源将一并移除，且不可恢复。"
            okText="确认删除"
            cancelText="返回"
            okButtonProps={{ danger: true }}
            onConfirm={() => handleDelete(record.id)}
          >
            <Button type="link" danger size="small">删除</Button>
          </Popconfirm>
        </Space>
      ),
    },
  ];

  const state = renderState({
    isLoading,
    error,
    isEmpty: tenantsData.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 8,
    empty: (
      <EmptyState
        title="还没有租户"
        description="创建租户以划分命名空间与配额边界。"
        action={<Button type="primary" onClick={() => setIsModalVisible(true)}>创建租户</Button>}
      />
    ),
  });

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">多租户管理</h1>
          <p className="mc-page-desc">共 {tenantsData.length} 个租户 · 按租户划分命名空间与配额边界</p>
        </div>
        <div className="mc-page-head-extra">
          <Button type="primary" onClick={() => setIsModalVisible(true)}>创建租户</Button>
        </div>
      </div>

      <Card>
        {state ?? (
          <Table
            columns={columns}
            dataSource={tenantsData}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 900 }}
          />
        )}
      </Card>

      <Modal title="创建租户" open={isModalVisible} onCancel={() => setIsModalVisible(false)} footer={null} destroyOnHidden>
        <Form form={form} onFinish={handleCreate} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入租户名称' }]}>
            <Input placeholder="例如：team-vision" />
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea rows={3} placeholder="选填，便于后续追溯" />
          </Form.Item>
          <Form.Item name="gpu_quota" label="GPU配额" rules={[{ required: true, message: '请输入 GPU 配额' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="cpu_quota" label="CPU配额" rules={[{ required: true, message: '请输入 CPU 配额' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="memory_quota" label="内存配额(GB)" rules={[{ required: true, message: '请输入内存配额' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Button type="primary" htmlType="submit">创建</Button>
              <Button onClick={() => setIsModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default MultiTenantManagement;
