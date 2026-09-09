import { Can } from '../components/Can';
import React, { memo, useCallback, useMemo, useState } from 'react';
import {
  Card,
  Row,
  Col,
  Statistic,
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
  Progress,
  Popconfirm,
  Drawer,
} from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import {
  useGetGPUDevicesQuery,
  useCreateGPUDeviceMutation,
  useUpdateGPUDeviceMutation,
  useDeleteGPUDeviceMutation,
  useGetGPUAllocationsQuery,
  useGetClustersQuery,
} from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { GPUDevice, GPUVendor } from '../types';

const VENDOR_OPTIONS: { label: string; value: GPUVendor }[] = [
  { label: 'NVIDIA', value: 'nvidia' },
  { label: '燧原 Enflame', value: 'enflame' },
  { label: '摩尔线程', value: 'moore_threads' },
  { label: '国产 X', value: 'domestic_x' },
];

const VENDOR_COLOR: Record<GPUVendor, string> = {
  nvidia: 'green',
  enflame: 'blue',
  moore_threads: 'purple',
  domestic_x: 'orange',
};

const VENDOR_LABEL: Record<GPUVendor, string> = {
  nvidia: 'NVIDIA',
  enflame: '燧原',
  moore_threads: '摩尔线程',
  domestic_x: '国产X',
};

/** 厂商标签：颜色 + 文字，memo 避免重渲染 */
const VendorTag: React.FC<{ vendor: GPUVendor }> = memo(({ vendor }) => (
  <Tag color={VENDOR_COLOR[vendor]}>{VENDOR_LABEL[vendor] ?? vendor}</Tag>
));

/** GPU 设备表单值 */
interface GPUFormValues {
  node_name: string;
  vendor: GPUVendor;
  model?: string;
  index?: number;
  total_memory_gb?: number;
  allocatable_memory_gb?: number;
  mig_enabled?: boolean;
  driver_version?: string;
  cuda_version?: string;
  status?: string;
  cluster_id?: number;
}

const GPUManagement: React.FC = () => {
  const { message } = App.useApp();

  // 筛选状态
  const [vendorFilter, setVendorFilter] = useState<string | undefined>(undefined);
  const [statusFilter, setStatusFilter] = useState<string | undefined>(undefined);
  const [clusterFilter, setClusterFilter] = useState<number | undefined>(undefined);

  // 详情抽屉
  const [detailDevice, setDetailDevice] = useState<GPUDevice | null>(null);

  // 表单
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [editingDevice, setEditingDevice] = useState<GPUDevice | null>(null);
  const [form] = Form.useForm<GPUFormValues>();

  // API
  const { data: devices, isLoading, error, refetch } = useGetGPUDevicesQuery({
    vendor: vendorFilter,
    status: statusFilter,
    cluster_id: clusterFilter,
  });
  const { data: allocations } = useGetGPUAllocationsQuery({});
  const { data: clusters } = useGetClustersQuery(undefined);
  const devicesData = devices ?? [];
  const allocationsData = allocations ?? [];
  const clustersData = extractArrayData(clusters);

  const [createGPUDevice] = useCreateGPUDeviceMutation();
  const [updateGPUDevice] = useUpdateGPUDeviceMutation();
  const [deleteGPUDevice] = useDeleteGPUDeviceMutation();

  // 统计卡片
  const stats = useMemo(() => {
    const total = devicesData.length;
    const allocated = devicesData.filter((d) => d.status === 'allocated').length;
    const available = devicesData.filter((d) => d.status === 'available').length;
    const avgUtil =
      total > 0
        ? Math.round(
            devicesData.reduce((s, d) => s + (d.utilization ?? 0), 0) / total,
          )
        : 0;
    const totalMem = devicesData.reduce((s, d) => s + (d.total_memory_gb ?? 0), 0);
    const usedMem = devicesData.reduce((s, d) => s + (d.used_memory_gb ?? 0), 0);
    return { total, allocated, available, avgUtil, totalMem, usedMem };
  }, [devicesData]);

  // 打开新建
  const handleOpenCreate = useCallback(() => {
    setEditingDevice(null);
    form.resetFields();
    setIsModalVisible(true);
  }, [form]);

  // 打开编辑
  const handleOpenEdit = useCallback(
    (record: GPUDevice) => {
      setEditingDevice(record);
      form.setFieldsValue({
        node_name: record.node_name,
        vendor: record.vendor,
        model: record.model,
        index: record.index,
        total_memory_gb: record.total_memory_gb,
        allocatable_memory_gb: record.allocatable_memory_gb,
        mig_enabled: record.mig_enabled,
        driver_version: record.driver_version,
        cuda_version: record.cuda_version,
        status: record.status,
        cluster_id: record.cluster_id,
      });
      setIsModalVisible(true);
    },
    [form],
  );

  // 提交表单
  const handleSubmit = useCallback(
    async (values: GPUFormValues) => {
      try {
        if (editingDevice) {
          await updateGPUDevice({ id: editingDevice.id, data: values as Partial<GPUDevice> }).unwrap();
          message.success('GPU 设备更新成功');
        } else {
          await createGPUDevice(values as Partial<GPUDevice>).unwrap();
          message.success('GPU 设备创建成功');
        }
        setIsModalVisible(false);
        form.resetFields();
        refetch();
      } catch {
        message.error('操作失败，请检查必填项后重试');
      }
    },
    [editingDevice, createGPUDevice, updateGPUDevice, form, message, refetch],
  );

  // 删除
  const handleDelete = useCallback(
    async (id: number) => {
      try {
        await deleteGPUDevice(id).unwrap();
        message.success('GPU 设备删除成功');
        refetch();
      } catch {
        message.error('删除失败，请稍后重试');
      }
    },
    [deleteGPUDevice, message, refetch],
  );

  // 设为维护模式
  const handleSetMaintenance = useCallback(
    async (record: GPUDevice) => {
      try {
        await updateGPUDevice({
          id: record.id,
          data: { status: 'maintenance' },
        }).unwrap();
        message.success('已设为维护模式');
        refetch();
      } catch {
        message.error('操作失败，请稍后重试');
      }
    },
    [updateGPUDevice, message, refetch],
  );

  // 设备的分配记录
  const deviceAllocations = useCallback(
    (deviceId: number) => allocationsData.filter((a) => a.device_id === deviceId),
    [allocationsData],
  );

  // 列定义
  const columns = useMemo<ColumnsType<GPUDevice>>(
    () => [
      {
        title: '节点名',
        dataIndex: 'node_name',
        key: 'node_name',
        width: 140,
        render: (v: string, record) => (
          <Button type="link" size="small" style={{ padding: 0 }} onClick={() => setDetailDevice(record)}>
            {v}
          </Button>
        ),
      },
      {
        title: '厂商',
        dataIndex: 'vendor',
        key: 'vendor',
        width: 100,
        render: (v: GPUVendor) => <VendorTag vendor={v} />,
      },
      { title: '型号', dataIndex: 'model', key: 'model', width: 100 },
      {
        title: '索引',
        dataIndex: 'index',
        key: 'index',
        width: 70,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: '总显存(GB)',
        dataIndex: 'total_memory_gb',
        key: 'total_memory_gb',
        width: 110,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: '已用显存(GB)',
        dataIndex: 'used_memory_gb',
        key: 'used_memory_gb',
        width: 110,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: 'MIG',
        dataIndex: 'mig_enabled',
        key: 'mig_enabled',
        width: 70,
        render: (v: boolean) => (v ? <Tag color="blue">开启</Tag> : <Tag>关闭</Tag>),
      },
      { title: '驱动版本', dataIndex: 'driver_version', key: 'driver_version', width: 100 },
      { title: 'CUDA版本', dataIndex: 'cuda_version', key: 'cuda_version', width: 100 },
      {
        title: '状态',
        dataIndex: 'status',
        key: 'status',
        width: 100,
        render: (status: string) => <StatusCell status={status} />,
      },
      {
        title: '利用率',
        dataIndex: 'utilization',
        key: 'utilization',
        width: 130,
        render: (v: number) => <Progress percent={v ?? 0} size="small" />,
      },
      {
        title: '温度(°C)',
        dataIndex: 'temperature',
        key: 'temperature',
        width: 90,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: '功耗(W)',
        dataIndex: 'power_draw',
        key: 'power_draw',
        width: 90,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: '操作',
        key: 'action',
        width: 220,
        fixed: 'right',
        render: (_: unknown, record: GPUDevice) => (
          <Space size={4}>
            <Can perm="gpu:write">
              <Button type="link" size="small" onClick={() => handleOpenEdit(record)}>
                编辑
              </Button>
            </Can>
            <Can perm="gpu:write">
              <Button type="link" size="small" onClick={() => handleSetMaintenance(record)}>
                维护
              </Button>
            </Can>
            <Can perm="gpu:write">
              <Popconfirm
                title="删除该 GPU 设备？"
                description="设备记录将被移除，且不可恢复。"
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
    [handleOpenEdit, handleSetMaintenance, handleDelete],
  );

  // 展开行：分配记录
  const expandedRowRender = useCallback(
    (record: GPUDevice) => {
      const allocs = deviceAllocations(record.id);
      if (allocs.length === 0) {
        return <div style={{ padding: '8px 16px', color: 'var(--mc-text-3)' }}>暂无分配记录</div>;
      }
      return (
        <div style={{ padding: '8px 16px' }}>
          <ResponsiveTable
            columns={[
              { title: '分配ID', dataIndex: 'id', key: 'id', width: 80 },
              { title: '作业ID', dataIndex: 'job_id', key: 'job_id', width: 90 },
              { title: '租户ID', dataIndex: 'tenant_id', key: 'tenant_id', width: 90 },
              { title: '用户ID', dataIndex: 'user_id', key: 'user_id', width: 90 },
              {
                title: '分数',
                dataIndex: 'fraction',
                key: 'fraction',
                width: 80,
                render: (v: number) => <Tag color="blue">{v}</Tag>,
              },
              { title: '显存(GB)', dataIndex: 'memory_gb', key: 'memory_gb', width: 100 },
              {
                title: '状态',
                dataIndex: 'status',
                key: 'status',
                width: 100,
                render: (s: string) => <StatusCell status={s} />,
              },
              { title: '开始时间', dataIndex: 'started_at', key: 'started_at', width: 170 },
            ]}
            dataSource={allocs}
            rowKey="id"
            pagination={false}
            size="small"
          />
        </div>
      );
    },
    [deviceAllocations],
  );

  const state = renderState({
    isLoading,
    error,
    isEmpty: devicesData.length === 0,
    onRetry: refetch,
    skeletonRows: 6,
    skeletonColumns: 10,
    empty: (
      <EmptyState
        title="暂无 GPU 设备"
        description="登记 GPU 设备后，可在此进行细粒度分配、显存管理与状态监控。"
        action={
          <Can perm="gpu:write">
            <Button type="primary" onClick={handleOpenCreate}>
              新增 GPU 设备
            </Button>
          </Can>
        }
      />
    ),
  });

  return (
    <div className="mc-page">
      <a href="#gpu-table" className="mc-skip-link">
        跳转到 GPU 设备列表
      </a>
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">GPU 细粒度管理</h1>
          <p className="mc-page-desc">
            共 {stats.total} 张 GPU · 已分配 {stats.allocated} · 可用 {stats.available} · 平均利用率 {stats.avgUtil}%
          </p>
        </div>
        <div className="mc-page-head-extra">
          <Can perm="gpu:write">
            <Button type="primary" onClick={handleOpenCreate}>
              新增 GPU 设备
            </Button>
          </Can>
        </div>
      </div>

      {/* 统计卡片 */}
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        <Col xs={12} sm={12} md={6} lg={6} xl={4}>
          <Card>
            <Statistic title="总 GPU 数" value={stats.total} suffix="张" />
          </Card>
        </Col>
        <Col xs={12} sm={12} md={6} lg={6} xl={4}>
          <Card>
            <Statistic title="已分配" value={stats.allocated} suffix="张" valueStyle={{ color: '#faad14' }} />
          </Card>
        </Col>
        <Col xs={12} sm={12} md={6} lg={6} xl={4}>
          <Card>
            <Statistic title="可用" value={stats.available} suffix="张" valueStyle={{ color: '#52c41a' }} />
          </Card>
        </Col>
        <Col xs={12} sm={12} md={6} lg={6} xl={4}>
          <Card>
            <Statistic title="平均利用率" value={stats.avgUtil} suffix="%" />
          </Card>
        </Col>
        <Col xs={12} sm={12} md={6} lg={6} xl={4}>
          <Card>
            <Statistic title="总显存" value={stats.totalMem} suffix="GB" />
          </Card>
        </Col>
        <Col xs={12} sm={12} md={6} lg={6} xl={4}>
          <Card>
            <Statistic title="已用显存" value={stats.usedMem} suffix="GB" valueStyle={{ color: '#faad14' }} />
          </Card>
        </Col>
      </Row>

      {/* 筛选器 */}
      <Card style={{ marginBottom: 16 }}>
        <Space wrap>
          <Select
            placeholder="厂商筛选"
            allowClear
            style={{ width: 160 }}
            value={vendorFilter}
            onChange={(v) => setVendorFilter(v)}
            options={VENDOR_OPTIONS}
          />
          <Select
            placeholder="状态筛选"
            allowClear
            style={{ width: 140 }}
            value={statusFilter}
            onChange={(v) => setStatusFilter(v)}
            options={[
              { label: '可用', value: 'available' },
              { label: '已分配', value: 'allocated' },
              { label: '维护中', value: 'maintenance' },
              { label: '故障', value: 'fault' },
            ]}
          />
          <Select
            placeholder="集群筛选"
            allowClear
            style={{ width: 180 }}
            value={clusterFilter}
            onChange={(v) => setClusterFilter(v)}
            options={clustersData.map((c: { id: number; name: string }) => ({ label: c.name, value: c.id }))}
          />
        </Space>
      </Card>

      {/* 设备表格 */}
      <Card id="gpu-table">
        {state ?? (
          <ResponsiveTable
            columns={columns}
            dataSource={devicesData}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 1600, y: 520 }}
            expandable={{ expandedRowRender }}
            virtual
          />
        )}
      </Card>

      {/* 新建/编辑 Modal */}
      <Modal
        className="mc-modal-full"
        title={editingDevice ? '编辑 GPU 设备' : '新增 GPU 设备'}
        open={isModalVisible}
        onCancel={() => setIsModalVisible(false)}
        footer={null}
        destroyOnHidden
      >
        <Form form={form} onFinish={handleSubmit} layout="vertical">
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="node_name" label="节点名" rules={[{ required: true, message: '请输入节点名' }]}>
                <Input placeholder="例如：gpu-node-01" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="vendor" label="厂商" rules={[{ required: true, message: '请选择厂商' }]}>
                <Select placeholder="请选择厂商" options={VENDOR_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="model" label="型号">
                <Input placeholder="例如：A100-80GB" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="index" label="GPU 索引">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="total_memory_gb" label="总显存(GB)">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="allocatable_memory_gb" label="可分配显存(GB)">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="driver_version" label="驱动版本">
                <Input placeholder="例如：535.129.03" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="cuda_version" label="CUDA 版本">
                <Input placeholder="例如：12.2" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="cluster_id" label="所属集群">
                <Select
                  placeholder="请选择集群"
                  allowClear
                  options={clustersData.map((c: { id: number; name: string }) => ({ label: c.name, value: c.id }))}
                />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="status" label="状态">
                <Select
                  placeholder="请选择状态"
                  options={[
                    { label: '可用', value: 'available' },
                    { label: '已分配', value: 'allocated' },
                    { label: '维护中', value: 'maintenance' },
                    { label: '故障', value: 'fault' },
                  ]}
                />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="mig_enabled" label="MIG 开启" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Can perm="gpu:write">
                <Button type="primary" htmlType="submit">
                  {editingDevice ? '保存' : '创建'}
                </Button>
              </Can>
              <Button onClick={() => setIsModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* 详情抽屉 */}
      <Drawer
        title="GPU 设备详情"
        open={detailDevice !== null}
        onClose={() => setDetailDevice(null)}
        width={480}
      >
        {detailDevice && (
          <div>
            <DescriptionsLike label="节点名" value={detailDevice.node_name} />
            <DescriptionsLike label="厂商" value={<VendorTag vendor={detailDevice.vendor} />} />
            <DescriptionsLike label="型号" value={detailDevice.model ?? '-'} />
            <DescriptionsLike label="索引" value={detailDevice.index ?? '-'} />
            <DescriptionsLike label="总显存" value={`${detailDevice.total_memory_gb ?? '-'} GB`} />
            <DescriptionsLike label="已用显存" value={`${detailDevice.used_memory_gb ?? '-'} GB`} />
            <DescriptionsLike label="可分配显存" value={`${detailDevice.allocatable_memory_gb ?? '-'} GB`} />
            <DescriptionsLike label="MIG" value={detailDevice.mig_enabled ? '开启' : '关闭'} />
            <DescriptionsLike label="驱动版本" value={detailDevice.driver_version ?? '-'} />
            <DescriptionsLike label="CUDA 版本" value={detailDevice.cuda_version ?? '-'} />
            <DescriptionsLike label="状态" value={<StatusCell status={detailDevice.status ?? ''} />} />
            <DescriptionsLike label="利用率" value={`${detailDevice.utilization ?? 0}%`} />
            <DescriptionsLike label="温度" value={`${detailDevice.temperature ?? '-'} °C`} />
            <DescriptionsLike label="功耗" value={`${detailDevice.power_draw ?? '-'} W`} />
            <DescriptionsLike label="MIG Profiles" value={detailDevice.mig_profiles ?? '-'} />
            <DescriptionsLike label="详情" value={detailDevice.details ?? '-'} />
          </div>
        )}
      </Drawer>
    </div>
  );
};

/** 简易描述行（Drawer 内复用） */
const DescriptionsLike: React.FC<{ label: string; value: React.ReactNode }> = ({ label, value }) => (
  <div style={{ display: 'flex', padding: '8px 0', borderBottom: '1px solid var(--mc-border)' }}>
    <span style={{ width: 120, color: 'var(--mc-text-3)', flexShrink: 0 }}>{label}</span>
    <span style={{ flex: 1 }}>{value}</span>
  </div>
);

export default GPUManagement;
