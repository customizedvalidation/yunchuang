import { Can } from '../components/Can';
import React, { useCallback, useMemo, useState } from 'react';
import { Card } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import { Button, Space, App, Modal, Form, Input, InputNumber, Popconfirm, Tag, Switch, Row, Col, Select } from 'antd';
import {
  useGetClustersQuery,
  useCreateClusterMutation,
  useUpdateClusterMutation,
  useDeleteClusterMutation,
  useGetPartitionsQuery,
} from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { Cluster } from '../types';

/**
 * 集群行：在领域类型基础上扩展集群管理层 P0 要求的运行时能力字段
 * （高可用 / 多方式访问 / 动态扩容）。这些字段由后端集群详情回显，
 * 前端仅用于展示与创建/编辑表单回写，不改变领域模型。
 */
type ClusterRecord = Cluster & {
  ha_enabled?: boolean;
  scheduler_ha_enabled?: boolean;
  access_methods?: string;
  auto_expand?: boolean;
};

/** 新建/编辑集群表单值 */
interface ClusterFormValues {
  name: string;
  description?: string;
  nodes?: number;
  gpus?: number;
  gpu_vendors?: string;
  scheduler_types?: string;
  multi_cluster_enabled?: boolean;
  federation_id?: string;
  ha_enabled?: boolean;
  scheduler_ha_enabled?: boolean;
  access_methods?: string[];
  auto_expand?: boolean;
}

/** 多方式访问：SSH / VNC / WEB */
const ACCESS_METHOD_OPTIONS = [
  { label: 'SSH', value: 'ssh' },
  { label: 'VNC', value: 'vnc' },
  { label: 'WEB', value: 'web' },
];

const STATUS_FILTER_OPTIONS = [
  { label: '全部状态', value: '' },
  { label: '活跃', value: 'active' },
  { label: '非活跃', value: 'inactive' },
  { label: '维护', value: 'maintenance' },
];

/** 把表单访问方式数组序列化为后端存储的逗号分隔字符串 */
const serializeFormValues = (values: ClusterFormValues): Partial<ClusterRecord> => ({
  ...values,
  access_methods: values.access_methods?.length ? values.access_methods.join(',') : undefined,
});

const ClusterManagement: React.FC = () => {
  const { message } = App.useApp();
  const { data: clusters, isLoading, error, refetch } = useGetClustersQuery(undefined);
  const { data: partitions } = useGetPartitionsQuery({});
  const clustersData = extractArrayData<ClusterRecord>(clusters);
  const partitionsData = partitions ?? [];

  const [createCluster] = useCreateClusterMutation();
  const [updateCluster] = useUpdateClusterMutation();
  const [deleteCluster] = useDeleteClusterMutation();

  const [isModalVisible, setIsModalVisible] = useState(false);
  const [editingCluster, setEditingCluster] = useState<ClusterRecord | null>(null);
  const [detailCluster, setDetailCluster] = useState<ClusterRecord | null>(null);
  const [form] = Form.useForm<ClusterFormValues>();

  // 动态扩容 Modal
  const [scaleModalVisible, setScaleModalVisible] = useState(false);
  const [scaleTarget, setScaleTarget] = useState<ClusterRecord | null>(null);
  const [scaleNodes, setScaleNodes] = useState<number>(0);
  const [scaleGpus, setScaleGpus] = useState<number>(0);

  // 搜索 / 筛选
  const [searchText, setSearchText] = useState('');
  const [statusFilter, setStatusFilter] = useState<string>('');

  // useCallback：稳定回调引用，避免子组件（Popconfirm/Button）无谓重渲染
  const handleCreate = useCallback(
    async (values: ClusterFormValues) => {
      try {
        await createCluster(serializeFormValues(values)).unwrap();
        message.success('集群创建成功');
        setIsModalVisible(false);
        form.resetFields();
        refetch();
      } catch {
        message.error('集群创建失败，请检查必填项后重试');
      }
    },
    [createCluster, form, message, refetch],
  );

  // 打开新建
  const handleOpenCreate = useCallback(() => {
    setEditingCluster(null);
    form.resetFields();
    setIsModalVisible(true);
  }, [form]);

  // 打开编辑：集群生命周期管理 —— 查看 / 修改
  const handleOpenEdit = useCallback(
    (record: ClusterRecord) => {
      setEditingCluster(record);
      form.setFieldsValue({
        name: record.name,
        description: record.description,
        nodes: record.nodes,
        gpus: record.gpus,
        gpu_vendors: record.gpu_vendors,
        scheduler_types: record.scheduler_types,
        multi_cluster_enabled: record.multi_cluster_enabled,
        federation_id: record.federation_id,
        ha_enabled: record.ha_enabled,
        scheduler_ha_enabled: record.scheduler_ha_enabled,
        access_methods: record.access_methods ? record.access_methods.split(',') : [],
        auto_expand: record.auto_expand,
      });
      setIsModalVisible(true);
    },
    [form],
  );

  // 编辑保存
  const handleUpdate = useCallback(
    async (values: ClusterFormValues) => {
      if (!editingCluster) return;
      try {
        await updateCluster({ id: editingCluster.id, ...serializeFormValues(values) }).unwrap();
        message.success('集群更新成功');
        setIsModalVisible(false);
        form.resetFields();
        refetch();
      } catch {
        message.error('集群更新失败，请稍后重试');
      }
    },
    [editingCluster, updateCluster, form, message, refetch],
  );

  const handleDelete = useCallback(
    async (id: number) => {
      try {
        await deleteCluster(id).unwrap();
        message.success('集群删除成功');
        refetch();
      } catch {
        message.error('集群删除失败，请稍后重试');
      }
    },
    [deleteCluster, message, refetch],
  );

  // 动态扩容：根据业务需求自动 / 手动扩容集群资源
  const handleOpenScale = useCallback((record: ClusterRecord) => {
    setScaleTarget(record);
    setScaleNodes(record.nodes ?? 0);
    setScaleGpus(record.gpus ?? 0);
    setScaleModalVisible(true);
  }, []);

  const handleSaveScale = useCallback(async () => {
    if (!scaleTarget) return;
    try {
      await updateCluster({ id: scaleTarget.id, nodes: scaleNodes, gpus: scaleGpus }).unwrap();
      message.success('集群扩容指令已下发');
      setScaleModalVisible(false);
      refetch();
    } catch {
      message.error('扩容失败，请稍后重试');
    }
  }, [scaleTarget, scaleNodes, scaleGpus, updateCluster, message, refetch]);

  // 重建：集群生命周期管理 —— 销毁释放后重新拉起
  const handleRebuild = useCallback(
    async (record: ClusterRecord) => {
      try {
        await updateCluster({ id: record.id, status: 'rebuilding' }).unwrap();
        message.success('集群重建指令已下发');
        refetch();
      } catch {
        message.error('重建失败，请稍后重试');
      }
    },
    [updateCluster, message, refetch],
  );

  // 集群的分区列表
  const clusterPartitions = useCallback(
    (clusterId: number) => partitionsData.filter((p) => p.cluster_id === clusterId),
    [partitionsData],
  );

  // 搜索 + 状态筛选
  const filteredClusters = useMemo(() => {
    const kw = searchText.trim().toLowerCase();
    return clustersData.filter((c) => {
      const matchKw =
        !kw ||
        c.name.toLowerCase().includes(kw) ||
        (c.description ?? '').toLowerCase().includes(kw) ||
        (c.location ?? '').toLowerCase().includes(kw);
      const matchStatus = !statusFilter || c.status === statusFilter;
      return matchKw && matchStatus;
    });
  }, [clustersData, searchText, statusFilter]);

  // 列配置用 useMemo 缓存
  const columns: ColumnsType<ClusterRecord> = useMemo(
    () => [
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
      { title: '描述', dataIndex: 'description', key: 'description', ellipsis: true },
      { title: '状态', dataIndex: 'status', key: 'status', width: 110, render: (status: string) => <StatusCell status={status} /> },
      { title: '节点数', dataIndex: 'nodes', key: 'nodes', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
      { title: 'GPU数', dataIndex: 'gpus', key: 'gpus', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
      {
        title: 'GPU厂商',
        dataIndex: 'gpu_vendors',
        key: 'gpu_vendors',
        width: 150,
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
        width: 150,
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
        render: (v: boolean) => (v ? <Tag color="green">联邦</Tag> : <Tag>未启用</Tag>),
      },
      {
        title: '高可用',
        key: 'ha',
        width: 120,
        render: (_: unknown, record: ClusterRecord) => {
          const ha = record.ha_enabled || record.scheduler_ha_enabled;
          return ha ? <Tag color="success">HA</Tag> : <Tag>未启用</Tag>;
        },
      },
      {
        title: '访问方式',
        dataIndex: 'access_methods',
        key: 'access_methods',
        width: 150,
        render: (v: string) => {
          if (!v) return '-';
          return v.split(',').map((m) => (
            <Tag key={m} color="geekblue">{m.trim().toUpperCase()}</Tag>
          ));
        },
      },
      {
        title: '操作',
        key: 'action',
        width: 240,
        fixed: 'right',
        render: (_: unknown, record: ClusterRecord) => (
          <Space size={4} wrap>
            <Can perm="cluster:write">
              <Button type="link" size="small" onClick={() => handleOpenEdit(record)}>
                编辑
              </Button>
            </Can>
            <Can perm="cluster:write">
              <Button type="link" size="small" onClick={() => handleOpenScale(record)}>
                扩容
              </Button>
            </Can>
            <Can perm="cluster:write">
              <Popconfirm
                title="重建该集群？"
                description="集群将重新拉起，运行中的作业会被迁移或排队。"
                okText="确认重建"
                cancelText="返回"
                onConfirm={() => handleRebuild(record)}
              >
                <Button type="link" size="small">
                  重建
                </Button>
              </Popconfirm>
            </Can>
            <Can perm="cluster:write">
              <Popconfirm
                title="删除该集群？"
                description="集群下的节点与配额将一并移除，且不可恢复。"
                okText="确认删除"
                cancelText="返回"
                okButtonProps={{ danger: true }}
                onConfirm={() => handleDelete(record.id)}
              >
                <Button type="link" danger size="small">
                  删除
                </Button>
              </Popconfirm>
            </Can>
          </Space>
        ),
      },
    ],
    [handleOpenEdit, handleOpenScale, handleRebuild, handleDelete],
  );

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
        action={
          <Can perm="cluster:write">
            <Button type="primary" onClick={handleOpenCreate}>
              创建集群
            </Button>
          </Can>
        }
      />
    ),
  });

  return (
    <div className="mc-page">
      <a href="#cluster-table" className="mc-skip-link">
        跳转到集群列表
      </a>
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">集群管理</h1>
          <p className="mc-page-desc">共 {clustersData.length} 个集群 · 多集群统一调度 / 高可用 / SSH·VNC·WEB 多方式访问</p>
        </div>
        <div className="mc-page-head-extra">
          <Can perm="cluster:write">
            <Button type="primary" onClick={handleOpenCreate}>
              创建集群
            </Button>
          </Can>
        </div>
      </div>

      <Card id="cluster-table">
        {/* 搜索 + 状态筛选工具栏 */}
        <Row gutter={12} style={{ marginBottom: 16 }}>
          <Col flex="auto">
            <Input
              allowClear
              placeholder="搜索集群名称 / 描述 / 位置"
              value={searchText}
              onChange={(e) => setSearchText(e.target.value)}
            />
          </Col>
          <Col>
            <Select
              value={statusFilter}
              onChange={setStatusFilter}
              options={STATUS_FILTER_OPTIONS}
              style={{ width: 140 }}
            />
          </Col>
        </Row>

        {state ?? (
          <ResponsiveTable
            columns={columns}
            dataSource={filteredClusters}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 1500 }}
          />
        )}
      </Card>

      {/* 创建 / 编辑集群 Modal */}
      <Modal
        className="mc-modal-full"
        title={editingCluster ? '编辑集群' : '创建集群'}
        open={isModalVisible}
        onCancel={() => setIsModalVisible(false)}
        footer={null}
        destroyOnHidden
        width={680}
      >
        <Form
          form={form}
          onFinish={editingCluster ? handleUpdate : handleCreate}
          layout="vertical"
        >
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
                <Input placeholder="例如：slurm,lsf" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="federation_id" label="联邦ID（多集群）">
                <Input placeholder="例如：fed-001" />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="multi_cluster_enabled" label="多集群联邦" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="auto_expand" label="动态扩容" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
            <Col span={24}>
              <Form.Item name="access_methods" label="多方式访问（SSH / VNC / WEB）">
                <Select
                  mode="multiple"
                  allowClear
                  placeholder="选择支持的访问方式"
                  options={ACCESS_METHOD_OPTIONS}
                />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="ha_enabled" label="管理平台高可用(HA)" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="scheduler_ha_enabled" label="核心调度节点HA" valuePropName="checked">
                <Switch />
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
              <Can perm="cluster:write">
                <Button type="primary" htmlType="submit">
                  {editingCluster ? '保存' : '创建'}
                </Button>
              </Can>
              <Button onClick={() => setIsModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* 动态扩容 Modal */}
      <Modal
        title={`动态扩容 - ${scaleTarget?.name ?? ''}`}
        open={scaleModalVisible}
        onCancel={() => setScaleModalVisible(false)}
        onOk={handleSaveScale}
        okText="下发扩容"
        cancelText="取消"
      >
        <p style={{ marginBottom: 16, color: 'var(--mc-text-3)', fontSize: 12 }}>
          调整后集群将按新的节点 / GPU 容量重新调度，现有作业不受影响。
        </p>
        <Row gutter={16}>
          <Col span={12}>
            <p><b>节点数</b></p>
            <InputNumber min={1} value={scaleNodes} onChange={(v) => setScaleNodes(v ?? 1)} style={{ width: '100%' }} />
          </Col>
          <Col span={12}>
            <p><b>GPU数</b></p>
            <InputNumber min={0} value={scaleGpus} onChange={(v) => setScaleGpus(v ?? 0)} style={{ width: '100%' }} />
          </Col>
        </Row>
      </Modal>

      {/* 集群详情 Modal */}
      <Modal
        title={`集群详情 - ${detailCluster?.name ?? ''}`}
        open={detailCluster !== null}
        onCancel={() => setDetailCluster(null)}
        footer={null}
        width={760}
      >
        {detailCluster && (
          <div>
            <Row gutter={16}>
              <Col span={12}>
                <p><b>状态：</b><StatusCell status={detailCluster.status} /></p>
              </Col>
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

            {/* 高可用 */}
            <h3 style={{ marginTop: 16, marginBottom: 8 }}>高可用</h3>
            <Space wrap>
              <Tag color={detailCluster.ha_enabled ? 'success' : 'default'}>
                管理平台HA {detailCluster.ha_enabled ? '开启' : '关闭'}
              </Tag>
              <Tag color={detailCluster.scheduler_ha_enabled ? 'success' : 'default'}>
                调度节点HA {detailCluster.scheduler_ha_enabled ? '开启' : '关闭'}
              </Tag>
              <Tag color={detailCluster.auto_expand ? 'blue' : 'default'}>
                动态扩容 {detailCluster.auto_expand ? '开启' : '关闭'}
              </Tag>
            </Space>

            {/* 多方式访问 */}
            <h3 style={{ marginTop: 16, marginBottom: 8 }}>访问方式</h3>
            <Space wrap>
              {(detailCluster.access_methods
                ? detailCluster.access_methods.split(',')
                : ['web']
              ).map((m) => (
                <Tag key={m} color="geekblue">
                  {m.trim().toUpperCase()}
                </Tag>
              ))}
            </Space>

            <p style={{ marginTop: 16 }}><b>描述：</b>{detailCluster.description ?? '-'}</p>

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
