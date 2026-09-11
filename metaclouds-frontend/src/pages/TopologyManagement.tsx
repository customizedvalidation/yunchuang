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
  Tag,
  Popconfirm,
  Drawer,
  Row,
  Col,
} from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import ResponsiveChart from '../components/ResponsiveChart';
import type { ColumnsType } from 'antd/es/table';
import {
  useGetNodeTopologiesQuery,
  useCreateNodeTopologyMutation,
  useUpdateNodeTopologyMutation,
  useDeleteNodeTopologyMutation,
  useCalculateTopologyScoreMutation,
  useGetClustersQuery,
  useGetJobsQuery,
} from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import type { NodeTopology } from '../types';
import { useThemeMode } from '../theme/ThemeModeContext';
import { chartPalette, getNeutral } from '../theme/tokens';

const NETWORK_TYPE_OPTIONS = [
  { label: 'RoCE', value: 'roce' },
  { label: 'InfiniBand', value: 'ib' },
  { label: 'Ethernet', value: 'ethernet' },
];

/** 拓扑表单值 */
interface TopologyFormValues {
  node_name: string;
  cluster_id?: number;
  rack_id?: string;
  switch_id?: string;
  pod_id?: string;
  numa_nodes?: string;
  gpu_topology?: string;
  network_type?: 'roce' | 'ib' | 'ethernet';
  network_bandwidth_gbps?: number;
  rdma_enabled?: boolean;
  fault_domain?: string;
}

const TopologyManagement: React.FC = () => {
  const { message } = App.useApp();
  const { mode } = useThemeMode();
  const colors = useMemo(() => getNeutral(mode), [mode]);

  // 表单
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [editingTopology, setEditingTopology] = useState<NodeTopology | null>(null);
  const [form] = Form.useForm<TopologyFormValues>();

  // 详情 Drawer
  const [detailTopology, setDetailTopology] = useState<NodeTopology | null>(null);

  // 拓扑评分工具
  const [scoreModalVisible, setScoreModalVisible] = useState(false);
  const [scoreJobId, setScoreJobId] = useState<number | undefined>(undefined);
  const [scoreCandidateNodes, setScoreCandidateNodes] = useState('');
  const [scoreResults, setScoreResults] = useState<Record<string, number> | null>(null);

  // 搜索 / 筛选
  const [searchText, setSearchText] = useState('');
  const [clusterFilter, setClusterFilter] = useState<number | undefined>(undefined);
  const [networkFilter, setNetworkFilter] = useState<string>('');

  // API
  const { data: topologies, isLoading, error, refetch } = useGetNodeTopologiesQuery({});
  const { data: clusters } = useGetClustersQuery(undefined);
  const { data: jobs } = useGetJobsQuery(undefined);
  const topologiesData = topologies ?? [];
  const clustersData = extractArrayData(clusters);
  const jobsData = extractArrayData(jobs);

  const [createTopology] = useCreateNodeTopologyMutation();
  const [updateTopology] = useUpdateNodeTopologyMutation();
  const [deleteTopology] = useDeleteNodeTopologyMutation();
  const [calculateScore] = useCalculateTopologyScoreMutation();

  const clusterName = useCallback(
    (id?: number) => clustersData.find((c: { id: number }) => c.id === id)?.name ?? '-',
    [clustersData],
  );

  // 搜索 + 集群 / 网络类型筛选
  const filteredTopologies = useMemo(() => {
    const kw = searchText.trim().toLowerCase();
    return topologiesData.filter((t) => {
      const matchKw =
        !kw ||
        t.node_name.toLowerCase().includes(kw) ||
        (t.rack_id ?? '').toLowerCase().includes(kw) ||
        (t.switch_id ?? '').toLowerCase().includes(kw);
      const matchCluster = clusterFilter == null || t.cluster_id === clusterFilter;
      const matchNetwork = !networkFilter || t.network_type === networkFilter;
      return matchKw && matchCluster && matchNetwork;
    });
  }, [topologiesData, searchText, clusterFilter, networkFilter]);

  // 打开新建
  const handleOpenCreate = useCallback(() => {
    setEditingTopology(null);
    form.resetFields();
    setIsModalVisible(true);
  }, [form]);

  // 打开编辑
  const handleOpenEdit = useCallback(
    (record: NodeTopology) => {
      setEditingTopology(record);
      form.setFieldsValue({
        node_name: record.node_name,
        cluster_id: record.cluster_id,
        rack_id: record.rack_id,
        switch_id: record.switch_id,
        pod_id: record.pod_id,
        numa_nodes: record.numa_nodes,
        gpu_topology: record.gpu_topology,
        network_type: record.network_type,
        network_bandwidth_gbps: record.network_bandwidth_gbps,
        rdma_enabled: record.rdma_enabled,
        fault_domain: record.fault_domain,
      });
      setIsModalVisible(true);
    },
    [form],
  );

  // 提交
  const handleSubmit = useCallback(
    async (values: TopologyFormValues) => {
      try {
        if (editingTopology) {
          await updateTopology({ id: editingTopology.id, data: values }).unwrap();
          message.success('拓扑更新成功');
        } else {
          await createTopology(values).unwrap();
          message.success('拓扑创建成功');
        }
        setIsModalVisible(false);
        form.resetFields();
        refetch();
      } catch {
        message.error('操作失败，请检查必填项后重试');
      }
    },
    [editingTopology, createTopology, updateTopology, form, message, refetch],
  );

  // 删除
  const handleDelete = useCallback(
    async (id: number) => {
      try {
        await deleteTopology(id).unwrap();
        message.success('拓扑删除成功');
        refetch();
      } catch {
        message.error('删除失败，请稍后重试');
      }
    },
    [deleteTopology, message, refetch],
  );

  // 拓扑评分
  const handleCalculateScore = useCallback(async () => {
    if (!scoreJobId) {
      message.warning('请选择作业');
      return;
    }
    const candidates = scoreCandidateNodes
      .split(',')
      .map((s) => s.trim())
      .filter(Boolean);
    if (candidates.length === 0) {
      message.warning('请输入候选节点列表');
      return;
    }
    try {
      const result = await calculateScore({ job_id: scoreJobId, candidate_nodes: candidates }).unwrap();
      setScoreResults(result as Record<string, number>);
      message.success('拓扑评分计算完成');
    } catch {
      message.error('评分计算失败，请稍后重试');
    }
  }, [scoreJobId, scoreCandidateNodes, calculateScore, message]);

  // 评分图表 option
  const scoreChartOption = useMemo(() => {
    if (!scoreResults) return null;
    const entries = Object.entries(scoreResults).sort((a, b) => b[1] - a[1]);
    return {
      tooltip: { trigger: 'axis' as const, formatter: '{b}: {c} 分' },
      grid: { left: 80, right: 30, top: 20, bottom: 40 },
      xAxis: {
        type: 'value' as const,
        max: 100,
        axisLabel: { color: colors.text3 },
        splitLine: { lineStyle: { color: colors.line } },
      },
      yAxis: {
        type: 'category' as const,
        data: entries.map(([name]) => name),
        axisLabel: { color: colors.text3 },
      },
      series: [
        {
          type: 'bar' as const,
          data: entries.map(([, score]) => ({
            value: score,
            itemStyle: {
              color: score >= 80 ? chartPalette.teal : score >= 50 ? chartPalette.gpu : '#faad14',
              borderRadius: [0, 4, 4, 0],
            },
          })),
          barWidth: 20,
          label: { show: true, position: 'right' as const, color: colors.text3, formatter: '{c}' },
        },
      ],
    };
  }, [scoreResults, colors]);

  // 列定义
  const columns = useMemo<ColumnsType<NodeTopology>>(
    () => [
      { title: 'ID', dataIndex: 'id', key: 'id', width: 70, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
      {
        title: '节点名',
        dataIndex: 'node_name',
        key: 'node_name',
        width: 140,
        render: (v: string, record) => (
          <Button type="link" size="small" style={{ padding: 0 }} onClick={() => setDetailTopology(record)}>
            {v}
          </Button>
        ),
      },
      {
        title: '集群',
        dataIndex: 'cluster_id',
        key: 'cluster_id',
        width: 120,
        render: (v: number) => clusterName(v),
      },
      { title: '机架ID', dataIndex: 'rack_id', key: 'rack_id', width: 100 },
      { title: '交换机ID', dataIndex: 'switch_id', key: 'switch_id', width: 110 },
      { title: 'Pod ID', dataIndex: 'pod_id', key: 'pod_id', width: 100 },
      {
        title: '网络类型',
        dataIndex: 'network_type',
        key: 'network_type',
        width: 100,
        render: (v: string) => {
          const opt = NETWORK_TYPE_OPTIONS.find((o) => o.value === v);
          return opt ? <Tag color="blue">{opt.label}</Tag> : '-';
        },
      },
      {
        title: '带宽(Gbps)',
        dataIndex: 'network_bandwidth_gbps',
        key: 'network_bandwidth_gbps',
        width: 110,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: 'RDMA',
        dataIndex: 'rdma_enabled',
        key: 'rdma_enabled',
        width: 80,
        render: (v: boolean) => (v ? <Tag color="green">开启</Tag> : <Tag>关闭</Tag>),
      },
      { title: '故障域', dataIndex: 'fault_domain', key: 'fault_domain', width: 110 },
      {
        title: '操作',
        key: 'action',
        width: 180,
        fixed: 'right',
        render: (_: unknown, record: NodeTopology) => (
          <Space size={4}>
            <Can perm="topology:write">
              <Button type="link" size="small" onClick={() => handleOpenEdit(record)}>
                编辑
              </Button>
            </Can>
            <Can perm="topology:write">
              <Popconfirm
                title="删除该拓扑记录？"
                description="拓扑配置将被移除，且不可恢复。"
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
    [clusterName, handleOpenEdit, handleDelete],
  );

  const state = renderState({
    isLoading,
    error,
    isEmpty: topologiesData.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 8,
    empty: (
      <EmptyState
        title="暂无节点拓扑"
        description="登记节点拓扑信息（机架/交换机/NUMA/GPU NVLink），支持拓扑感知调度评分。"
        action={
          <Can perm="topology:write">
            <Button type="primary" onClick={handleOpenCreate}>
              新增拓扑
            </Button>
          </Can>
        }
      />
    ),
  });

  return (
    <div className="mc-page">
      <a href="#topology-table" className="mc-skip-link">
        跳转到拓扑列表
      </a>
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">拓扑管理</h1>
          <p className="mc-page-desc">共 {topologiesData.length} 条拓扑记录 · 机架/交换机/NUMA/GPU 层级管理与评分</p>
        </div>
        <div className="mc-page-head-extra">
          <Button onClick={() => setScoreModalVisible(true)} style={{ marginRight: 8 }}>
            拓扑评分工具
          </Button>
          <Can perm="topology:write">
            <Button type="primary" onClick={handleOpenCreate}>
              新增拓扑
            </Button>
          </Can>
        </div>
      </div>

      <Card id="topology-table">
        {/* 搜索 + 集群 / 网络类型筛选工具栏 */}
        <Row gutter={12} style={{ marginBottom: 16 }}>
          <Col flex="auto">
            <Input
              allowClear
              placeholder="搜索节点名 / 机架 / 交换机"
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
              placeholder="网络类型"
              style={{ width: 140 }}
              value={networkFilter || undefined}
              onChange={(v) => setNetworkFilter(v ?? '')}
              options={NETWORK_TYPE_OPTIONS}
            />
          </Col>
        </Row>
        {state ?? (
          <ResponsiveTable
            columns={columns}
            dataSource={filteredTopologies}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 1300, y: 520 }}
            virtual
          />
        )}
      </Card>

      {/* 新建/编辑 Modal */}
      <Modal
        className="mc-modal-full"
        title={editingTopology ? '编辑拓扑' : '新增拓扑'}
        open={isModalVisible}
        onCancel={() => setIsModalVisible(false)}
        footer={null}
        destroyOnHidden
        width={680}
      >
        <Form form={form} onFinish={handleSubmit} layout="vertical">
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="node_name" label="节点名" rules={[{ required: true, message: '请输入节点名' }]}>
                <Input placeholder="例如：gpu-node-01" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="cluster_id" label="集群">
                <Select
                  placeholder="请选择集群"
                  allowClear
                  options={clustersData.map((c: { id: number; name: string }) => ({ label: c.name, value: c.id }))}
                />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="rack_id" label="机架 ID">
                <Input placeholder="例如：rack-01" />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="switch_id" label="交换机 ID">
                <Input placeholder="例如：sw-01" />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="pod_id" label="Pod ID">
                <Input placeholder="例如：pod-a" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="network_type" label="网络类型">
                <Select placeholder="请选择网络类型" allowClear options={NETWORK_TYPE_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="network_bandwidth_gbps" label="带宽 (Gbps)">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="rdma_enabled" label="RDMA 开启" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="fault_domain" label="故障域">
                <Input placeholder="例如：rack-01" />
              </Form.Item>
            </Col>
            <Col span={24}>
              <Form.Item name="numa_nodes" label="NUMA 节点 (JSON)">
                <Input.TextArea rows={2} placeholder='例如：[{"node":0,"cpus":"0-31"},{"node":1,"cpus":"32-63"}]' />
              </Form.Item>
            </Col>
            <Col span={24}>
              <Form.Item name="gpu_topology" label="GPU 拓扑 (JSON)">
                <Input.TextArea rows={2} placeholder='例如：{"nvlink":[[0,1],[1,2]],"pcie":"x16"}' />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Can perm="topology:write">
                <Button type="primary" htmlType="submit">
                  {editingTopology ? '保存' : '创建'}
                </Button>
              </Can>
              <Button onClick={() => setIsModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* 详情 Drawer */}
      <Drawer
        title="节点拓扑详情"
        open={detailTopology !== null}
        onClose={() => setDetailTopology(null)}
        width={480}
      >
        {detailTopology && (
          <div>
            <DetailRow label="节点名" value={detailTopology.node_name} />
            <DetailRow label="集群" value={clusterName(detailTopology.cluster_id)} />
            <DetailRow label="机架 ID" value={detailTopology.rack_id ?? '-'} />
            <DetailRow label="交换机 ID" value={detailTopology.switch_id ?? '-'} />
            <DetailRow label="Pod ID" value={detailTopology.pod_id ?? '-'} />
            <DetailRow label="网络类型" value={detailTopology.network_type ?? '-'} />
            <DetailRow label="带宽" value={`${detailTopology.network_bandwidth_gbps ?? '-'} Gbps`} />
            <DetailRow label="RDMA" value={detailTopology.rdma_enabled ? '开启' : '关闭'} />
            <DetailRow label="故障域" value={detailTopology.fault_domain ?? '-'} />
            <DetailRow label="NUMA 节点" value={detailTopology.numa_nodes ?? '-'} />
            <DetailRow label="GPU 拓扑" value={detailTopology.gpu_topology ?? '-'} />
          </div>
        )}
      </Drawer>

      {/* 拓扑评分工具 Modal */}
      <Modal
        title="拓扑评分工具"
        open={scoreModalVisible}
        onCancel={() => {
          setScoreModalVisible(false);
          setScoreResults(null);
        }}
        footer={[
          <Button key="close" onClick={() => setScoreModalVisible(false)}>
            关闭
          </Button>,
          <Button key="calc" type="primary" onClick={handleCalculateScore}>
            计算评分
          </Button>,
        ]}
        width={720}
      >
        <Form layout="vertical">
          <Form.Item label="选择作业">
            <Select
              placeholder="请选择作业"
              value={scoreJobId}
              onChange={(v) => setScoreJobId(v)}
              style={{ width: '100%' }}
              options={jobsData.map((j: { id: number; name: string }) => ({ label: j.name, value: j.id }))}
            />
          </Form.Item>
          <Form.Item label="候选节点列表（逗号分隔）">
            <Input
              placeholder="例如：gpu-node-01, gpu-node-02, gpu-node-03"
              value={scoreCandidateNodes}
              onChange={(e) => setScoreCandidateNodes(e.target.value)}
            />
          </Form.Item>
        </Form>

        {scoreChartOption && (
          <div style={{ marginTop: 16 }}>
            <h3 style={{ marginBottom: 12 }}>拓扑评分结果</h3>
            <ResponsiveChart option={scoreChartOption} size="md" />
          </div>
        )}
      </Modal>
    </div>
  );
};

/** 详情行 */
const DetailRow: React.FC<{ label: string; value: React.ReactNode }> = ({ label, value }) => (
  <div style={{ display: 'flex', padding: '8px 0', borderBottom: '1px solid var(--mc-border)' }}>
    <span style={{ width: 110, color: 'var(--mc-text-3)', flexShrink: 0 }}>{label}</span>
    <span style={{ flex: 1, wordBreak: 'break-all' }}>{value}</span>
  </div>
);

export default TopologyManagement;
