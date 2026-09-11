import { Can } from '../components/Can';
import React, { useCallback, useMemo, useState } from 'react';
import { Card, Statistic, Drawer, Descriptions, Tag, Switch, App, Button, Space, Modal, Form, Input, Select, Row, Col } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import { useNavigate } from 'react-router-dom';
import { useGetAccelerationSuitesQuery, useUpdateAccelerationSuiteMutation } from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { AccelerationSuite, GPUVendor } from '../types';

const CATEGORY_OPTIONS = [
  { label: '数据加速', value: 'fluid_cache' },
  { label: '分布式训练', value: 'distributed_training' },
  { label: '推理加速', value: 'inference' },
  { label: '通信优化', value: 'communication' },
];

const VENDOR_OPTIONS: { label: string; value: GPUVendor }[] = [
  { label: 'NVIDIA', value: 'nvidia' },
  { label: '燧原', value: 'enflame' },
  { label: '摩尔线程', value: 'moore_threads' },
  { label: '国产 X', value: 'domestic_x' },
];

/** config_json 中常见键 → 友好中文名（skill.md 4.4 细分能力） */
const CONFIG_LABELS: Record<string, string> = {
  runtime: '运行时',
  cache_capacity_gb: '缓存容量(GB)',
  replicas: '副本数',
  medium_type: '缓存介质',
  prefetch_enabled: '数据预取',
  prefetch_policy: '预取策略',
  compression_enabled: '数据压缩',
  metadata_acceleration_enabled: '元数据加速',
  cache_hit_rate: '缓存命中率',
  parallel_strategy: '并行策略',
  world_size: '世界大小',
  tensor_parallel_size: '张量并行度',
  pipeline_stages: '流水线段数',
  deepspeed_enabled: 'DeepSpeed',
  megatron_enabled: 'Megatron-LM',
  communication_backend: '通信后端',
  nccl_config: 'NCCL 配置',
  gradient_compression: '梯度通信压缩',
  backend: '推理后端引擎',
  precision: '计算精度',
  quantization_enabled: '模型量化',
  batch_size: '批大小',
  dynamic_batching: '动态批处理',
  max_batch_size: '最大批大小',
  max_latency_ms: '最大时延(ms)',
  operator_fusion: '算子融合',
};

/** 加速套件表单值 */
interface SuiteFormValues {
  config_json?: string;
  category?: 'fluid_cache' | 'distributed_training' | 'inference' | 'communication';
  vendor?: GPUVendor;
}

/** 安全解析 config_json */
const parseConfig = (raw?: string): Record<string, unknown> => {
  if (!raw) return {};
  try {
    const v = JSON.parse(raw);
    return v && typeof v === 'object' ? (v as Record<string, unknown>) : {};
  } catch {
    return {};
  }
};

const AccelerationSuiteManagement: React.FC = () => {
  const { message } = App.useApp();
  const navigate = useNavigate();
  const { data: suites, isLoading, error, refetch } = useGetAccelerationSuitesQuery(undefined);
  const suitesData = useMemo(() => extractArrayData<AccelerationSuite>(suites), [suites]);
  const [updateSuite] = useUpdateAccelerationSuiteMutation();

  const [editModalVisible, setEditModalVisible] = useState(false);
  const [editingSuite, setEditingSuite] = useState<AccelerationSuite | null>(null);
  const [form] = Form.useForm<SuiteFormValues>();

  // 分类筛选
  const [categoryFilter, setCategoryFilter] = useState<string>('all');

  // 详情 Drawer
  const [detailVisible, setDetailVisible] = useState(false);
  const [detailSuite, setDetailSuite] = useState<AccelerationSuite | null>(null);

  const handleToggle = useCallback(
    async (id: number, enabled: boolean) => {
      try {
        await updateSuite({ id, enabled }).unwrap();
        message.success(`加速套件${enabled ? '启用' : '禁用'}成功`);
        refetch();
      } catch {
        message.error('操作失败，请稍后重试');
      }
    },
    [updateSuite, message, refetch],
  );

  const handleOpenEdit = useCallback(
    (record: AccelerationSuite) => {
      setEditingSuite(record);
      form.setFieldsValue({
        config_json: record.config_json,
        category: record.category,
        vendor: record.vendor,
      });
      setEditModalVisible(true);
    },
    [form],
  );

  const handleOpenDetail = useCallback((record: AccelerationSuite) => {
    setDetailSuite(record);
    setDetailVisible(true);
  }, []);

  const handleSaveEdit = useCallback(
    async (values: SuiteFormValues) => {
      if (!editingSuite) return;
      try {
        await updateSuite({ id: editingSuite.id, ...values }).unwrap();
        message.success('加速套件更新成功');
        setEditModalVisible(false);
        form.resetFields();
        refetch();
      } catch {
        message.error('更新失败，请稍后重试');
      }
    },
    [editingSuite, updateSuite, form, message, refetch],
  );

  // 分类统计
  const categoryStats = useMemo(() => {
    const stats: Record<string, { total: number; enabled: number }> = {};
    suitesData.forEach((s) => {
      const key = s.category ?? 'other';
      if (!stats[key]) stats[key] = { total: 0, enabled: 0 };
      stats[key].total += 1;
      if (s.enabled) stats[key].enabled += 1;
    });
    return stats;
  }, [suitesData]);

  const filteredSuites = useMemo(
    () => (categoryFilter === 'all' ? suitesData : suitesData.filter((s) => s.category === categoryFilter)),
    [suitesData, categoryFilter],
  );

  const columns: ColumnsType<AccelerationSuite> = useMemo(
    () => [
      { title: 'ID', dataIndex: 'id', key: 'id', width: 70, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
      { title: '名称', dataIndex: 'name', key: 'name' },
      { title: '类型', dataIndex: 'type', key: 'type', width: 90, render: (type: string) => <Tag>{type}</Tag> },
      {
        title: '类别',
        dataIndex: 'category',
        key: 'category',
        width: 110,
        render: (v: string) => {
          const opt = CATEGORY_OPTIONS.find((o) => o.value === v);
          return opt ? <Tag color="blue">{opt.label}</Tag> : '-';
        },
      },
      {
        title: '厂商',
        dataIndex: 'vendor',
        key: 'vendor',
        width: 90,
        render: (v: GPUVendor) => (v ? <Tag color="green">{v}</Tag> : '-'),
      },
      { title: '版本', dataIndex: 'version', key: 'version', width: 90 },
      { title: '状态', dataIndex: 'status', key: 'status', width: 90, render: (status: string) => <StatusCell status={status} /> },
      {
        title: '启用',
        key: 'enabled',
        width: 80,
        render: (_: unknown, record: AccelerationSuite) => (
          <Can perm="acceleration:write">
            <Switch checked={record.enabled} onChange={(checked) => handleToggle(record.id, checked)} />
          </Can>
        ),
      },
      {
        title: '操作',
        key: 'action',
        width: 140,
        render: (_: unknown, record: AccelerationSuite) => (
          <Space size={0}>
            <Button type="link" size="small" onClick={() => handleOpenDetail(record)}>
              详情
            </Button>
            <Can perm="acceleration:write">
              <Button type="link" size="small" onClick={() => handleOpenEdit(record)}>
                编辑
              </Button>
            </Can>
          </Space>
        ),
      },
    ],
    [handleToggle, handleOpenEdit, handleOpenDetail],
  );

  const state = renderState({
    isLoading,
    error,
    isEmpty: filteredSuites.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 7,
    empty: (
      <EmptyState
        title="暂无加速套件"
        description={categoryFilter === 'all' ? '还没有登记任何加速套件。' : '该分类下暂无加速套件。'}
      />
    ),
  });

  const detailConfig = detailSuite ? parseConfig(detailSuite.config_json) : {};
  const detailEntries = Object.entries(detailConfig);

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">加速套件管理</h1>
          <p className="mc-page-desc">
            共 {suitesData.length} 个加速套件 · 数据加速 / 分布式训练 / 推理加速 / 通信优化
          </p>
        </div>
        <div className="mc-page-head-extra">
          <Button onClick={() => navigate('/datasets')}>数据集与缓存</Button>
        </div>
      </div>

      {/* 分类汇总卡片 */}
      <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
        {[
          { key: 'fluid_cache', label: '数据加速', hint: 'Fluid 缓存 / 预取 / 压缩' },
          { key: 'distributed_training', label: '分布式训练', hint: 'NCCL / DeepSpeed / 并行' },
          { key: 'inference', label: '推理加速', hint: 'TensorRT / 量化 / 批处理' },
          { key: 'communication', label: '通信优化', hint: '梯度通信 / 通信策略' },
        ].map((c) => (
          <Col xs={12} sm={12} md={6} key={c.key}>
            <Card
              size="small"
              onClick={() => setCategoryFilter(categoryFilter === c.key ? 'all' : c.key)}
              style={{ cursor: 'pointer', borderColor: categoryFilter === c.key ? '#1677ff' : undefined }}
            >
              <Statistic
                title={c.label}
                value={categoryStats[c.key]?.total ?? 0}
                suffix={
                  <span style={{ fontSize: 12, color: 'var(--mc-text-3)' }}>
                    启用 {categoryStats[c.key]?.enabled ?? 0}
                  </span>
                }
              />
              <div style={{ fontSize: 12, color: 'var(--mc-text-3)', marginTop: 4 }}>{c.hint}</div>
            </Card>
          </Col>
        ))}
      </Row>

      <Card>
        {state ?? (
          <ResponsiveTable
            columns={columns}
            dataSource={filteredSuites}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 1080 }}
          />
        )}
      </Card>

      {/* 编辑 Modal */}
      <Modal
        title={`编辑加速套件 - ${editingSuite?.name ?? ''}`}
        open={editModalVisible}
        onCancel={() => setEditModalVisible(false)}
        footer={null}
        destroyOnHidden
        width={600}
      >
        <Form form={form} onFinish={handleSaveEdit} layout="vertical">
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="category" label="类别">
                <Select placeholder="请选择类别" allowClear options={CATEGORY_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="vendor" label="厂商">
                <Select placeholder="请选择厂商" allowClear options={VENDOR_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={24}>
              <Form.Item name="config_json" label="Config JSON">
                <Input.TextArea rows={6} placeholder='例如：{"runtime":"alluxio","cache_capacity_gb":100}' />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Can perm="acceleration:write">
                <Button type="primary" htmlType="submit">保存</Button>
              </Can>
              <Button onClick={() => setEditModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* 详情 Drawer：解析 config_json 展示细分能力 */}
      <Drawer
        title={`加速套件详情 - ${detailSuite?.name ?? ''}`}
        open={detailVisible}
        onClose={() => setDetailVisible(false)}
        width={560}
      >
        {detailSuite && (
          <Descriptions column={1} bordered size="small">
            <Descriptions.Item label="名称">{detailSuite.name}</Descriptions.Item>
            <Descriptions.Item label="类型">{detailSuite.type}</Descriptions.Item>
            <Descriptions.Item label="类别">
              {CATEGORY_OPTIONS.find((o) => o.value === detailSuite.category)?.label ?? detailSuite.category ?? '-'}
            </Descriptions.Item>
            <Descriptions.Item label="厂商">{detailSuite.vendor ?? '-'}</Descriptions.Item>
            <Descriptions.Item label="版本">{detailSuite.version ?? '-'}</Descriptions.Item>
            <Descriptions.Item label="状态">
              <StatusCell status={detailSuite.status ?? ''} />
            </Descriptions.Item>
            <Descriptions.Item label="描述">{detailSuite.description ?? '-'}</Descriptions.Item>
          </Descriptions>
        )}

        <Card size="small" title="细分能力配置" style={{ marginTop: 16 }}>
          {detailEntries.length > 0 ? (
            <Descriptions column={1} size="small">
              {detailEntries.map(([k, v]) => (
                <Descriptions.Item key={k} label={CONFIG_LABELS[k] ?? k}>
                  {typeof v === 'boolean' ? (v ? <Tag color="green">开</Tag> : <Tag>关</Tag>) : String(v)}
                </Descriptions.Item>
              ))}
            </Descriptions>
          ) : (
            <EmptyState title="未配置细分能力" description="该套件尚未填写 config_json 能力参数。" />
          )}
        </Card>
      </Drawer>
    </div>
  );
};

export default AccelerationSuiteManagement;
