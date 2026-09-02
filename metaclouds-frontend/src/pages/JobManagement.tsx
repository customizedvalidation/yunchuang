import { Can } from '../components/Can';
import React from 'react';
import { Card } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import { Button, Space, App, Modal, Form, Input, Select, InputNumber, Tabs, Popconfirm } from 'antd';
import { useGetJobsQuery, useCreateJobMutation, useCancelJobMutation } from '../store/api';
import { extractArrayData } from '../utils/api';
import { useLocation, useNavigate } from 'react-router-dom';
import { renderState, EmptyState } from '../components/States';
import { statusColor, statusText } from '../theme/tokens';
import type { Job, JobType } from '../types';

/** 新建作业表单值 */
interface JobFormValues {
  name: string;
  type: JobType;
  gpus: number;
  description?: string;
}

const JOB_TABS = [
  { key: '/job/list', label: '作业列表' },
  { key: '/job/queue', label: '任务队列' },
  { key: '/job/history', label: '历史记录' },
];

/** 状态 = 色点 + 文字，禁止仅用颜色表意 */
const StatusCell: React.FC<{ status: string }> = ({ status }) => {
  const cls = ['running', 'pending', 'completed', 'failed'].includes(status) ? status : 'idle';
  return (
    <span className={`mc-status ${cls}`}>
      <i className="mc-status-dot" style={{ background: statusColor[status] }} />
      {statusText[status] ?? status}
    </span>
  );
};

const JobManagement: React.FC = () => {
  const { message } = App.useApp();
  const location = useLocation();
  const navigate = useNavigate();
  // 子路由驱动当前激活的页签；父路由 /job 默认落到“作业列表”
  const activeKey = location.pathname.startsWith('/job/') ? location.pathname : '/job/list';

  const { data: jobs, isLoading, error: jobsError, refetch } = useGetJobsQuery(undefined);
  const jobsData = extractArrayData<Job>(jobs);
  const [createJob] = useCreateJobMutation();
  const [cancelJob] = useCancelJobMutation();
  const [isModalVisible, setIsModalVisible] = React.useState(false);
  const [form] = Form.useForm();

  const handleCreate = async (values: JobFormValues) => {
    try {
      await createJob(values).unwrap();
      message.success('作业创建成功');
      setIsModalVisible(false);
      form.resetFields();
      refetch();
    } catch {
      message.error('作业创建失败，请检查必填项后重试');
    }
  };

  const handleCancel = async (id: number) => {
    try {
      await cancelJob(id).unwrap();
      message.success('作业已取消');
      refetch();
    } catch {
      message.error('作业取消失败，请稍后重试');
    }
  };

  const baseColumns: ColumnsType<Job> = [
    {
      title: 'ID',
      dataIndex: 'id',
      key: 'id',
      width: 90,
      render: (v: React.ReactNode) => <span className="mc-mono">{v}</span>,
    },
    { title: '名称', dataIndex: 'name', key: 'name' },
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
  ];

  // 历史记录额外展示错误信息列
  const columns =
    activeKey === '/job/history'
      ? [
          ...baseColumns,
          {
            title: '错误信息',
            dataIndex: 'error_msg',
            key: 'error_msg',
            render: (v: string) => v || '-',
          },
        ]
      : baseColumns;

  // 按子路由过滤数据，使每个子菜单项对应真实数据集
  const currentData =
    activeKey === '/job/queue'
      ? jobsData.filter((j) => j.status === 'pending')
      : activeKey === '/job/history'
      ? jobsData.filter((j) => ['completed', 'failed', 'cancelled'].includes(j.status))
      : jobsData;

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
        scroll={{ x: 900 }}
      />
    );
  };

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">作业管理</h1>
          <p className="mc-page-desc">
            共 {jobsData.length} 个作业 · 排队{' '}
            {jobsData.filter((j) => j.status === 'pending').length} 个 · 运行中{' '}
            {jobsData.filter((j) => j.status === 'running').length} 个
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
      >
        <Form form={form} onFinish={handleCreate} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入作业名称' }]}>
            <Input placeholder="例如：llama3-70b-finetune" />
          </Form.Item>
          <Form.Item name="type" label="类型" rules={[{ required: true, message: '请选择作业类型' }]}>
            <Select placeholder="请选择类型">
              <Select.Option value="training">训练</Select.Option>
              <Select.Option value="inference">推理</Select.Option>
              <Select.Option value="batch">批处理</Select.Option>
            </Select>
          </Form.Item>
          <Form.Item name="gpus" label="GPU数量" rules={[{ required: true, message: '请输入 GPU 数量' }]}>
            <InputNumber min={1} max={8} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea rows={3} placeholder="选填，便于后续追溯" />
          </Form.Item>
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
    </div>
  );
};

export default JobManagement;
