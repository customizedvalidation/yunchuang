import { Can } from '../components/Can';
import React, { useState } from 'react';
import { Card, Button, Space, App, Modal, Progress, Tag, List, Typography, Tabs, Segmented, Statistic } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import {
  useGetJobsQuery,
  useSubmitJobToK8SMutation,
  useCancelK8SJobMutation,
  useGetK8SGPUResourcesQuery,
  useGetTenantsQuery,
  useGetResourcesQuery,
} from '../store/api';
import { useLocation, useNavigate } from 'react-router-dom';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { Job, Tenant, GPUResource, Resource } from '../types';

const { Text } = Typography;

const K8S_TABS = [
  { key: '/k8s', label: '概览' },
  { key: '/k8s/nodes', label: '节点管理' },
  { key: '/k8s/pods', label: 'Pod管理' },
  { key: '/k8s/services', label: '服务管理' },
];

const K8SManagement: React.FC = () => {
  const { message } = App.useApp();
  const location = useLocation();
  const navigate = useNavigate();
  const activeKey = location.pathname.startsWith('/k8s/') ? location.pathname : '/k8s';

  const { data: jobs, isLoading: jobsLoading, error: jobsError, refetch: refetchJobs } = useGetJobsQuery(undefined);
  const [submitJobToK8S] = useSubmitJobToK8SMutation();
  const [cancelK8SJob] = useCancelK8SJobMutation();
  const { data: gpuResources, isLoading: gpuLoading, error: gpuError, refetch: refetchGPU } = useGetK8SGPUResourcesQuery(undefined);
  const { data: tenants } = useGetTenantsQuery(undefined);
  const { data: resources, isLoading: resourcesLoading, error: resourcesError } = useGetResourcesQuery(undefined);

  const jobsData = extractArrayData<Job>(jobs);
  const gpuResourcesData = extractArrayData<GPUResource>(gpuResources);
  const tenantsData = extractArrayData<Tenant>(tenants);
  const resourcesData = extractArrayData(resources);
  const namespaces: Tenant[] = tenantsData.length
    ? tenantsData
    : [{ id: 0, name: 'default', description: '默认命名空间', status: 'active', gpu_quota: 0, cpu_quota: 0, memory_quota: 0, storage_quota: 0 }];
  const [selectedNs, setSelectedNs] = useState<string>(namespaces[0]?.name || 'default');
  const podsData = jobsData.filter((j) => j.status === 'running');

  const [selectedJob, setSelectedJob] = useState<Job | null>(null);
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

  const handleSubmitToK8S = async (job: Job) => {
    try {
      await submitJobToK8S(job.id).unwrap();
      message.success('作业已提交到K8S');
      refetchJobs();
    } catch {
      message.error('提交作业到K8S失败');
    }
  };

  const handleCancelK8SJob = async (job: Job) => {
    try {
      await cancelK8SJob(job.id).unwrap();
      message.success('作业已取消');
      refetchJobs();
    } catch {
      message.error('取消作业失败');
    }
  };

  const handleViewStatus = (job: Job) => {
    setSelectedJob(job);
    setIsModalVisible(true);
  };

  const jobColumns: ColumnsType<Job> = [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 80, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag>{type}</Tag> },
    { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (status: string) => <StatusCell status={status} /> },
    { title: 'GPU数', dataIndex: 'gpus', key: 'gpus', width: 80, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: 'CPU数', dataIndex: 'cpus', key: 'cpus', width: 80, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '内存(GB)', dataIndex: 'memory', key: 'memory', width: 100, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '进度', dataIndex: 'progress', key: 'progress', width: 120, render: (progress: number) => <Progress percent={progress} size="small" /> },
    {
      title: '操作', key: 'action', width: 220,
      render: (_: React.ReactNode, record: Job) => (
        <Space>
          <Can perm="job:submit"><Button type="link" onClick={() => handleSubmitToK8S(record)}>提交到K8S</Button></Can>
          <Button type="link" onClick={() => handleViewStatus(record)}>查看状态</Button>
          <Can perm="job:write"><Button type="link" danger onClick={() => handleCancelK8SJob(record)}>取消作业</Button></Can>
        </Space>
      ),
    },
  ];

  const gpuColumns: ColumnsType<GPUResource> = [
    { title: '名称', dataIndex: 'gpuName', key: 'gpuName' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag>{type}</Tag> },
    { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (status: string) => <StatusCell status={status} /> },
    { title: '总量', dataIndex: 'total', key: 'total', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '已用', dataIndex: 'used', key: 'used', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '可用', dataIndex: 'available', key: 'available', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '利用率', dataIndex: 'utilization', key: 'utilization', width: 140, render: (util: number) => <Progress percent={util} size="small" /> },
    { title: '详情', dataIndex: 'details', key: 'details' },
  ];

  const renderGpuCard = () => {
    const state = renderState({
      isLoading: gpuLoading,
      error: gpuError,
      isEmpty: gpuResourcesData.length === 0,
      onRetry: refetchGPU,
      skeletonRows: 5,
      skeletonColumns: 8,
      empty: <EmptyState title="暂无 GPU 节点" description="当前集群没有登记 GPU 节点资源。" />,
    });
    return (
      <Card title="GPU资源管理（节点）">
        <Button type="primary" className="mc-mb" onClick={handleRefreshGPU} loading={isGpuRefreshing}>
          刷新GPU资源
        </Button>
        {state ?? (
          <ResponsiveTable
            columns={gpuColumns}
            dataSource={gpuResourcesData}
            loading={gpuLoading}
            rowKey={(record: GPUResource) => record.gpuName}
            pagination={false}
            scroll={{ x: 900 }}
          />
        )}
      </Card>
    );
  };

  const renderJobsCard = (dataSource: Job[], loading: boolean, err: unknown) => {
    const state = renderState({
      isLoading: loading,
      error: err,
      isEmpty: dataSource.length === 0,
      onRetry: refetchJobs,
      skeletonRows: 6,
      skeletonColumns: 9,
      empty: <EmptyState title="暂无作业" description="提交或调度作业后，运行中的工作负载会出现在这里。" />,
    });
    return (
      <Card title="作业管理（Pod）">
        <Button type="primary" className="mc-mb" onClick={() => refetchJobs()}>
          刷新作业列表
        </Button>
        {state ?? (
          <ResponsiveTable
            columns={jobColumns}
            dataSource={dataSource}
            rowKey="id"
            pagination={{ pageSize: 10 }}
            scroll={{ x: 1100 }}
          />
        )}
      </Card>
    );
  };

  const renderServicesCard = () => {
    const ns = namespaces.find((n) => n.name === selectedNs) || namespaces[0];
    const runningJobs = jobsData.filter((j) => j.status === 'running');
    const usedGpu = runningJobs.reduce((s, j) => s + (Number(j.gpus) || 0), 0);
    const usedCpu = runningJobs.reduce((s, j) => s + (Number(j.cpus) || 0), 0);
    const usedMem = runningJobs.reduce((s, j) => s + (Number(j.memory) || 0), 0);
    const quotaItems = [
      { label: 'GPU 配额', value: ns.gpu_quota, used: usedGpu, total: ns.gpu_quota },
      { label: 'CPU 配额', value: ns.cpu_quota, used: usedCpu, total: ns.cpu_quota },
      { label: '内存配额(GB)', value: ns.memory_quota, used: usedMem, total: ns.memory_quota },
      { label: '存储配额(TB)', value: ns.storage_quota },
    ];
    const serviceColumns: ColumnsType<Resource> = [
      { title: '命名空间', dataIndex: 'namespace', key: 'namespace', width: 120, render: () => <Tag color="blue">{ns.name}</Tag> },
      { title: '服务名', dataIndex: 'gpuName', key: 'gpuName' },
      { title: '类型', dataIndex: 'type', key: 'type', render: (t: string) => <Tag>{t}</Tag> },
      { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (s: string) => <StatusCell status={s} /> },
      { title: '容量', dataIndex: 'total', key: 'total', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
      { title: '已用', dataIndex: 'used', key: 'used', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
      { title: '可用', dataIndex: 'available', key: 'available', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
      { title: '利用率', dataIndex: 'utilization', key: 'utilization', width: 140, render: (u: number) => <Progress percent={u} size="small" /> },
    ];
    const state = renderState({
      isLoading: resourcesLoading,
      error: resourcesError,
      isEmpty: resourcesData.length === 0,
      onRetry: refetchJobs,
      skeletonRows: 5,
      skeletonColumns: 8,
      empty: <EmptyState title="暂无服务" description="该命名空间下还没有可调度的服务资源。" />,
    });
    return (
      <Card title="服务管理（按命名空间 / 租户）">
        <Segmented
          className="mc-mb"
          options={namespaces.map((n) => ({ label: n.name, value: n.name }))}
          value={selectedNs}
          onChange={(v) => setSelectedNs(v as string)}
        />
        <div className="mc-mb">
          <Text type="secondary">
            命名空间「{ns.name}」 · 租户状态：<StatusCell status={ns.status} /> · {ns.description}
          </Text>
        </div>
        <div className="mc-quota-grid mc-mb">
          {quotaItems.map((q) => (
            <Card key={q.label} size="small">
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
        {state ?? (
          <ResponsiveTable
            columns={serviceColumns}
            dataSource={resourcesData}
            loading={resourcesLoading}
            rowKey="id"
            pagination={{ pageSize: 10 }}
            scroll={{ x: 900 }}
          />
        )}
        <div className="mc-mt">
          <Text type="secondary">
            说明：K8S 服务的命名空间按租户划分；配额「已用」取当前运行作业实际占用的 GPU/CPU/内存。当前为单租户演示环境，全部服务归属于默认命名空间；多租户部署时需在后端建立 job↔tenant 关联后按租户归集配额。
          </Text>
        </div>
      </Card>
    );
  };

  const renderTab = (key: string) => {
    if (key === '/k8s') {
      return (
        <div className="mc-stack">
          {renderGpuCard()}
          {renderJobsCard(jobsData, jobsLoading, jobsError)}
        </div>
      );
    }
    if (key === '/k8s/nodes') return renderGpuCard();
    if (key === '/k8s/pods') return renderJobsCard(podsData, jobsLoading, jobsError);
    if (key === '/k8s/services') return renderServicesCard();
    return null;
  };

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">K8S 管理</h1>
          <p className="mc-page-desc">GPU 节点、运行 Pod 与按命名空间划分的服务配额总览</p>
        </div>
      </div>

      <Tabs
        activeKey={activeKey}
        items={K8S_TABS.map((t) => ({ ...t, children: renderTab(t.key) }))}
        onChange={(key) => navigate(key)}
      />

      <Modal className="mc-modal-full" title="作业状态" open={isModalVisible} onCancel={() => setIsModalVisible(false)} footer={null} destroyOnHidden>
        {selectedJob && (
          <div>
            <List
              dataSource={[
                { key: 'id', label: '作业ID', value: selectedJob.id },
                { key: 'name', label: '作业名称', value: selectedJob.name },
                { key: 'type', label: '作业类型', value: selectedJob.type },
                { key: 'status', label: '作业状态', value: <StatusCell status={selectedJob.status} /> },
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
            <div className="mc-mt">
              <Button type="primary" onClick={() => setIsModalVisible(false)}>关闭</Button>
            </div>
          </div>
        )}
      </Modal>
    </div>
  );
};

export default K8SManagement;
