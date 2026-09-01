import { Can } from '../components/Can';
import React from 'react';
import { Card, Table } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { Button, Space, App, Modal, Form, Input, InputNumber, Popconfirm } from 'antd';
import { useGetClustersQuery, useCreateClusterMutation, useDeleteClusterMutation } from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { Cluster } from '../types';

/** 新建集群表单值 */
interface ClusterFormValues {
  name: string;
  description?: string;
  nodes?: number;
  gpus?: number;
}

const ClusterManagement: React.FC = () => {
  const { message } = App.useApp();
  const { data: clusters, isLoading, error, refetch } = useGetClustersQuery(undefined);
  const clustersData = extractArrayData<Cluster>(clusters);
  const [createCluster] = useCreateClusterMutation();
  const [deleteCluster] = useDeleteClusterMutation();
  const [isModalVisible, setIsModalVisible] = React.useState(false);
  const [form] = Form.useForm();

  const handleCreate = async (values: ClusterFormValues) => {
    try {
      await createCluster(values).unwrap();
      message.success('集群创建成功');
      setIsModalVisible(false);
      form.resetFields();
      refetch();
    } catch {
      message.error('集群创建失败，请检查必填项后重试');
    }
  };

  const handleDelete = async (id: number) => {
    try {
      await deleteCluster(id).unwrap();
      message.success('集群删除成功');
      refetch();
    } catch {
      message.error('集群删除失败，请稍后重试');
    }
  };

  const columns: ColumnsType<Cluster> = [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 90, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '描述', dataIndex: 'description', key: 'description' },
    { title: '状态', dataIndex: 'status', key: 'status', width: 110, render: (status: string) => <StatusCell status={status} /> },
    { title: '节点数', dataIndex: 'nodes', key: 'nodes', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: 'GPU数', dataIndex: 'gpus', key: 'gpus', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    {
      title: '操作', key: 'action', width: 100,
      render: (_: unknown, record: Cluster) => (
        <Space>
          <Can perm="cluster:write">
          <Popconfirm
            title="删除该集群？"
            description="集群下的节点与配额将一并移除，且不可恢复。"
            okText="确认删除"
            cancelText="返回"
            okButtonProps={{ danger: true }}
            onConfirm={() => handleDelete(record.id)}
          >
            <Button type="link" danger size="small">删除</Button>
          </Popconfirm>
          </Can>
        </Space>
      ),
    },
  ];

  const state = renderState({
    isLoading,
    error,
    isEmpty: clustersData.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 7,
    empty: (
      <EmptyState
        title="还没有集群"
        description="创建第一个集群来管理算力节点与 GPU 配额。"
        action={<Can perm="cluster:write"><Button type="primary" onClick={() => setIsModalVisible(true)}>创建集群</Button></Can>}
      />
    ),
  });

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">集群管理</h1>
          <p className="mc-page-desc">共 {clustersData.length} 个集群 · 管理算力节点、GPU 配额与调度边界</p>
        </div>
        <div className="mc-page-head-extra">
          <Can perm="cluster:write"><Button type="primary" onClick={() => setIsModalVisible(true)}>创建集群</Button></Can>
        </div>
      </div>

      <Card>
        {state ?? (
          <Table
            columns={columns}
            dataSource={clustersData}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 820 }}
          />
        )}
      </Card>

      <Modal title="创建集群" open={isModalVisible} onCancel={() => setIsModalVisible(false)} footer={null} destroyOnHidden>
        <Form form={form} onFinish={handleCreate} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入集群名称' }]}>
            <Input placeholder="例如：gpu-cluster-sh" />
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea rows={3} placeholder="选填，便于后续追溯" />
          </Form.Item>
          <Form.Item name="nodes" label="节点数" rules={[{ required: true, message: '请输入节点数' }]}>
            <InputNumber min={1} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="gpus" label="GPU数" rules={[{ required: true, message: '请输入 GPU 数' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Can perm="cluster:write"><Button type="primary" htmlType="submit">创建</Button></Can>
              <Button onClick={() => setIsModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default ClusterManagement;
