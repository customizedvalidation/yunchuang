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
  Progress,
} from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import ResponsiveChart from '../components/ResponsiveChart';
import type { ColumnsType } from 'antd/es/table';
import {
  useGetDatasetsQuery,
  useCreateDatasetMutation,
  useUpdateDatasetMutation,
  useDeleteDatasetMutation,
  useGetFluidCachesQuery,
  useCreateFluidCacheMutation,
  useUpdateFluidCacheMutation,
  useDeleteFluidCacheMutation,
  useEnableFluidCacheMutation,
  useDisableFluidCacheMutation,
  useTriggerPrefetchMutation,
} from '../store/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { Dataset, FluidCache } from '../types';
import { useThemeMode } from '../theme/ThemeModeContext';
import { chartPalette, getNeutral } from '../theme/tokens';

const SOURCE_TYPE_OPTIONS = [
  { label: 'Ceph', value: 'ceph' },
  { label: 'NFS', value: 'nfs' },
  { label: 'S3', value: 's3' },
  { label: 'GlusterFS', value: 'glusterfs' },
  { label: 'Lustre', value: 'lustre' },
  { label: 'BeeGFS', value: 'beegfs' },
];

const RUNTIME_TYPE_OPTIONS = [
  { label: 'Alluxio', value: 'alluxio' },
  { label: 'JindoFS', value: 'jindofs' },
];

const MEDIUM_TYPE_OPTIONS = [
  { label: '内存', value: 'memory' },
  { label: '磁盘', value: 'disk' },
  { label: '混合', value: 'mixed' },
];

/** 数据集表单值 */
interface DatasetFormValues {
  name: string;
  description?: string;
  source_type: 'ceph' | 'nfs' | 's3' | 'glusterfs' | 'lustre' | 'beegfs';
  source_path?: string;
  mount_path?: string;
  size_gb?: number;
  mount_options?: string;
  access_mode?: string;
  status?: string;
  tenant_id?: number;
}

/** Fluid 缓存表单值 */
interface FluidCacheFormValues {
  runtime_type?: 'alluxio' | 'jindofs';
  cache_capacity_gb?: number;
  replicas?: number;
  medium_type?: 'memory' | 'disk' | 'mixed';
  prefetch_enabled?: boolean;
  prefetch_policy?: string;
  compression_enabled?: boolean;
  metadata_acceleration_enabled?: boolean;
}

const DatasetManagement: React.FC = () => {
  const { message } = App.useApp();
  const { mode } = useThemeMode();
  const colors = useMemo(() => getNeutral(mode), [mode]);

  // 数据集表单
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [editingDataset, setEditingDataset] = useState<Dataset | null>(null);
  const [form] = Form.useForm<DatasetFormValues>();

  // Fluid 缓存 Drawer
  const [cacheDrawerVisible, setCacheDrawerVisible] = useState(false);
  const [cacheTarget, setCacheTarget] = useState<Dataset | null>(null);

  // 新增缓存 Modal
  const [cacheModalVisible, setCacheModalVisible] = useState(false);
  const [editingCache, setEditingCache] = useState<FluidCache | null>(null);
  const [cacheForm] = Form.useForm<FluidCacheFormValues>();

  // API
  const { data: datasets, isLoading, error, refetch } = useGetDatasetsQuery();
  const datasetsData = datasets ?? [];

  const { data: fluidCaches } = useGetFluidCachesQuery(cacheTarget?.id ?? 0, {
    skip: !cacheTarget,
  });
  const cachesData = fluidCaches ?? [];

  const [createDataset] = useCreateDatasetMutation();
  const [updateDataset] = useUpdateDatasetMutation();
  const [deleteDataset] = useDeleteDatasetMutation();
  const [createFluidCache] = useCreateFluidCacheMutation();
  const [updateFluidCache] = useUpdateFluidCacheMutation();
  const [deleteFluidCache] = useDeleteFluidCacheMutation();
  const [enableCache] = useEnableFluidCacheMutation();
  const [disableCache] = useDisableFluidCacheMutation();
  const [triggerPrefetch] = useTriggerPrefetchMutation();

  // 打开新建数据集
  const handleOpenCreate = useCallback(() => {
    setEditingDataset(null);
    form.resetFields();
    setIsModalVisible(true);
  }, [form]);

  // 打开编辑数据集
  const handleOpenEdit = useCallback(
    (record: Dataset) => {
      setEditingDataset(record);
      form.setFieldsValue({
        name: record.name,
        description: record.description,
        source_type: record.source_type,
        source_path: record.source_path,
        mount_path: record.mount_path,
        size_gb: record.size_gb,
        mount_options: record.mount_options,
        access_mode: record.access_mode,
        status: record.status,
        tenant_id: record.tenant_id,
      });
      setIsModalVisible(true);
    },
    [form],
  );

  // 提交数据集
  const handleSubmit = useCallback(
    async (values: DatasetFormValues) => {
      try {
        if (editingDataset) {
          await updateDataset({ id: editingDataset.id, data: values }).unwrap();
          message.success('数据集更新成功');
        } else {
          await createDataset(values).unwrap();
          message.success('数据集创建成功');
        }
        setIsModalVisible(false);
        form.resetFields();
        refetch();
      } catch {
        message.error('操作失败，请检查必填项后重试');
      }
    },
    [editingDataset, createDataset, updateDataset, form, message, refetch],
  );

  // 删除数据集
  const handleDelete = useCallback(
    async (id: number) => {
      try {
        await deleteDataset(id).unwrap();
        message.success('数据集删除成功');
        refetch();
      } catch {
        message.error('删除失败，请稍后重试');
      }
    },
    [deleteDataset, message, refetch],
  );

  // 打开缓存管理
  const handleOpenCache = useCallback((record: Dataset) => {
    setCacheTarget(record);
    setCacheDrawerVisible(true);
  }, []);

  // 打开新建缓存
  const handleOpenCreateCache = useCallback(() => {
    setEditingCache(null);
    cacheForm.resetFields();
    setCacheModalVisible(true);
  }, [cacheForm]);

  // 打开编辑缓存
  const handleOpenEditCache = useCallback(
    (record: FluidCache) => {
      setEditingCache(record);
      cacheForm.setFieldsValue({
        runtime_type: record.runtime_type,
        cache_capacity_gb: record.cache_capacity_gb,
        replicas: record.replicas,
        medium_type: record.medium_type,
        prefetch_enabled: record.prefetch_enabled,
        prefetch_policy: record.prefetch_policy,
        compression_enabled: record.compression_enabled,
        metadata_acceleration_enabled: record.metadata_acceleration_enabled,
      });
      setCacheModalVisible(true);
    },
    [cacheForm],
  );

  // 提交缓存
  const handleSubmitCache = useCallback(
    async (values: FluidCacheFormValues) => {
      if (!cacheTarget) return;
      try {
        if (editingCache) {
          await updateFluidCache({ cacheId: editingCache.id, data: values }).unwrap();
          message.success('缓存更新成功');
        } else {
          await createFluidCache({ dataset_id: cacheTarget.id, data: values }).unwrap();
          message.success('缓存创建成功');
        }
        setCacheModalVisible(false);
        cacheForm.resetFields();
      } catch {
        message.error('操作失败，请检查必填项后重试');
      }
    },
    [cacheTarget, editingCache, createFluidCache, updateFluidCache, cacheForm, message],
  );

  // 删除缓存
  const handleDeleteCache = useCallback(
    async (id: number) => {
      try {
        await deleteFluidCache(id).unwrap();
        message.success('缓存删除成功');
      } catch {
        message.error('删除失败，请稍后重试');
      }
    },
    [deleteFluidCache, message],
  );

  // 启用/禁用缓存
  const handleEnableCache = useCallback(
    async (id: number) => {
      try {
        await enableCache(id).unwrap();
        message.success('缓存已启用');
      } catch {
        message.error('操作失败，请稍后重试');
      }
    },
    [enableCache, message],
  );

  const handleDisableCache = useCallback(
    async (id: number) => {
      try {
        await disableCache(id).unwrap();
        message.success('缓存已禁用');
      } catch {
        message.error('操作失败，请稍后重试');
      }
    },
    [disableCache, message],
  );

  // 触发预取
  const handleTriggerPrefetch = useCallback(
    async (id: number) => {
      try {
        await triggerPrefetch(id).unwrap();
        message.success('预取已触发');
      } catch {
        message.error('触发失败，请稍后重试');
      }
    },
    [triggerPrefetch, message],
  );

  // 缓存命中率趋势图（模拟数据）
  const cacheHitRateOption = useMemo(
    () => ({
      tooltip: { trigger: 'axis' as const },
      legend: { bottom: 0, textStyle: { color: colors.text3 } },
      grid: { left: 50, right: 20, top: 20, bottom: 40 },
      xAxis: {
        type: 'category' as const,
        data: ['00:00', '04:00', '08:00', '12:00', '16:00', '20:00', '24:00'],
        axisLabel: { color: colors.text3 },
        axisLine: { lineStyle: { color: colors.line } },
      },
      yAxis: {
        type: 'value' as const,
        max: 100,
        axisLabel: { color: colors.text3, formatter: '{value}%' },
        splitLine: { lineStyle: { color: colors.line } },
      },
      series: [
        {
          name: '缓存命中率',
          type: 'line' as const,
          smooth: true,
          data: [65, 72, 78, 85, 88, 82, 79],
          itemStyle: { color: chartPalette.teal },
          areaStyle: { color: 'rgba(0,184,169,0.15)' },
        },
      ],
    }),
    [colors],
  );

  // 数据集列定义
  const columns = useMemo<ColumnsType<Dataset>>(
    () => [
      { title: 'ID', dataIndex: 'id', key: 'id', width: 70, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
      { title: '名称', dataIndex: 'name', key: 'name', width: 140 },
      { title: '描述', dataIndex: 'description', key: 'description', ellipsis: true },
      {
        title: '源类型',
        dataIndex: 'source_type',
        key: 'source_type',
        width: 110,
        render: (v: string) => {
          const opt = SOURCE_TYPE_OPTIONS.find((o) => o.value === v);
          return <Tag color="blue">{opt?.label ?? v}</Tag>;
        },
      },
      { title: '源路径', dataIndex: 'source_path', key: 'source_path', width: 180, ellipsis: true },
      { title: '挂载路径', dataIndex: 'mount_path', key: 'mount_path', width: 160, ellipsis: true },
      {
        title: '大小(GB)',
        dataIndex: 'size_gb',
        key: 'size_gb',
        width: 100,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      { title: '访问模式', dataIndex: 'access_mode', key: 'access_mode', width: 100 },
      {
        title: '状态',
        dataIndex: 'status',
        key: 'status',
        width: 100,
        render: (status: string) => <StatusCell status={status} />,
      },
      {
        title: '操作',
        key: 'action',
        width: 200,
        fixed: 'right',
        render: (_: unknown, record: Dataset) => (
          <Space size={4}>
            <Can perm="dataset:write">
              <Button type="link" size="small" onClick={() => handleOpenEdit(record)}>
                编辑
              </Button>
            </Can>
            <Button type="link" size="small" onClick={() => handleOpenCache(record)}>
              管理缓存
            </Button>
            <Can perm="dataset:write">
              <Popconfirm
                title="删除该数据集？"
                description="数据集配置与关联缓存将一并移除，且不可恢复。"
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
    [handleOpenEdit, handleOpenCache, handleDelete],
  );

  // 缓存列表列定义
  const cacheColumns = useMemo<ColumnsType<FluidCache>>(
    () => [
      { title: 'ID', dataIndex: 'id', key: 'id', width: 70 },
      {
        title: '运行时',
        dataIndex: 'runtime_type',
        key: 'runtime_type',
        width: 100,
        render: (v: string) => {
          const opt = RUNTIME_TYPE_OPTIONS.find((o) => o.value === v);
          return <Tag color="purple">{opt?.label ?? v}</Tag>;
        },
      },
      {
        title: '缓存容量(GB)',
        dataIndex: 'cache_capacity_gb',
        key: 'cache_capacity_gb',
        width: 110,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: '副本数',
        dataIndex: 'replicas',
        key: 'replicas',
        width: 80,
        render: (v: React.ReactNode) => <span className="mc-num">{v}</span>,
      },
      {
        title: '介质类型',
        dataIndex: 'medium_type',
        key: 'medium_type',
        width: 90,
        render: (v: string) => {
          const opt = MEDIUM_TYPE_OPTIONS.find((o) => o.value === v);
          return opt?.label ?? v ?? '-';
        },
      },
      {
        title: '预取',
        dataIndex: 'prefetch_enabled',
        key: 'prefetch_enabled',
        width: 70,
        render: (v: boolean) => (v ? <Tag color="green">开</Tag> : <Tag>关</Tag>),
      },
      {
        title: '压缩',
        dataIndex: 'compression_enabled',
        key: 'compression_enabled',
        width: 70,
        render: (v: boolean) => (v ? <Tag color="green">开</Tag> : <Tag>关</Tag>),
      },
      {
        title: '元数据加速',
        dataIndex: 'metadata_acceleration_enabled',
        key: 'metadata_acceleration_enabled',
        width: 100,
        render: (v: boolean) => (v ? <Tag color="green">开</Tag> : <Tag>关</Tag>),
      },
      {
        title: '状态',
        dataIndex: 'status',
        key: 'status',
        width: 90,
        render: (s: string) => <StatusCell status={s} />,
      },
      {
        title: '缓存命中率',
        dataIndex: 'cache_hit_rate',
        key: 'cache_hit_rate',
        width: 130,
        render: (v: number) => <Progress percent={v ?? 0} size="small" />,
      },
      {
        title: '操作',
        key: 'action',
        width: 260,
        fixed: 'right',
        render: (_: unknown, record: FluidCache) => (
          <Space size={4} wrap>
            <Can perm="dataset:write">
              <Button type="link" size="small" onClick={() => handleOpenEditCache(record)}>
                编辑
              </Button>
            </Can>
            <Can perm="dataset:write">
              {record.status === 'active' ? (
                <Button type="link" size="small" onClick={() => handleDisableCache(record.id)}>
                  禁用
                </Button>
              ) : (
                <Button type="link" size="small" onClick={() => handleEnableCache(record.id)}>
                  启用
                </Button>
              )}
            </Can>
            <Can perm="dataset:write">
              <Button type="link" size="small" onClick={() => handleTriggerPrefetch(record.id)}>
                触发预取
              </Button>
            </Can>
            <Can perm="dataset:write">
              <Popconfirm
                title="删除该缓存？"
                onConfirm={() => handleDeleteCache(record.id)}
                okText="确认"
                cancelText="取消"
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
    [handleOpenEditCache, handleEnableCache, handleDisableCache, handleTriggerPrefetch, handleDeleteCache],
  );

  const state = renderState({
    isLoading,
    error,
    isEmpty: datasetsData.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 8,
    empty: (
      <EmptyState
        title="暂无数据集"
        description="登记数据集（Ceph/NFS/S3 等），配置 Fluid 分布式缓存加速训练数据读取。"
        action={
          <Can perm="dataset:write">
            <Button type="primary" onClick={handleOpenCreate}>
              新增数据集
            </Button>
          </Can>
        }
      />
    ),
  });

  return (
    <div className="mc-page">
      <a href="#dataset-table" className="mc-skip-link">
        跳转到数据集列表
      </a>
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">数据集与 Fluid 缓存管理</h1>
          <p className="mc-page-desc">共 {datasetsData.length} 个数据集 · 分布式缓存加速训练数据读取</p>
        </div>
        <div className="mc-page-head-extra">
          <Can perm="dataset:write">
            <Button type="primary" onClick={handleOpenCreate}>
              新增数据集
            </Button>
          </Can>
        </div>
      </div>

      <Card id="dataset-table">
        {state ?? (
          <ResponsiveTable
            columns={columns}
            dataSource={datasetsData}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 1400, y: 520 }}
            virtual
          />
        )}
      </Card>

      {/* 新建/编辑数据集 Modal */}
      <Modal
        className="mc-modal-full"
        title={editingDataset ? '编辑数据集' : '新增数据集'}
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
                <Input placeholder="例如：imagenet-1k" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="source_type" label="源类型" rules={[{ required: true, message: '请选择源类型' }]}>
                <Select placeholder="请选择源类型" options={SOURCE_TYPE_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={24}>
              <Form.Item name="description" label="描述">
                <Input.TextArea rows={2} placeholder="选填" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="source_path" label="源路径">
                <Input placeholder="例如：ceph://data/imagenet" />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="mount_path" label="挂载路径">
                <Input placeholder="例如：/mnt/data/imagenet" />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="size_gb" label="大小 (GB)">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="access_mode" label="访问模式">
                <Input placeholder="例如：ReadWriteMany" />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="status" label="状态">
                <Select
                  placeholder="请选择状态"
                  options={[
                    { label: '活跃', value: 'active' },
                    { label: '非活跃', value: 'inactive' },
                    { label: '维护', value: 'maintenance' },
                  ]}
                />
              </Form.Item>
            </Col>
            <Col span={24}>
              <Form.Item name="mount_options" label="挂载选项">
                <Input placeholder="例如：vers=4.1,rsize=1048576" />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Can perm="dataset:write">
                <Button type="primary" htmlType="submit">
                  {editingDataset ? '保存' : '创建'}
                </Button>
              </Can>
              <Button onClick={() => setIsModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* Fluid 缓存管理 Drawer */}
      <Drawer
        title={`Fluid 缓存管理 - ${cacheTarget?.name ?? ''}`}
        open={cacheDrawerVisible}
        onClose={() => setCacheDrawerVisible(false)}
        width={900}
      >
        <div style={{ marginBottom: 16 }}>
          <Can perm="dataset:write">
            <Button type="primary" onClick={handleOpenCreateCache}>
              新增缓存
            </Button>
          </Can>
        </div>

        <ResponsiveTable
          columns={cacheColumns}
          dataSource={cachesData}
          rowKey="id"
          pagination={false}
          size="small"
          scroll={{ x: 1300 }}
        />

        {/* 缓存命中率趋势图 */}
        <Card title="缓存命中率趋势（近24小时）" style={{ marginTop: 16 }}>
          <ResponsiveChart option={cacheHitRateOption} size="md" />
        </Card>
      </Drawer>

      {/* 新建/编辑缓存 Modal */}
      <Modal
        title={editingCache ? '编辑 Fluid 缓存' : '新增 Fluid 缓存'}
        open={cacheModalVisible}
        onCancel={() => setCacheModalVisible(false)}
        footer={null}
        destroyOnHidden
        width={600}
      >
        <Form form={cacheForm} onFinish={handleSubmitCache} layout="vertical">
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="runtime_type" label="运行时类型">
                <Select placeholder="请选择运行时" allowClear options={RUNTIME_TYPE_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="cache_capacity_gb" label="缓存容量 (GB)">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="replicas" label="副本数">
                <InputNumber min={1} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="medium_type" label="介质类型">
                <Select placeholder="请选择介质类型" allowClear options={MEDIUM_TYPE_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="prefetch_enabled" label="预取开启" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="compression_enabled" label="压缩开启" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="metadata_acceleration_enabled" label="元数据加速" valuePropName="checked">
                <Switch />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="prefetch_policy" label="预取策略">
                <Input placeholder="例如：aggressive" />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Can perm="dataset:write">
                <Button type="primary" htmlType="submit">
                  {editingCache ? '保存' : '创建'}
                </Button>
              </Can>
              <Button onClick={() => setCacheModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default DatasetManagement;
