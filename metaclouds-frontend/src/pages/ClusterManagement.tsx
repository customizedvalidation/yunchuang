import { Can } from '../components/Can';
import React, { useCallback, useMemo, useState } from 'react';
import { Card } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import { Button, Space, App, Modal, Form, Input, InputNumber, Popconfirm, Tag, Switch, Row, Col } from 'antd';
import { useGetClustersQuery, useCreateClusterMutation, useDeleteClusterMutation, useGetPartitionsQuery } from '../store/api';
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
  gpu_vendors?: string;
  scheduler_types?: string;
  multi_cluster_enabled?: boolean;
  federation_id?: string;
}

const ClusterManagement: React.FC = () => {
  const { message } = App.useApp();
  const { data: clusters, isLoading, error, refetch } = useGetClustersQuery(undefined);
  const { data: partitions } = useGetPartitionsQuery({});
  const clustersData = extractArrayData<Cluster>(clusters);
  const partitionsData = partitions ?? [];
  const [createCluster] = useCreateClusterMutation();
  const [deleteCluster] = useDeleteClusterMutation();
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [detailCluster, setDetailCluster] = useState<Cluster | null>(null);
  const [form] = Form.useForm();

  // useCallback：稳定回调引用，避免子组件（Popconfirm/Button）无谓重渲染
  const handleCreate = useCallback(async (values: ClusterFormValues) => {
    try {
      await createCluster(values).unwrap();
      message.success('集群创建成功');
      setIsModalVisible(false);
      form.resetFields();
      refetch();
    } catch {
      message.error('集群创建失败，请检查必填项后重试');
    }
  }, [createCluster, form, message, refetch]);

  const handleDelete = useCallback(async (id: number) => {
    try {
      await deleteCluster(id).unwrap();
      message.success('集群删除成功');
      refetch();
    } catch {
      message.error('集群删除失败，请稍后重试');
    }
  }, [deleteCluster, message, refetch]);

  // 集群的分区列表
  const clusterPartitions = useCallback(
    (clusterId: number) => partitionsData.filter((p) => p.cluster_id === clusterId),
    [partitionsData],
  );

  // 列配置用 useMemo 缓存
  const columns: ColumnsType<Cluster> = useMemo(() => [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 90, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    {
      title: '名称',
      dataIndex: 'name',
      key: 'name',
      render: (v: string, record) => (
        <Button type="link" size="small" style={{ padding: 0 }} onClick={() => setDetailCluster(record)}>
          {v}
        </Button>
      ),
    },
    { title: '描述', dataIndex: 'description', key: 'description' },
    { title: '状态', dataIndex: 'status', key: 'status', width: 110, render: (status: string) => <StatusCell status={status} /> },
    { title: '节点数', dataIndex: 'nodes', key: 'nodes', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: 'GPU数', dataIndex: 'gpus', key: 'gpus', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    {
      title: 'GPU厂商',
      dataIndex: 'gpu_vendors',
      key: 'gpu_vendors',
      width: 160,
      render: (v: string) => {
        if (!v) return '-';
        return v.split(',').map((vendor) => (
          <Tag key={vendor} color="blue">{vendor.trim()}</Tag>
        ));
      },
    },
    {
      title: '调度器类型',
      dataIndex: 'scheduler_types',
      key: 'scheduler_types',
      width: 160,
      render: (v: string) => {
        if (!v) return '-';
        return v.split(',').map((s) => (
          <Tag key={s} color="purple">{s.trim()}</Tag>
        ));
      },
    },
    {
      title: '多集群',
      dataIndex: 'multi_cluster_enabled',
      key: 'multi_cluster_enabled',
      width: 90,
      render: (v: boolean) => (v ? <Tag color="green">启用</Tag> : <Tag>未启用</Tag>),
    },
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
  ], [handleDelete]);

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
          <ResponsiveTable
            columns={columns}
            dataSource={clustersData}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 1300 }}
          />
        )}
      </Card>

      <Modal className="mc-modal-full" title="创建集群" open={isModalVisible} onCancel={() => setIsModalVisible(false)} footer={null} destroyOnHidden width={640}>
        <Form form={form} onFinish={handleCreate} layout="vertical">
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入集群名称' }]}>
                <Input placeholder="例如：gpu-cluster-sh" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="nodes" label="节点数" rules={[{ required: true, message: '请输入节点数' }]}>
                <InputNumber min={1} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="gpus" label="GPU数" rules={[{ required: true, message: '请输入 GPU 数' }]}>
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="gpu_vendors" label="GPU厂商(逗号分隔)">
                <Input placeholder="例如：nvidia,enflame" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="scheduler_types" label="调度器类型(逗号分隔)">
                <Input placeholder="例如：slurm,k8s_native" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="multi_cluster_enabled" label="多集群启用" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="federation_id" label="联邦ID">
                <Input placeholder="例如：fed-001" />
              </Form.Item>
            </Col>
            <Col span={24}>
              <Form.Item name="description" label="描述">
                <Input.TextArea rows={3} placeholder="选填，便于后续追溯" />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Can perm="cluster:write"><Button type="primary" htmlType="submit">创建</Button></Can>
              <Button onClick={() => setIsModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* 集群详情 Drawer */}
      <Modal
        title={`集群详情 - ${detailCluster?.name ?? ''}`}
        open={detailCluster !== null}
        onCancel={() => setDetailCluster(null)}
        footer={null}
        width={720}
      >
        {detailCluster && (
          <div>
            <Row gutter={16}>
              <Col span={12}><p><b>状态：</b><StatusCell status={detailCluster.status} /></p></Col>
              <Col span={12}><p><b>节点数：</b>{detailCluster.nodes ?? '-'}</p></Col>
              <Col span={12}><p><b>GPU数：</b>{detailCluster.gpus ?? '-'}</p></Col>
              <Col span={12}><p><b>CPU数：</b>{detailCluster.cpus ?? '-'}</p></Col>
              <Col span={12}><p><b>内存(GB)：</b>{detailCluster.memory ?? '-'}</p></Col>
              <Col span={12}><p><b>存储(TB)：</b>{detailCluster.storage ?? '-'}</p></Col>
              <Col span={12}><p><b>网络类型：</b>{detailCluster.network_type ?? '-'}</p></Col>
              <Col span={12}><p><b>位置：</b>{detailCluster.location ?? '-'}</p></Col>
              <Col span={12}><p><b>GPU厂商：</b>{detailCluster.gpu_vendors ?? '-'}</p></Col>
              <Col span={12}><p><b>调度器类型：</b>{detailCluster.scheduler_types ?? '-'}</p></Col>
              <Col span={12}><p><b>多集群：</b>{detailCluster.multi_cluster_enabled ? '启用' : '未启用'}</p></Col>
              <Col span={12}><p><b>联邦ID：</b>{detailCluster.federation_id ?? '-'}</p></Col>
            </Row>
            <p><b>描述：</b>{detailCluster.description ?? '-'}</p>

            {/* 分区列表子视图 */}
            <h3 style={{ marginTop: 16, marginBottom: 8 }}>分区列表</h3>
            <ResponsiveTable
              columns={[
                { title: '名称', dataIndex: 'name', key: 'name' },
                { title: '描述', dataIndex: 'description', key: 'description' },
                { title: '优先级', dataIndex: 'priority', key: 'priority', width: 80 },
                { title: '节点数', dataIndex: 'node_count', key: 'node_count', width: 80 },
                { title: 'GPU数', dataIndex: 'gpu_count', key: 'gpu_count', width: 80 },
                { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (s: string) => <StatusCell status={s} /> },
              ]}
              dataSource={clusterPartitions(detailCluster.id)}
              rowKey="id"
              pagination={false}
              size="small"
            />
          </div>
        )}
      </Modal>
    </div>
  );
};

export default ClusterManagement;
