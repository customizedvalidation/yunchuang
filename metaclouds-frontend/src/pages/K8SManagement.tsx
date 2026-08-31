import React, { useState } from 'react';
import { Card, Table, Button, Space, message, Modal, Progress, Tag, List, Typography, Tabs, Segmented, Statistic } from 'antd';
import { useGetJobsQuery, useSubmitJobToK8SMutation, useCancelK8SJobMutation, useGetK8SGPUResourcesQuery, useGetTenantsQuery, useGetResourcesQuery } from '../store/api';
import { useLocation, useNavigate } from 'react-router-dom';

const { Title, Text } = Typography;

const extractArrayData = (response: any): any[] => {
  if (!response) {
    return [];
  }

  if (response.success === false) {
    return [];
  }

  if (response.data && Array.isArray(response.data)) {
    return response.data;
  }

  if (Array.isArray(response)) {
    return response;
  }

  return [];
};

const K8S_TABS = [
  { key: '/k8s', label: '概览' },
  { key: '/k8s/nodes', label: '节点管理' },
  { key: '/k8s/pods', label: 'Pod管理' },
  { key: '/k8s/services', label: '服务管理' },
];

const K8SManagement: React.FC = () => {
  const location = useLocation();
  const navigate = useNavigate();
  // 子路由驱动当前激活页签；父路由 /k8s 默认落到“概览”
  const activeKey = location.pathname.startsWith('/k8s/') ? location.pathname : '/k8s';

  const { data: jobs, isLoading: jobsLoading, refetch: refetchJobs } = useGetJobsQuery(undefined);
  const [submitJobToK8S] = useSubmitJobToK8SMutation();
  const [cancelK8SJob] = useCancelK8SJobMutation();
  const { data: gpuResources, isLoading: gpuLoading, refetch: refetchGPU } = useGetK8SGPUResourcesQuery(undefined);
  const { data: tenants } = useGetTenantsQuery(undefined);
  const { data: resources, isLoading: resourcesLoading } = useGetResourcesQuery(undefined);

  const jobsData = extractArrayData(jobs);
  const gpuResourcesData = extractArrayData(gpuResources);
  const tenantsData = extractArrayData(tenants);
  const resourcesData = extractArrayData(resources);
  // 命名空间按租户划分；K8S 服务的“命名空间”即租户边界
  const namespaces = tenantsData.length ? tenantsData : [{ id: 0, name: 'default', description: '默认命名空间', status: 'active', gpu_quota: 0, cpu_quota: 0, memory_quota: 0, storage_quota: 0 }];
  const [selectedNs, setSelectedNs] = useState<string>(namespaces[0]?.name || 'default');
  // Pod 管理展示正在运行的作业（活跃工作负载）
  const podsData = jobsData.filter((j: any) => j.status === 'running');

  const [selectedJob, setSelectedJob] = useState<any>(null);
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [isGpuRefreshing, setIsGpuRefreshing] = useState(false);

  const handleRefreshGPU = async () => {
    setIsGpuRefreshing(true);
    message.loading('正在刷新GPU资源...', 0);

    try {
      await refetchGPU();
      message.success('GPU资源刷新成功');
    } catch {
      message.error('GPU资源刷新失败');
    } finally {
      setIsGpuRefreshing(false);
      message.destroy();
    }
  };

  const handleSubmitToK8S = async (job: any) => {
    try {
      await submitJobToK8S(job.id).unwrap();
      message.success('作业已提交到K8S');
      refetchJobs();
    } catch {
      message.error('提交作业到K8S失败');
    }
  };

  const handleCancelK8SJob = async (job: any) => {
    try {
      await cancelK8SJob(job.id).unwrap();
      message.success('作业已取消');
      refetchJobs();
    } catch {
      message.error('取消作业失败');
    }
  };

  const handleViewStatus = (job: any) => {
    setSelectedJob(job);
    setIsModalVisible(true);
  };

  const jobColumns: any[] = [
    { title: 'ID', dataIndex: 'id', key: 'id' },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag>{type}</Tag> },
    { title: '状态', dataIndex: 'status', key: 'status', render: (status: string) => <Tag color={status === 'running' ? 'green' : status === 'completed' ? 'blue' : status === 'failed' ? 'red' : 'orange'}>{status}</Tag> },
    { title: 'GPU数', dataIndex: 'gpus', key: 'gpus' },
    { title: 'CPU数', dataIndex: 'cpus', key: 'cpus' },
    { title: '内存(GB)', dataIndex: 'memory', key: 'memory' },
    { title: '进度', dataIndex: 'progress', key: 'progress', render: (progress: number) => <Progress percent={progress} size="small" /> },
    { title: '操作', key: 'action', render: (_: any, record: any) => (
      <Space>
        <Button type="link" onClick={() => handleSubmitToK8S(record)}>提交到K8S</Button>
        <Button type="link" onClick={() => handleViewStatus(record)}>查看状态</Button>
        <Button type="link" onClick={() => handleCancelK8SJob(record)}>取消作业</Button>
      </Space>
    )},
  ];

  const gpuColumns: any[] = [
    { title: '名称', dataIndex: 'gpuName', key: 'gpuName' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag>{type}</Tag> },
    { title: '状态', dataIndex: 'status', key: 'status', render: (status: string) => <Tag color={status === 'available' ? 'green' : 'red'}>{status}</Tag> },
    { title: '总量', dataIndex: 'total', key: 'total' },
    { title: '已用', dataIndex: 'used', key: 'used' },
    { title: '可用', dataIndex: 'available', key: 'available' },
    { title: '利用率', dataIndex: 'utilization', key: 'utilization', render: (util: number) => <Progress percent={util} size="small" /> },
    { title: '详情', dataIndex: 'details', key: 'details' },
  ];

  const renderGpuCard = () => (
    <Card title="GPU资源管理（节点）" className="mb-6">
      <Button
        type="primary"
        onClick={handleRefreshGPU}
        loading={isGpuRefreshing}
        className="mb-4"
      >
        刷新GPU资源
      </Button>
      <Table
        columns={gpuColumns}
        dataSource={gpuResourcesData}
        loading={gpuLoading}
        rowKey={(record: any) => record.id}
        pagination={false}
        bordered
        className="gpu-resource-table"
      />
    </Card>
  );

  const renderJobsCard = (dataSource: any[], loading: boolean) => (
    <Card title="作业管理（Pod）">
      <Button type="primary" onClick={() => refetchJobs()} className="mb-4">
        刷新作业列表
      </Button>
      <Table
        columns={jobColumns}
        dataSource={dataSource}
        loading={loading}
        rowKey="id"
        pagination={{ pageSize: 10 }}
      />
    </Card>
  );

  // 服务管理：按命名空间（租户）重新映射语义 —— 命名空间=租户边界，GPU 资源=该命名空间下可调度服务
  // 配额“已用”取自实际运行作业占用的 GPU/CPU/内存（单租户下即全平台运行负载；多租户需后端建立 job↔tenant 关联后按租户归集）
  const renderServicesCard = () => {
    const ns = namespaces.find((n: any) => n.name === selectedNs) || namespaces[0];
    const runningJobs = jobsData.filter((j: any) => j.status === 'running');
    const usedGpu = runningJobs.reduce((s: number, j: any) => s + (Number(j.gpus) || 0), 0);
    const usedCpu = runningJobs.reduce((s: number, j: any) => s + (Number(j.cpus) || 0), 0);
    const usedMem = runningJobs.reduce((s: number, j: any) => s + (Number(j.memory) || 0), 0);
    const quotaItems = [
      { label: 'GPU 配额', value: ns.gpu_quota, used: usedGpu, total: ns.gpu_quota },
      { label: 'CPU 配额', value: ns.cpu_quota, used: usedCpu, total: ns.cpu_quota },
      { label: '内存配额(GB)', value: ns.memory_quota, used: usedMem, total: ns.memory_quota },
      { label: '存储配额(TB)', value: ns.storage_quota },
    ];
    const serviceColumns: any[] = [
      { title: '命名空间', dataIndex: 'namespace', key: 'namespace', render: () => <Tag color="blue">{ns.name}</Tag> },
      { title: '服务名', dataIndex: 'gpuName', key: 'gpuName' },
      { title: '类型', dataIndex: 'type', key: 'type', render: (t: string) => <Tag>{t}</Tag> },
      { title: '状态', dataIndex: 'status', key: 'status', render: (s: string) => <Tag color={s === 'available' ? 'green' : 'red'}>{s}</Tag> },
      { title: '容量', dataIndex: 'total', key: 'total' },
      { title: '已用', dataIndex: 'used', key: 'used' },
      { title: '可用', dataIndex: 'available', key: 'available' },
      { title: '利用率', dataIndex: 'utilization', key: 'utilization', render: (u: number) => <Progress percent={u} size="small" /> },
    ];

    return (
      <Card title="服务管理（按命名空间 / 租户）">
        <Segmented
          options={namespaces.map((n: any) => ({ label: n.name, value: n.name }))}
          value={selectedNs}
          onChange={(v) => setSelectedNs(v as string)}
          className="mb-4"
        />
        <div className="mb-4">
          <Text type="secondary">
            命名空间「{ns.name}」 · 租户状态：{ns.status} · {ns.description}
          </Text>
        </div>
        <div style={{ display: 'flex', gap: 16, flexWrap: 'wrap', marginBottom: 16 }}>
          {quotaItems.map((q) => (
            <Card key={q.label} size="small" style={{ flex: '1 1 200px' }}>
              <Statistic title={q.label} value={q.value} />
              {q.used != null && (
                <Progress
                  percent={q.total ? Math.round((q.used / q.total) * 100) : 0}
                  size="small"
                  status={(q.used / (q.total || 1)) > 0.8 ? 'exception' : 'normal'}
                />
              )}
            </Card>
          ))}
        </div>
        <Table
          columns={serviceColumns}
          dataSource={resourcesData}
          loading={resourcesLoading}
          rowKey="id"
          pagination={{ pageSize: 10 }}
          bordered
        />
        <div className="mt-3">
          <Text type="secondary">
            说明：K8S 服务的命名空间按租户划分；配额「已用」取当前运行作业实际占用的 GPU/CPU/内存。当前为单租户演示环境，全部服务归属于默认命名空间；多租户部署时需在后端建立 job↔tenant 关联后按租户归集配额。
          </Text>
        </div>
      </Card>
    );
  };

  const renderTab = (key: string) => {
    if (key === '/k8s') return (<><>{renderGpuCard()}</>{renderJobsCard(jobsData, jobsLoading)}</>);
    if (key === '/k8s/nodes') return renderGpuCard();
    if (key === '/k8s/pods') return renderJobsCard(podsData, jobsLoading);
    if (key === '/k8s/services') return renderServicesCard();
    return null;
  };

  return (
    <div className="p-6">
      <Title level={2}>K8S管理</Title>

      <Tabs
        activeKey={activeKey}
        items={K8S_TABS.map((t) => ({ ...t, children: renderTab(t.key) }))}
        onChange={(key) => navigate(key)}
      />

      <Modal
        title="作业状态"
        open={isModalVisible}
        onCancel={() => setIsModalVisible(false)}
        footer={null}
      >
        {selectedJob && (
          <div>
            <List
              dataSource={[
                { key: 'id', label: '作业ID', value: selectedJob.id },
                { key: 'name', label: '作业名称', value: selectedJob.name },
                { key: 'type', label: '作业类型', value: selectedJob.type },
                { key: 'status', label: '作业状态', value: selectedJob.status },
                { key: 'gpus', label: 'GPU数', value: selectedJob.gpus },
                { key: 'cpus', label: 'CPU数', value: selectedJob.cpus },
                { key: 'memory', label: '内存(GB)', value: selectedJob.memory },
                { key: 'progress', label: '进度', value: `${selectedJob.progress}%` },
              ]}
              renderItem={(item) => (
                <List.Item>
                  <Text strong>{item.label}:</Text> {item.value}
                </List.Item>
              )}
            />
            <div className="mt-4">
              <Button type="primary" onClick={() => setIsModalVisible(false)}>
                关闭
              </Button>
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
};

export default K8SManagement;
