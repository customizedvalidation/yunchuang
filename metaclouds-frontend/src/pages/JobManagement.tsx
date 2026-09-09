import { Can } from '../components/Can';
import React, { memo, useCallback, useMemo, useState } from 'react';
import { Card } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import { Button, Space, App, Modal, Form, Input, Select, InputNumber, Tabs, Popconfirm, Switch, Drawer, Row, Col, Tag } from 'antd';
import { useGetJobsQuery, useCreateJobMutation, useCancelJobMutation, useGetPartitionsQuery } from '../store/api';
import { extractArrayData } from '../utils/api';
import { useLocation, useNavigate } from 'react-router-dom';
import { renderState, EmptyState } from '../components/States';
import { statusColor, statusText } from '../theme/tokens';
import type { Job, JobType, GPUVendor } from '../types';

/** 新建作业表单值 */
interface JobFormValues {
  name: string;
  type: JobType;
  gpus: number;
  description?: string;
  partition_id?: number;
  scheduler_type?: 'k8s_native' | 'slurm' | 'lsf' | 'sge';
  gpu_fraction?: number;
  gpu_memory_gb?: number;
  gpu_vendor?: GPUVendor;
  qos?: string;
  nodes_requested?: number;
  node_selector?: string;
  affinity?: string;
  tolerations?: string;
  elastic_enabled?: boolean;
  min_gpus?: number;
  max_gpus?: number;
  scaling_policy?: string;
  checkpoint_enabled?: boolean;
  checkpoint_interval_minutes?: number;
  max_retries?: number;
  fault_tolerance_level?: 'none' | 'node' | 'rack' | 'switch';
  topology_affinity?: 'node' | 'rack' | 'switch' | 'cluster';
  topology_anti_affinity?: boolean;
  network_requirement?: 'rdma' | 'ethernet' | 'any';
}

const JOB_TABS = [
  { key: '/job/list', label: '作业列表' },
  { key: '/job/queue', label: '任务队列' },
  { key: '/job/history', label: '历史记录' },
];

const SCHEDULER_TYPE_OPTIONS = [
  { label: 'K8s Native', value: 'k8s_native' },
  { label: 'Slurm', value: 'slurm' },
  { label: 'LSF', value: 'lsf' },
  { label: 'SGE', value: 'sge' },
];

const GPU_VENDOR_OPTIONS: { label: string; value: GPUVendor }[] = [
  { label: 'NVIDIA', value: 'nvidia' },
  { label: '燧原', value: 'enflame' },
  { label: '摩尔线程', value: 'moore_threads' },
  { label: '国产 X', value: 'domestic_x' },
];

const GPU_FRACTION_OPTIONS = [
  { label: '1 GPU', value: 1 },
  { label: '1/2 GPU', value: 0.5 },
  { label: '1/4 GPU', value: 0.25 },
];

const FAULT_TOLERANCE_OPTIONS = [
  { label: '无', value: 'none' },
  { label: '节点级', value: 'node' },
  { label: '机架级', value: 'rack' },
  { label: '交换机级', value: 'switch' },
];

const TOPOLOGY_AFFINITY_OPTIONS = [
  { label: '节点', value: 'node' },
  { label: '机架', value: 'rack' },
  { label: '交换机', value: 'switch' },
  { label: '集群', value: 'cluster' },
];

const NETWORK_REQUIREMENT_OPTIONS = [
  { label: 'RDMA', value: 'rdma' },
  { label: '以太网', value: 'ethernet' },
  { label: '任意', value: 'any' },
];

/** 状态 = 色点 + 文字，禁止仅用颜色表意。React.memo 避免父组件重渲染时无谓重绘。 */
const StatusCell: React.FC<{ status: string }> = memo(({ status }) => {
  const cls = ['running', 'pending', 'completed', 'failed'].includes(status) ? status : 'idle';
  return (
    <span className={`mc-status ${cls}`}>
      <i className="mc-status-dot" style={{ background: statusColor[status] }} />
      {statusText[status] ?? status}
    </span>
  );
});

const JobManagement: React.FC = () => {
  const { message } = App.useApp();
  const location = useLocation();
  const navigate = useNavigate();
  // 子路由驱动当前激活的页签；父路由 /job 默认落到"作业列表"
  const activeKey = location.pathname.startsWith('/job/') ? location.pathname : '/job/list';

  const { data: jobs, isLoading, error: jobsError, refetch } = useGetJobsQuery(undefined);
  const { data: partitions } = useGetPartitionsQuery({});
  const jobsData = extractArrayData<Job>(jobs);
  const partitionsData = partitions ?? [];
  const [createJob] = useCreateJobMutation();
  const [cancelJob] = useCancelJobMutation();
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [detailJob, setDetailJob] = useState<Job | null>(null);
  const [form] = Form.useForm<JobFormValues>();

  const partitionName = useCallback(
    (id?: number) => partitionsData.find((p) => p.id === id)?.name ?? '-',
    [partitionsData],
  );

  // useCallback：稳定回调引用，避免传给子组件（Popconfirm/Button）时触发无谓重渲染
  const handleCreate = useCallback(async (values: JobFormValues) => {
    try {
      await createJob(values).unwrap();
      message.success('作业创建成功');
      setIsModalVisible(false);
      form.resetFields();
      refetch();
    } catch {
      message.error('作业创建失败，请检查必填项后重试');
    }
  }, [createJob, form, message, refetch]);

  const handleCancel = useCallback(async (id: number) => {
    try {
      await cancelJob(id).unwrap();
      message.success('作业已取消');
      refetch();
    } catch {
      message.error('作业取消失败，请稍后重试');
    }
  }, [cancelJob, message, refetch]);

  // 基础列定义用 useMemo 缓存：列配置不随渲染变化，避免每次渲染重建导致 Table 深比较失效
  const baseColumns = useMemo<ColumnsType<Job>>(() => [
    {
      title: 'ID',
      dataIndex: 'id',
      key: 'id',
      width: 90,
      render: (v: React.ReactNode) => <span className="mc-mono">{v}</span>,
    },
    {
      title: '名称',
      dataIndex: 'name',
      key: 'name',
      render: (v: string, record) => (
        <Button type="link" size="small" style={{ padding: 0 }} onClick={() => setDetailJob(record)}>
          {v}
        </Button>
      ),
    },
    { title: '类型', dataIndex: 'type', key: 'type' },
    {
      title: '状态',
      dataIndex: 'status',
      key: 'status',
      width: 110,
      render: (status: string) => <StatusCell status={status} />,
    },
    {
      title: 'GPU',
      dataIndex: 'gpus',
      key: 'gpus',
      width: 80,
      render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
    },
    {
      title: 'GPU分数',
      dataIndex: 'gpu_fraction',
      key: 'gpu_fraction',
      width: 90,
      render: (v: number) => (v != null ? <Tag color="blue">{v}</Tag> : '-'),
    },
    {
      title: '分区',
      dataIndex: 'partition_id',
      key: 'partition_id',
      width: 110,
      render: (v: number) => partitionName(v),
    },
    {
      title: '调度器',
      dataIndex: 'scheduler_type',
      key: 'scheduler_type',
      width: 110,
      render: (v: string) => {
        const opt = SCHEDULER_TYPE_OPTIONS.find((o) => o.value === v);
        return opt ? <Tag>{opt.label}</Tag> : '-';
      },
    },
    {
      title: '进度',
      dataIndex: 'progress',
      key: 'progress',
      width: 90,
      render: (progress: number) => <span className="mc-num">{progress}%</span>,
    },
    { title: '创建时间', dataIndex: 'created_at', key: 'created_at' },
    {
      title: '操作',
      key: 'action',
      width: 90,
      render: (_: unknown, record: Job) => (
        <Space>
          {record.status === 'running' || record.status === 'pending' ? (
            // 破坏性操作：二次确认，避免误触
            <Can perm="job:write">
            <Popconfirm
              title="取消该作业？"
              description="作业将立即终止且不可恢复。"
              okText="确认取消"
              cancelText="返回"
              okButtonProps={{ danger: true }}
              onConfirm={() => handleCancel(record.id)}
            >
              <Button type="link" danger size="small">
                取消
              </Button>
            </Popconfirm>
            </Can>
          ) : null}
        </Space>
      ),
    },
  ], [handleCancel, partitionName]);

  // 历史记录额外展示错误信息列
  const columns = useMemo(() => {
    if (activeKey === '/job/history') {
      return [
        ...baseColumns,
        {
          title: '错误信息',
          dataIndex: 'error_msg',
          key: 'error_msg',
          render: (v: string) => v || '-',
        },
      ];
    }
    return baseColumns;
  }, [activeKey, baseColumns]);

  // 按子路由过滤数据，使每个子菜单项对应真实数据集
  const currentData = useMemo(() => {
    if (activeKey === '/job/queue') {
      return jobsData.filter((j) => j.status === 'pending');
    }
    if (activeKey === '/job/history') {
      return jobsData.filter((j) => ['completed', 'failed', 'cancelled'].includes(j.status));
    }
    return jobsData;
  }, [activeKey, jobsData]);

  const emptyText: Record<string, { title: string; desc: string }> = {
    '/job/list': { title: '还没有作业', desc: '提交第一个作业后，就可以在这里跟踪它的运行状态。' },
    '/job/queue': { title: '队列为空', desc: '当前没有排队中的作业，提交的作业会立即进入调度。' },
    '/job/history': { title: '暂无历史记录', desc: '作业完成、失败或取消后会归档到这里。' },
  };

  /** 三态守卫：错误 → 加载骨架屏 → 空态，其余情况渲染真实表格 */
  const renderTable = () => {
    const state = renderState({
      isLoading,
      error: jobsError,
      isEmpty: currentData.length === 0,
      onRetry: refetch,
      skeletonRows: 6,
      skeletonColumns: 7,
      empty: (
        <EmptyState
          title={emptyText[activeKey]?.title}
          description={emptyText[activeKey]?.desc}
          action={
            <Can perm="job:write">
              <Button type="primary" onClick={() => setIsModalVisible(true)}>
                新建作业
              </Button>
            </Can>
          }
        />
      ),
    });
    if (state) return state;

    return (
      <ResponsiveTable
        columns={columns}
        dataSource={currentData}
        rowKey="id"
        pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
        scroll={{ x: 1200, y: 520 }}
        // 作业列表可能达数百行，启用 antd 虚拟滚动仅渲染可视区行，
        // 要求行高固定（本表格均为纯文本行，满足条件）。
        virtual
      />
    );
  };

  // 页头统计数据用 useMemo 缓存，避免每次渲染重复 filter
  const headerStats = useMemo(() => ({
    total: jobsData.length,
    pending: jobsData.filter((j) => j.status === 'pending').length,
    running: jobsData.filter((j) => j.status === 'running').length,
  }), [jobsData]);

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">作业管理</h1>
          <p className="mc-page-desc">
            共 {headerStats.total} 个作业 · 排队{' '}
            {headerStats.pending} 个 · 运行中{' '}
            {headerStats.running} 个
          </p>
        </div>
        <div className="mc-page-head-extra">
          <Can perm="job:write">
            <Button type="primary" onClick={() => setIsModalVisible(true)}>
              新建作业
            </Button>
          </Can>
        </div>
      </div>

      <Card>
        <Tabs
          activeKey={activeKey}
          items={JOB_TABS.map((t) => ({ ...t, children: renderTable() }))}
          onChange={(key) => navigate(key)}
        />
      </Card>

      <Modal
        className="mc-modal-full"
        title="创建作业"
        open={isModalVisible}
        onCancel={() => setIsModalVisible(false)}
        footer={null}
        destroyOnHidden
        width={800}
      >
        <Form form={form} onFinish={handleCreate} layout="vertical">
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入作业名称' }]}>
                <Input placeholder="例如：llama3-70b-finetune" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="type" label="类型" rules={[{ required: true, message: '请选择作业类型' }]}>
                <Select placeholder="请选择类型">
                  <Select.Option value="training">训练</Select.Option>
                  <Select.Option value="inference">推理</Select.Option>
                  <Select.Option value="batch">批处理</Select.Option>
                </Select>
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="gpus" label="GPU数量" rules={[{ required: true, message: '请输入 GPU 数量' }]}>
                <InputNumber min={1} max={8} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="gpu_fraction" label="GPU分数">
                <Select placeholder="请选择分数" allowClear options={GPU_FRACTION_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="gpu_memory_gb" label="GPU显存(GB)">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="gpu_vendor" label="GPU厂商">
                <Select placeholder="请选择厂商" allowClear options={GPU_VENDOR_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="partition_id" label="分区">
                <Select
                  placeholder="请选择分区"
                  allowClear
                  options={partitionsData.map((p) => ({ label: p.name, value: p.id }))}
                />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="scheduler_type" label="调度器类型">
                <Select placeholder="请选择调度器" allowClear options={SCHEDULER_TYPE_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="qos" label="QoS">
                <Input placeholder="例如：normal" />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="nodes_requested" label="请求节点数">
                <InputNumber min={1} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="network_requirement" label="网络需求">
                <Select placeholder="请选择网络需求" allowClear options={NETWORK_REQUIREMENT_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={24}>
              <Form.Item name="node_selector" label="节点选择器 (JSON)">
                <Input.TextArea rows={2} placeholder='例如：{"gpu-type":"a100"}' />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="affinity" label="亲和性 (JSON)">
                <Input.TextArea rows={2} placeholder="K8s affinity 配置" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="tolerations" label="容忍度 (JSON)">
                <Input.TextArea rows={2} placeholder="K8s tolerations 配置" />
              </Form.Item>
            </Col>
            {/* 弹性训练 */}
            <Col span={8}>
              <Form.Item name="elastic_enabled" label="弹性训练" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="min_gpus" label="最小GPU">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="max_gpus" label="最大GPU">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="scaling_policy" label="扩缩容策略">
                <Input placeholder="例如：cpu_utilization" />
              </Form.Item>
            </Col>
            {/* 容错训练 */}
            <Col span={8}>
              <Form.Item name="checkpoint_enabled" label="Checkpoint" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="checkpoint_interval_minutes" label="Checkpoint间隔(分钟)">
                <InputNumber min={1} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="max_retries" label="最大重试次数">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="fault_tolerance_level" label="容错级别">
                <Select placeholder="请选择容错级别" allowClear options={FAULT_TOLERANCE_OPTIONS} />
              </Form.Item>
            </Col>
            {/* 拓扑感知 */}
            <Col span={8}>
              <Form.Item name="topology_affinity" label="拓扑亲和">
                <Select placeholder="请选择拓扑亲和" allowClear options={TOPOLOGY_AFFINITY_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="topology_anti_affinity" label="拓扑反亲和" valuePropName="checked">
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
              <Can perm="job:write">
              <Button type="primary" htmlType="submit">
                创建
              </Button>
              </Can>
              <Button onClick={() => setIsModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* 作业详情 Drawer */}
      <Drawer
        title="作业详情"
        open={detailJob !== null}
        onClose={() => setDetailJob(null)}
        width={520}
      >
        {detailJob && (
          <div>
            <DetailRow label="名称" value={detailJob.name} />
            <DetailRow label="类型" value={detailJob.type ?? '-'} />
            <DetailRow label="状态" value={<StatusCell status={detailJob.status} />} />
            <DetailRow label="GPU数量" value={detailJob.gpus ?? '-'} />
            <DetailRow label="GPU分数" value={detailJob.gpu_fraction ?? '-'} />
            <DetailRow label="GPU显存" value={detailJob.gpu_memory_gb ? `${detailJob.gpu_memory_gb} GB` : '-'} />
            <DetailRow label="GPU厂商" value={detailJob.gpu_vendor ?? '-'} />
            <DetailRow label="分区" value={partitionName(detailJob.partition_id)} />
            <DetailRow label="调度器类型" value={detailJob.scheduler_type ?? '-'} />
            <DetailRow label="调度器作业ID" value={detailJob.scheduler_job_id ?? '-'} />
            <DetailRow label="QoS" value={detailJob.qos ?? '-'} />
            <DetailRow label="请求节点数" value={detailJob.nodes_requested ?? '-'} />
            <DetailRow label="弹性训练" value={detailJob.elastic_enabled ? '开启' : '关闭'} />
            {detailJob.elastic_enabled && (
              <>
                <DetailRow label="最小GPU" value={detailJob.min_gpus ?? '-'} />
                <DetailRow label="最大GPU" value={detailJob.max_gpus ?? '-'} />
                <DetailRow label="扩缩容策略" value={detailJob.scaling_policy ?? '-'} />
              </>
            )}
            <DetailRow label="Checkpoint" value={detailJob.checkpoint_enabled ? '开启' : '关闭'} />
            {detailJob.checkpoint_enabled && (
              <DetailRow label="Checkpoint间隔" value={`${detailJob.checkpoint_interval_minutes ?? '-'} 分钟`} />
            )}
            <DetailRow label="最大重试次数" value={detailJob.max_retries ?? '-'} />
            <DetailRow label="已重试次数" value={detailJob.retry_count ?? '-'} />
            <DetailRow label="容错级别" value={detailJob.fault_tolerance_level ?? '-'} />
            <DetailRow label="拓扑亲和" value={detailJob.topology_affinity ?? '-'} />
            <DetailRow label="拓扑反亲和" value={detailJob.topology_anti_affinity ? '开启' : '关闭'} />
            <DetailRow label="网络需求" value={detailJob.network_requirement ?? '-'} />
            <DetailRow label="节点选择器" value={detailJob.node_selector ?? '-'} />
            <DetailRow label="亲和性" value={detailJob.affinity ?? '-'} />
            <DetailRow label="容忍度" value={detailJob.tolerations ?? '-'} />
            <DetailRow label="进度" value={`${detailJob.progress ?? 0}%`} />
            <DetailRow label="描述" value={detailJob.description ?? '-'} />
          </div>
        )}
      </Drawer>
    </div>
  );
};

/** 详情行 */
const DetailRow: React.FC<{ label: string; value: React.ReactNode }> = ({ label, value }) => (
  <div style={{ display: 'flex', padding: '8px 0', borderBottom: '1px solid var(--mc-border)' }}>
    <span style={{ width: 130, color: 'var(--mc-text-3)', flexShrink: 0 }}>{label}</span>
    <span style={{ flex: 1, wordBreak: 'break-all' }}>{value}</span>
  </div>
);

export default JobManagement;
