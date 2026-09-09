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
  Tag,
  Popconfirm,
  Drawer,
  Row,
  Col,
} from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import {
  useGetSchedulerIntegrationsQuery,
  useCreateSchedulerIntegrationMutation,
  useUpdateSchedulerIntegrationMutation,
  useDeleteSchedulerIntegrationMutation,
  useGetSchedulerQueuesQuery,
  useGetSchedulerNodesQuery,
  useSyncSchedulerJobsMutation,
  useGetSchedulerHealthQuery,
} from '../store/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { SchedulerIntegration } from '../types';

const SCHEDULER_TYPE_OPTIONS = [
  { label: 'Slurm', value: 'slurm' },
  { label: 'LSF', value: 'lsf' },
  { label: 'SGE', value: 'sge' },
  { label: 'K8s Native', value: 'k8s_native' },
];

const AUTH_TYPE_OPTIONS = [
  { label: '无认证', value: 'none' },
  { label: 'SSH', value: 'ssh' },
  { label: 'Token', value: 'token' },
  { label: 'mTLS', value: 'mTLS' },
];

/** 调度器表单值 */
interface SchedulerFormValues {
  name: string;
  type: 'slurm' | 'lsf' | 'sge' | 'k8s_native';
  endpoint?: string;
  auth_type?: 'none' | 'ssh' | 'token' | 'mTLS';
  auth_config?: string;
  version?: string;
  default_partition?: string;
  default_qos?: string;
  max_nodes?: number;
  max_jobs?: number;
}

const SchedulerManagement: React.FC = () => {
  const { message } = App.useApp();

  // 表单
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [editingScheduler, setEditingScheduler] = useState<SchedulerIntegration | null>(null);
  const [form] = Form.useForm<SchedulerFormValues>();

  // 队列 Drawer
  const [queueDrawerVisible, setQueueDrawerVisible] = useState(false);
  const [queueTarget, setQueueTarget] = useState<SchedulerIntegration | null>(null);

  // 节点 Drawer
  const [nodeDrawerVisible, setNodeDrawerVisible] = useState(false);
  const [nodeTarget, setNodeTarget] = useState<SchedulerIntegration | null>(null);

  // API
  const { data: schedulers, isLoading, error, refetch } = useGetSchedulerIntegrationsQuery();
  const schedulersData = schedulers ?? [];

  const { data: queueData } = useGetSchedulerQueuesQuery(queueTarget?.id ?? 0, {
    skip: !queueTarget,
  });
  const queuesData = queueData ?? [];

  const { data: nodeData } = useGetSchedulerNodesQuery(nodeTarget?.id ?? 0, {
    skip: !nodeTarget,
  });
  const nodesData = nodeData ?? [];

  const [createScheduler] = useCreateSchedulerIntegrationMutation();
  const [updateScheduler] = useUpdateSchedulerIntegrationMutation();
  const [deleteScheduler] = useDeleteSchedulerIntegrationMutation();
  const [syncJobs] = useSyncSchedulerJobsMutation();
  const [healthCheckId, setHealthCheckId] = useState<number | null>(null);
  const { data: healthData } = useGetSchedulerHealthQuery(healthCheckId ?? 0, {
    skip: !healthCheckId,
  });

  // 打开新建
  const handleOpenCreate = useCallback(() => {
    setEditingScheduler(null);
    form.resetFields();
    setIsModalVisible(true);
  }, [form]);

  // 打开编辑
  const handleOpenEdit = useCallback(
    (record: SchedulerIntegration) => {
      setEditingScheduler(record);
      form.setFieldsValue({
        name: record.name,
        type: record.type,
        endpoint: record.endpoint,
        auth_type: record.auth_type,
        auth_config: record.auth_config,
        version: record.version,
        default_partition: record.default_partition,
        default_qos: record.default_qos,
        max_nodes: record.max_nodes,
        max_jobs: record.max_jobs,
      });
      setIsModalVisible(true);
    },
    [form],
  );

  // 提交
  const handleSubmit = useCallback(
    async (values: SchedulerFormValues) => {
      try {
        if (editingScheduler) {
          await updateScheduler({ id: editingScheduler.id, data: values }).unwrap();
          message.success('调度器更新成功');
        } else {
          await createScheduler(values).unwrap();
          message.success('调度器创建成功');
        }
        setIsModalVisible(false);
        form.resetFields();
        refetch();
      } catch {
        message.error('操作失败，请检查必填项后重试');
      }
    },
    [editingScheduler, createScheduler, updateScheduler, form, message, refetch],
  );

  // 删除
  const handleDelete = useCallback(
    async (id: number) => {
      try {
        await deleteScheduler(id).unwrap();
        message.success('调度器删除成功');
        refetch();
      } catch {
        message.error('删除失败，请稍后重试');
      }
    },
    [deleteScheduler, message, refetch],
  );

  // 同步作业
  const handleSyncJobs = useCallback(
    async (id: number) => {
      try {
        await syncJobs(id).unwrap();
        message.success('作业同步已触发');
      } catch {
        message.error('同步失败，请稍后重试');
      }
    },
    [syncJobs, message],
  );

  // 健康检查
  const handleHealthCheck = useCallback((id: number) => {
    setHealthCheckId(id);
    message.info('健康检查已发起，结果将在下方显示');
  }, [message]);

  // 打开队列
  const handleOpenQueues = useCallback((record: SchedulerIntegration) => {
    setQueueTarget(record);
    setQueueDrawerVisible(true);
  }, []);

  // 打开节点
  const handleOpenNodes = useCallback((record: SchedulerIntegration) => {
    setNodeTarget(record);
    setNodeDrawerVisible(true);
  }, []);

  // 列定义
  const columns = useMemo<ColumnsType<SchedulerIntegration>>(
    () => [
      { title: 'ID', dataIndex: 'id', key: 'id', width: 70, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
      { title: '名称', dataIndex: 'name', key: 'name', width: 140 },
      {
        title: '类型',
        dataIndex: 'type',
        key: 'type',
        width: 110,
        render: (v: string) => {
          const opt = SCHEDULER_TYPE_OPTIONS.find((o) => o.value === v);
          return <Tag color="blue">{opt?.label ?? v}</Tag>;
        },
      },
      { title: '端点', dataIndex: 'endpoint', key: 'endpoint', width: 180, ellipsis: true },
      {
        title: '认证方式',
        dataIndex: 'auth_type',
        key: 'auth_type',
        width: 100,
        render: (v: string) => {
          const opt = AUTH_TYPE_OPTIONS.find((o) => o.value === v);
          return opt?.label ?? v ?? '-';
        },
      },
      { title: '版本', dataIndex: 'version', key: 'version', width: 90 },
      {
        title: '状态',
        dataIndex: 'status',
        key: 'status',
        width: 100,
        render: (status: string) => <StatusCell status={status} />,
      },
      { title: '默认分区', dataIndex: 'default_partition', key: 'default_partition', width: 110 },
      { title: '默认QoS', dataIndex: 'default_qos', key: 'default_qos', width: 100 },
      {
        title: '最大节点',
        dataIndex: 'max_nodes',
        key: 'max_nodes',
        width: 90,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: '最大作业',
        dataIndex: 'max_jobs',
        key: 'max_jobs',
        width: 90,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      { title: '最后同步', dataIndex: 'last_sync_at', key: 'last_sync_at', width: 170 },
      {
        title: '操作',
        key: 'action',
        width: 300,
        fixed: 'right',
        render: (_: unknown, record: SchedulerIntegration) => (
          <Space size={4} wrap>
            <Can perm="scheduler:write">
              <Button type="link" size="small" onClick={() => handleOpenEdit(record)}>
                编辑
              </Button>
            </Can>
            <Button type="link" size="small" onClick={() => handleOpenQueues(record)}>
              队列
            </Button>
            <Button type="link" size="small" onClick={() => handleOpenNodes(record)}>
              节点
            </Button>
            <Can perm="scheduler:write">
              <Button type="link" size="small" onClick={() => handleSyncJobs(record.id)}>
                同步作业
              </Button>
            </Can>
            <Button type="link" size="small" onClick={() => handleHealthCheck(record.id)}>
              健康检查
            </Button>
            <Can perm="scheduler:write">
              <Popconfirm
                title="删除该调度器集成？"
                description="集成配置将被移除，且不可恢复。"
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
    [handleOpenEdit, handleOpenQueues, handleOpenNodes, handleSyncJobs, handleHealthCheck, handleDelete],
  );

  const state = renderState({
    isLoading,
    error,
    isEmpty: schedulersData.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 10,
    empty: (
      <EmptyState
        title="暂无调度器集成"
        description="配置 Slurm/LSF/SGE/K8s 调度器集成，实现统一作业管理与状态同步。"
        action={
          <Can perm="scheduler:write">
            <Button type="primary" onClick={handleOpenCreate}>
              新增调度器
            </Button>
          </Can>
        }
      />
    ),
  });

  // 健康检查结果
  const healthResult = useMemo(() => {
    if (!healthCheckId || !healthData) return null;
    const healthy = (healthData as { healthy?: boolean }).healthy;
    return healthy ? (
      <Tag color="success">healthy</Tag>
    ) : (
      <Tag color="error">unhealthy</Tag>
    );
  }, [healthCheckId, healthData]);

  return (
    <div className="mc-page">
      <a href="#scheduler-table" className="mc-skip-link">
        跳转到调度器列表
      </a>
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">调度器集成管理</h1>
          <p className="mc-page-desc">共 {schedulersData.length} 个调度器集成 · 统一纳管 Slurm/LSF/SGE/K8s 调度器</p>
        </div>
        <div className="mc-page-head-extra">
          {healthResult && (
            <span style={{ marginRight: 12 }}>
              最近健康检查：{healthResult}
            </span>
          )}
          <Can perm="scheduler:write">
            <Button type="primary" onClick={handleOpenCreate}>
              新增调度器
            </Button>
          </Can>
        </div>
      </div>

      <Card id="scheduler-table">
        {state ?? (
          <ResponsiveTable
            columns={columns}
            dataSource={schedulersData}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 1800, y: 520 }}
            virtual
          />
        )}
      </Card>

      {/* 新建/编辑 Modal */}
      <Modal
        className="mc-modal-full"
        title={editingScheduler ? '编辑调度器' : '新增调度器'}
        open={isModalVisible}
        onCancel={() => setIsModalVisible(false)}
        footer={null}
        destroyOnHidden
        width={680}
      >
        <Form form={form} onFinish={handleSubmit} layout="vertical">
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入名称' }]}>
                <Input placeholder="例如：slurm-cluster-a" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="type" label="类型" rules={[{ required: true, message: '请选择类型' }]}>
                <Select placeholder="请选择调度器类型" options={SCHEDULER_TYPE_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={16}>
              <Form.Item name="endpoint" label="端点">
                <Input placeholder="例如：slurmctl.example.com:6817" />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="auth_type" label="认证方式">
                <Select placeholder="请选择认证方式" options={AUTH_TYPE_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={24}>
              <Form.Item name="auth_config" label="认证配置 (JSON)">
                <Input.TextArea rows={3} placeholder='例如：{"ssh_key":"/path/to/key","user":"slurm"}' />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="version" label="版本">
                <Input placeholder="例如：23.02" />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="default_partition" label="默认分区">
                <Input placeholder="例如：compute" />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="default_qos" label="默认 QoS">
                <Input placeholder="例如：normal" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="max_nodes" label="最大节点数">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="max_jobs" label="最大作业数">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Can perm="scheduler:write">
                <Button type="primary" htmlType="submit">
                  {editingScheduler ? '保存' : '创建'}
                </Button>
              </Can>
              <Button onClick={() => setIsModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* 队列信息 Drawer */}
      <Drawer
        title={`队列信息 - ${queueTarget?.name ?? ''}`}
        open={queueDrawerVisible}
        onClose={() => setQueueDrawerVisible(false)}
        width={720}
      >
        <ResponsiveTable
          columns={[
            { title: '队列名', dataIndex: 'name', key: 'name', width: 140 },
            {
              title: '状态',
              dataIndex: 'status',
              key: 'status',
              width: 100,
              render: (s: string) => <StatusCell status={s} />,
            },
            { title: '总节点', dataIndex: 'total_nodes', key: 'total_nodes', width: 90 },
            { title: '已分配节点', dataIndex: 'allocated_nodes', key: 'allocated_nodes', width: 110 },
            { title: '总GPU', dataIndex: 'total_gpus', key: 'total_gpus', width: 90 },
            { title: '已分配GPU', dataIndex: 'allocated_gpus', key: 'allocated_gpus', width: 110 },
            { title: '等待作业', dataIndex: 'pending_jobs', key: 'pending_jobs', width: 100 },
            { title: '运行作业', dataIndex: 'running_jobs', key: 'running_jobs', width: 100 },
          ]}
          dataSource={queuesData}
          rowKey="name"
          pagination={false}
          size="small"
          scroll={{ x: 800 }}
        />
      </Drawer>

      {/* 节点信息 Drawer */}
      <Drawer
        title={`节点信息 - ${nodeTarget?.name ?? ''}`}
        open={nodeDrawerVisible}
        onClose={() => setNodeDrawerVisible(false)}
        width={720}
      >
        <ResponsiveTable
          columns={[
            { title: '节点名', dataIndex: 'name', key: 'name', width: 140 },
            {
              title: '状态',
              dataIndex: 'status',
              key: 'status',
              width: 100,
              render: (s: string) => <StatusCell status={s} />,
            },
            { title: '分区', dataIndex: 'partition', key: 'partition', width: 120 },
            { title: 'CPU', dataIndex: 'cpu', key: 'cpu', width: 80 },
            { title: 'GPU', dataIndex: 'gpu', key: 'gpu', width: 80 },
            { title: '内存', dataIndex: 'memory', key: 'memory', width: 100 },
            { title: 'GRES', dataIndex: 'gres', key: 'gres', width: 150, ellipsis: true },
          ]}
          dataSource={nodesData}
          rowKey="name"
          pagination={false}
          size="small"
          scroll={{ x: 700 }}
        />
      </Drawer>
    </div>
  );
};

export default SchedulerManagement;
