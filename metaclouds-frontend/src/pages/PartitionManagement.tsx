import { Can } from '../components/Can';
import React, { useCallback, useMemo, useState } from 'react';
import {
  Card,
  Button,
  Space,
  App,
  Modal,
  Form,
  Input,
  Select,
  InputNumber,
  Switch,
  Slider,
  Tag,
  Popconfirm,
  Drawer,
  Row,
  Col,
} from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import {
  useGetPartitionsQuery,
  useCreatePartitionMutation,
  useUpdatePartitionMutation,
  useDeletePartitionMutation,
  useUpdatePartitionPriorityMutation,
  useUpdatePartitionMaxRuntimeMutation,
  useGetPartitionPermissionsQuery,
  useSetPartitionPermissionMutation,
  useRemovePartitionPermissionMutation,
  useGetClustersQuery,
} from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { Partition, PartitionPermission, GPUVendor } from '../types';

const SCHEDULER_OPTIONS = [
  { label: 'K8s Native', value: 'k8s_native' },
  { label: 'Slurm', value: 'slurm' },
  { label: 'LSF', value: 'lsf' },
  { label: 'SGE', value: 'sge' },
];

const VENDOR_OPTIONS: { label: string; value: GPUVendor }[] = [
  { label: 'NVIDIA', value: 'nvidia' },
  { label: '燧原', value: 'enflame' },
  { label: '摩尔线程', value: 'moore_threads' },
  { label: '国产 X', value: 'domestic_x' },
];

/** 分区表单值 */
interface PartitionFormValues {
  name: string;
  cluster_id: number;
  description?: string;
  priority?: number;
  max_runtime_minutes?: number;
  node_count?: number;
  cpu_limit?: number;
  gpu_count?: number;
  gpu_vendor?: GPUVendor;
  scheduler_type?: 'k8s_native' | 'slurm' | 'lsf' | 'sge';
  status?: string;
  allow_sharing?: boolean;
  default_qos?: string;
  nodes?: string;
  taints?: string;
  labels?: string;
}

const PartitionManagement: React.FC = () => {
  const { message } = App.useApp();

  // 表单状态
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [editingPartition, setEditingPartition] = useState<Partition | null>(null);
  const [form] = Form.useForm<PartitionFormValues>();

  // 优先级调整 Modal
  const [priorityModalVisible, setPriorityModalVisible] = useState(false);
  const [priorityTarget, setPriorityTarget] = useState<Partition | null>(null);
  const [priorityValue, setPriorityValue] = useState(50);

  // 最大运行时长 Modal
  const [runtimeModalVisible, setRuntimeModalVisible] = useState(false);
  const [runtimeTarget, setRuntimeTarget] = useState<Partition | null>(null);
  const [runtimeValue, setRuntimeValue] = useState(0);

  // 权限管理 Drawer
  const [permDrawerVisible, setPermDrawerVisible] = useState(false);
  const [permTarget, setPermTarget] = useState<Partition | null>(null);
  const [permForm] = Form.useForm();

  // 搜索 / 筛选
  const [searchText, setSearchText] = useState('');
  const [clusterFilter, setClusterFilter] = useState<number | undefined>(undefined);
  const [statusFilter, setStatusFilter] = useState<string>('');

  // API
  const { data: partitions, isLoading, error, refetch } = useGetPartitionsQuery({});
  const { data: clusters } = useGetClustersQuery(undefined);
  const partitionsData = partitions ?? [];
  const clustersData = extractArrayData(clusters);

  const { data: permData, refetch: refetchPerms } = useGetPartitionPermissionsQuery(permTarget?.id ?? 0, {
    skip: !permTarget,
  });
  const permissionsData = permData ?? [];

  const [createPartition] = useCreatePartitionMutation();
  const [updatePartition] = useUpdatePartitionMutation();
  const [deletePartition] = useDeletePartitionMutation();
  const [updatePriority] = useUpdatePartitionPriorityMutation();
  const [updateMaxRuntime] = useUpdatePartitionMaxRuntimeMutation();
  const [setPermission] = useSetPartitionPermissionMutation();
  const [removePermission] = useRemovePartitionPermissionMutation();

  const clusterName = useCallback(
    (id?: number) => clustersData.find((c: { id: number }) => c.id === id)?.name ?? '-',
    [clustersData],
  );

  // 搜索 + 集群 / 状态筛选
  const filteredPartitions = useMemo(() => {
    const kw = searchText.trim().toLowerCase();
    return partitionsData.filter((p) => {
      const matchKw = !kw || p.name.toLowerCase().includes(kw) || (p.description ?? '').toLowerCase().includes(kw);
      const matchCluster = clusterFilter == null || p.cluster_id === clusterFilter;
      const matchStatus = !statusFilter || p.status === statusFilter;
      return matchKw && matchCluster && matchStatus;
    });
  }, [partitionsData, searchText, clusterFilter, statusFilter]);

  // 打开新建
  const handleOpenCreate = useCallback(() => {
    setEditingPartition(null);
    form.resetFields();
    setIsModalVisible(true);
  }, [form]);

  // 打开编辑
  const handleOpenEdit = useCallback(
    (record: Partition) => {
      setEditingPartition(record);
      form.setFieldsValue({
        name: record.name,
        cluster_id: record.cluster_id,
        description: record.description,
        priority: record.priority,
        max_runtime_minutes: record.max_runtime_minutes,
        node_count: record.node_count,
        cpu_limit: record.cpu_limit,
        gpu_count: record.gpu_count,
        gpu_vendor: record.gpu_vendor,
        scheduler_type: record.scheduler_type,
        status: record.status,
        allow_sharing: record.allow_sharing,
        default_qos: record.default_qos,
        nodes: record.nodes,
        taints: record.taints,
        labels: record.labels,
      });
      setIsModalVisible(true);
    },
    [form],
  );

  // 提交
  const handleSubmit = useCallback(
    async (values: PartitionFormValues) => {
      try {
        if (editingPartition) {
          await updatePartition({ id: editingPartition.id, data: values as Partial<Partition> }).unwrap();
          message.success('分区更新成功');
        } else {
          await createPartition(values as Partial<Partition>).unwrap();
          message.success('分区创建成功');
        }
        setIsModalVisible(false);
        form.resetFields();
        refetch();
      } catch {
        message.error('操作失败，请检查必填项后重试');
      }
    },
    [editingPartition, createPartition, updatePartition, form, message, refetch],
  );

  // 删除
  const handleDelete = useCallback(
    async (id: number) => {
      try {
        await deletePartition(id).unwrap();
        message.success('分区删除成功');
        refetch();
      } catch {
        message.error('删除失败，请稍后重试');
      }
    },
    [deletePartition, message, refetch],
  );

  // 优先级调整
  const handleOpenPriority = useCallback((record: Partition) => {
    setPriorityTarget(record);
    setPriorityValue(record.priority ?? 50);
    setPriorityModalVisible(true);
  }, []);

  const handleSavePriority = useCallback(async () => {
    if (!priorityTarget) return;
    try {
      await updatePriority({ id: priorityTarget.id, priority: priorityValue }).unwrap();
      message.success('优先级更新成功');
      setPriorityModalVisible(false);
      refetch();
    } catch {
      message.error('操作失败，请稍后重试');
    }
  }, [priorityTarget, priorityValue, updatePriority, message, refetch]);

  // 最大运行时长调整
  const handleOpenRuntime = useCallback((record: Partition) => {
    setRuntimeTarget(record);
    setRuntimeValue(record.max_runtime_minutes ?? 0);
    setRuntimeModalVisible(true);
  }, []);

  const handleSaveRuntime = useCallback(async () => {
    if (!runtimeTarget) return;
    try {
      await updateMaxRuntime({ id: runtimeTarget.id, max_runtime_minutes: runtimeValue }).unwrap();
      message.success('最大运行时长更新成功');
      setRuntimeModalVisible(false);
      refetch();
    } catch {
      message.error('操作失败，请稍后重试');
    }
  }, [runtimeTarget, runtimeValue, updateMaxRuntime, message, refetch]);

  // 权限管理
  const handleOpenPerm = useCallback((record: Partition) => {
    setPermTarget(record);
    setPermDrawerVisible(true);
    permForm.resetFields();
  }, [permForm]);

  const handleAddPermission = useCallback(
    async (values: { principal_type: 'user' | 'group'; principal_id: number; access_level: 'view' | 'submit' | 'admin' }) => {
      if (!permTarget) return;
      try {
        await setPermission({ partition_id: permTarget.id, data: values }).unwrap();
        message.success('权限添加成功');
        permForm.resetFields();
        refetchPerms();
      } catch {
        message.error('添加失败，请稍后重试');
      }
    },
    [permTarget, setPermission, permForm, message, refetchPerms],
  );

  const handleRemovePermission = useCallback(
    async (id: number) => {
      try {
        await removePermission(id).unwrap();
        message.success('权限已移除');
        refetchPerms();
      } catch {
        message.error('移除失败，请稍后重试');
      }
    },
    [removePermission, message, refetchPerms],
  );

  // 列定义
  const columns = useMemo<ColumnsType<Partition>>(
    () => [
      { title: 'ID', dataIndex: 'id', key: 'id', width: 70, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
      { title: '名称', dataIndex: 'name', key: 'name', width: 140 },
      {
        title: '集群',
        dataIndex: 'cluster_id',
        key: 'cluster_id',
        width: 120,
        render: (v: number) => clusterName(v),
      },
      { title: '描述', dataIndex: 'description', key: 'description', ellipsis: true },
      {
        title: '优先级',
        dataIndex: 'priority',
        key: 'priority',
        width: 80,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: '最大运行(分钟)',
        dataIndex: 'max_runtime_minutes',
        key: 'max_runtime_minutes',
        width: 120,
        render: (v: number) => (v === 0 || v == null ? '无限' : <span className="mc-num">{v}</span>),
      },
      {
        title: '节点数',
        dataIndex: 'node_count',
        key: 'node_count',
        width: 80,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: 'CPU限制',
        dataIndex: 'cpu_limit',
        key: 'cpu_limit',
        width: 90,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: 'GPU数',
        dataIndex: 'gpu_count',
        key: 'gpu_count',
        width: 80,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: 'GPU厂商',
        dataIndex: 'gpu_vendor',
        key: 'gpu_vendor',
        width: 100,
        render: (v: GPUVendor) => (v ? <Tag>{v}</Tag> : '-'),
      },
      {
        title: '调度器',
        dataIndex: 'scheduler_type',
        key: 'scheduler_type',
        width: 110,
        render: (v: string) => {
          const opt = SCHEDULER_OPTIONS.find((o) => o.value === v);
          return opt ? <Tag color="blue">{opt.label}</Tag> : '-';
        },
      },
      {
        title: '状态',
        dataIndex: 'status',
        key: 'status',
        width: 100,
        render: (status: string) => <StatusCell status={status} />,
      },
      {
        title: '允许共享',
        dataIndex: 'allow_sharing',
        key: 'allow_sharing',
        width: 90,
        render: (v: boolean) => (v ? <Tag color="green">是</Tag> : <Tag>否</Tag>),
      },
      {
        title: '操作',
        key: 'action',
        width: 280,
        fixed: 'right',
        render: (_: unknown, record: Partition) => (
          <Space size={4} wrap>
            <Can perm="partition:write">
              <Button type="link" size="small" onClick={() => handleOpenEdit(record)}>
                编辑
              </Button>
            </Can>
            <Can perm="partition:write">
              <Button type="link" size="small" onClick={() => handleOpenPriority(record)}>
                优先级
              </Button>
            </Can>
            <Can perm="partition:write">
              <Button type="link" size="small" onClick={() => handleOpenRuntime(record)}>
                运行时长
              </Button>
            </Can>
            <Can perm="partition:write">
              <Button type="link" size="small" onClick={() => handleOpenPerm(record)}>
                权限
              </Button>
            </Can>
            <Can perm="partition:write">
              <Popconfirm
                title="删除该分区？"
                description="分区配置与权限将一并移除，且不可恢复。"
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
    [clusterName, handleOpenEdit, handleOpenPriority, handleOpenRuntime, handleOpenPerm, handleDelete],
  );

  const state = renderState({
    isLoading,
    error,
    isEmpty: partitionsData.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 10,
    empty: (
      <EmptyState
        title="暂无分区"
        description="创建分区以划分计算资源边界、配置优先级与运行时长限制。"
        action={
          <Can perm="partition:write">
            <Button type="primary" onClick={handleOpenCreate}>
              创建分区
            </Button>
          </Can>
        }
      />
    ),
  });

  return (
    <div className="mc-page">
      <a href="#partition-table" className="mc-skip-link">
        跳转到分区列表
      </a>
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">分区管理</h1>
          <p className="mc-page-desc">共 {partitionsData.length} 个分区 · 管理计算资源边界、优先级与权限</p>
        </div>
        <div className="mc-page-head-extra">
          <Can perm="partition:write">
            <Button type="primary" onClick={handleOpenCreate}>
              创建分区
            </Button>
          </Can>
        </div>
      </div>

      <Card id="partition-table">
        {/* 搜索 + 集群 / 状态筛选工具栏 */}
        <Row gutter={12} style={{ marginBottom: 16 }}>
          <Col flex="auto">
            <Input
              allowClear
              placeholder="搜索分区名称 / 描述"
              value={searchText}
              onChange={(e) => setSearchText(e.target.value)}
            />
          </Col>
          <Col>
            <Select
              allowClear
              placeholder="集群"
              style={{ width: 160 }}
              value={clusterFilter}
              onChange={(v) => setClusterFilter(v)}
              options={clustersData.map((c: { id: number; name: string }) => ({ label: c.name, value: c.id }))}
            />
          </Col>
          <Col>
            <Select
              allowClear
              placeholder="状态"
              style={{ width: 130 }}
              value={statusFilter || undefined}
              onChange={(v) => setStatusFilter(v ?? '')}
              options={[
                { label: '活跃', value: 'active' },
                { label: '未启用', value: 'inactive' },
                { label: '维护', value: 'maintenance' },
                { label: '已排空', value: 'drained' },
              ]}
            />
          </Col>
        </Row>
        {state ?? (
          <ResponsiveTable
            columns={columns}
            dataSource={filteredPartitions}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 1700, y: 520 }}
            virtual
          />
        )}
      </Card>

      {/* 新建/编辑 Modal */}
      <Modal
        className="mc-modal-full"
        title={editingPartition ? '编辑分区' : '创建分区'}
        open={isModalVisible}
        onCancel={() => setIsModalVisible(false)}
        footer={null}
        destroyOnHidden
        width={720}
      >
        <Form form={form} onFinish={handleSubmit} layout="vertical">
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入分区名称' }]}>
                <Input placeholder="例如：gpu-high" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="cluster_id" label="集群" rules={[{ required: true, message: '请选择集群' }]}>
                <Select
                  placeholder="请选择集群"
                  options={clustersData.map((c: { id: number; name: string }) => ({ label: c.name, value: c.id }))}
                />
              </Form.Item>
            </Col>
            <Col span={24}>
              <Form.Item name="description" label="描述">
                <Input.TextArea rows={2} placeholder="选填" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="priority" label="优先级 (0-100)">
                <Slider min={0} max={100} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="max_runtime_minutes" label="最大运行时长(分钟，0=无限)">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="node_count" label="节点数">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="cpu_limit" label="CPU 限制">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="gpu_count" label="GPU 数">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="gpu_vendor" label="GPU 厂商">
                <Select placeholder="请选择厂商" allowClear options={VENDOR_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="scheduler_type" label="调度器类型">
                <Select placeholder="请选择调度器" allowClear options={SCHEDULER_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="status" label="状态">
                <Select
                  placeholder="请选择状态"
                  options={[
                    { label: '活跃', value: 'active' },
                    { label: '非活跃', value: 'inactive' },
                    { label: '维护', value: 'maintenance' },
                    { label: '已排空', value: 'drained' },
                  ]}
                />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="default_qos" label="默认 QoS">
                <Input placeholder="例如：normal" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="allow_sharing" label="允许资源共享" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
            <Col span={24}>
              <Form.Item name="nodes" label="节点列表 (JSON)">
                <Input.TextArea rows={2} placeholder='例如：["node-01","node-02"]' />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="taints" label="Taints (JSON)">
                <Input.TextArea rows={2} placeholder='例如：[{"key":"gpu","value":"nvidia","effect":"NoSchedule"}]' />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="labels" label="Labels (JSON)">
                <Input.TextArea rows={2} placeholder='例如：{"gpu-type":"a100"}' />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Can perm="partition:write">
                <Button type="primary" htmlType="submit">
                  {editingPartition ? '保存' : '创建'}
                </Button>
              </Can>
              <Button onClick={() => setIsModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* 优先级调整 Modal */}
      <Modal
        title="调整分区优先级"
        open={priorityModalVisible}
        onCancel={() => setPriorityModalVisible(false)}
        onOk={handleSavePriority}
        okText="保存"
        cancelText="取消"
      >
        <p style={{ marginBottom: 16 }}>
          分区：<b>{priorityTarget?.name}</b>
        </p>
        <Slider min={0} max={100} value={priorityValue} onChange={setPriorityValue} />
        <div style={{ textAlign: 'center', marginTop: 8 }}>
          当前值：<span className="mc-num">{priorityValue}</span>
        </div>
      </Modal>

      {/* 最大运行时长 Modal */}
      <Modal
        title="修改最大运行时长"
        open={runtimeModalVisible}
        onCancel={() => setRuntimeModalVisible(false)}
        onOk={handleSaveRuntime}
        okText="保存"
        cancelText="取消"
      >
        <p style={{ marginBottom: 16 }}>
          分区：<b>{runtimeTarget?.name}</b>
        </p>
        <InputNumber
          min={0}
          value={runtimeValue}
          onChange={(v) => setRuntimeValue(v ?? 0)}
          style={{ width: '100%' }}
          addonAfter="分钟"
        />
        <p style={{ marginTop: 8, color: 'var(--mc-text-3)', fontSize: 12 }}>0 表示不限制运行时长</p>
      </Modal>

      {/* 权限管理 Drawer */}
      <Drawer
        title={`分区权限管理 - ${permTarget?.name ?? ''}`}
        open={permDrawerVisible}
        onClose={() => setPermDrawerVisible(false)}
        width={560}
      >
        <Form form={permForm} onFinish={handleAddPermission} layout="inline" style={{ marginBottom: 16 }}>
          <Form.Item name="principal_type" label="主体类型" rules={[{ required: true }]} initialValue="user">
            <Select style={{ width: 100 }} options={[{ label: '用户', value: 'user' }, { label: '用户组', value: 'group' }]} />
          </Form.Item>
          <Form.Item name="principal_id" label="主体ID" rules={[{ required: true }]}>
            <InputNumber min={1} style={{ width: 100 }} />
          </Form.Item>
          <Form.Item name="access_level" label="访问级别" rules={[{ required: true }]} initialValue="view">
            <Select
              style={{ width: 120 }}
              options={[
                { label: '查看', value: 'view' },
                { label: '提交', value: 'submit' },
                { label: '管理', value: 'admin' },
              ]}
            />
          </Form.Item>
          <Form.Item>
            <Can perm="partition:write">
              <Button type="primary" htmlType="submit">
                添加
              </Button>
            </Can>
          </Form.Item>
        </Form>

        <ResponsiveTable
          columns={[
            { title: 'ID', dataIndex: 'id', key: 'id', width: 70 },
            {
              title: '主体类型',
              dataIndex: 'principal_type',
              key: 'principal_type',
              width: 100,
              render: (v: string) => <Tag>{v === 'user' ? '用户' : '用户组'}</Tag>,
            },
            { title: '主体ID', dataIndex: 'principal_id', key: 'principal_id', width: 90 },
            {
              title: '访问级别',
              dataIndex: 'access_level',
              key: 'access_level',
              width: 100,
              render: (v: string) => {
                const colorMap: Record<string, string> = { view: 'default', submit: 'blue', admin: 'red' };
                return <Tag color={colorMap[v]}>{v}</Tag>;
              },
            },
            {
              title: '操作',
              key: 'action',
              width: 80,
              render: (_: unknown, record: PartitionPermission) => (
                <Can perm="partition:write">
                  <Popconfirm
                    title="移除此权限？"
                    onConfirm={() => handleRemovePermission(record.id)}
                    okText="确认"
                    cancelText="取消"
                  >
                    <Button type="link" danger size="small">
                      移除
                    </Button>
                  </Popconfirm>
                </Can>
              ),
            },
          ]}
          dataSource={permissionsData}
          rowKey="id"
          pagination={false}
          size="small"
        />
      </Drawer>
    </div>
  );
};

export default PartitionManagement;
