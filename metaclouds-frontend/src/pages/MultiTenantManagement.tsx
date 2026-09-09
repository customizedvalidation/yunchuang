import { Can } from '../components/Can';
import React, { useCallback, useMemo, useState } from 'react';
import { Card, Button, Space, App, Modal, Form, Input, InputNumber, Popconfirm, Tabs, Tag, Select, Row, Col, Progress } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import { useGetTenantsQuery, useCreateTenantMutation, useDeleteTenantMutation, useGetQuotasQuery, useCreateQuotaMutation, useUpdateQuotaMutation, useDeleteQuotaMutation } from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { Tenant, ResourceQuota } from '../types';

/** 创建租户表单值（与 MultiTenantManagement 表单字段一致） */
interface TenantFormValues {
  name: string;
  description?: string;
  gpu_quota: number;
  cpu_quota: number;
  memory_quota: number;
}

/** 配额表单值 */
interface QuotaFormValues {
  resource_type: 'gpu' | 'cpu' | 'memory' | 'storage';
  limit: number;
  gpu_fraction_limit?: number;
  gpu_memory_limit_gb?: number;
  max_pod_cpu?: number;
  min_pod_cpu?: number;
  max_pod_memory?: number;
  min_pod_memory?: number;
  max_pod_gpu?: number;
  min_pod_gpu?: number;
}

const RESOURCE_TYPE_OPTIONS = [
  { label: 'GPU', value: 'gpu' },
  { label: 'CPU', value: 'cpu' },
  { label: '内存', value: 'memory' },
  { label: '存储', value: 'storage' },
];

const MultiTenantManagement: React.FC = () => {
  const { message } = App.useApp();
  const { data: tenants, isLoading, error, refetch } = useGetTenantsQuery(undefined);
  const tenantsData = extractArrayData<Tenant>(tenants);
  const [createTenant] = useCreateTenantMutation();
  const [deleteTenant] = useDeleteTenantMutation();
  const [isModalVisible, setIsModalVisible] = useState(false);
  const [form] = Form.useForm();

  // 配额管理
  const [activeTab, setActiveTab] = useState<'tenants' | 'quotas'>('tenants');
  const [selectedTenantId, setSelectedTenantId] = useState<number | null>(null);
  const [quotaModalVisible, setQuotaModalVisible] = useState(false);
  const [editingQuota, setEditingQuota] = useState<ResourceQuota | null>(null);
  const [quotaForm] = Form.useForm<QuotaFormValues>();

  const { data: quotas } = useGetQuotasQuery(
    selectedTenantId ? { scope_type: 'tenant', scope_id: selectedTenantId } : { scope_type: 'tenant' },
  );
  const quotasData = quotas ?? [];
  const [createQuota] = useCreateQuotaMutation();
  const [updateQuota] = useUpdateQuotaMutation();
  const [deleteQuota] = useDeleteQuotaMutation();

  // useCallback：稳定回调引用
  const handleCreate = useCallback(async (values: TenantFormValues) => {
    try {
      await createTenant(values).unwrap();
      message.success('租户创建成功');
      setIsModalVisible(false);
      form.resetFields();
      refetch();
    } catch {
      message.error('租户创建失败，请检查必填项后重试');
    }
  }, [createTenant, form, message, refetch]);

  const handleDelete = useCallback(async (id: number) => {
    try {
      await deleteTenant(id).unwrap();
      message.success('租户删除成功');
      refetch();
    } catch {
      message.error('租户删除失败，请稍后重试');
    }
  }, [deleteTenant, message, refetch]);

  // 配额操作
  const handleOpenCreateQuota = useCallback(() => {
    setEditingQuota(null);
    quotaForm.resetFields();
    setQuotaModalVisible(true);
  }, [quotaForm]);

  const handleOpenEditQuota = useCallback(
    (record: ResourceQuota) => {
      setEditingQuota(record);
      quotaForm.setFieldsValue({
        resource_type: record.resource_type,
        limit: record.limit,
        gpu_fraction_limit: record.gpu_fraction_limit,
        gpu_memory_limit_gb: record.gpu_memory_limit_gb,
        max_pod_cpu: record.max_pod_cpu,
        min_pod_cpu: record.min_pod_cpu,
        max_pod_memory: record.max_pod_memory,
        min_pod_memory: record.min_pod_memory,
        max_pod_gpu: record.max_pod_gpu,
        min_pod_gpu: record.min_pod_gpu,
      });
      setQuotaModalVisible(true);
    },
    [quotaForm],
  );

  const handleSaveQuota = useCallback(
    async (values: QuotaFormValues) => {
      if (!selectedTenantId) {
        message.warning('请先选择租户');
        return;
      }
      try {
        if (editingQuota) {
          await updateQuota({ id: editingQuota.id, data: values }).unwrap();
          message.success('配额更新成功');
        } else {
          await createQuota({ ...values, scope_type: 'tenant', scope_id: selectedTenantId, used: 0 }).unwrap();
          message.success('配额创建成功');
        }
        setQuotaModalVisible(false);
        quotaForm.resetFields();
      } catch {
        message.error('操作失败，请稍后重试');
      }
    },
    [selectedTenantId, editingQuota, createQuota, updateQuota, quotaForm, message],
  );

  const handleDeleteQuota = useCallback(
    async (id: number) => {
      try {
        await deleteQuota(id).unwrap();
        message.success('配额删除成功');
      } catch {
        message.error('删除失败，请稍后重试');
      }
    },
    [deleteQuota, message],
  );

  // 列配置用 useMemo 缓存
  const columns: ColumnsType<Tenant> = useMemo(() => [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 80, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '描述', dataIndex: 'description', key: 'description' },
    { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (status: string) => <StatusCell status={status} /> },
    { title: 'GPU配额', dataIndex: 'gpu_quota', key: 'gpu_quota', width: 100, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: 'CPU配额', dataIndex: 'cpu_quota', key: 'cpu_quota', width: 100, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '内存配额(GB)', dataIndex: 'memory_quota', key: 'memory_quota', width: 120, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    {
      title: '操作', key: 'action', width: 180,
      render: (_: React.ReactNode, record: Tenant) => (
        <Space>
          <Button type="link" size="small" onClick={() => { setSelectedTenantId(record.id); setActiveTab('quotas'); }}>
            配额管理
          </Button>
          <Can perm="tenant:write">
          <Popconfirm
            title="删除该租户？"
            description="租户下的配额与关联资源将一并移除，且不可恢复。"
            okText="确认删除"
            cancelText="返回"
            okButtonProps={{ danger: true }}
            onConfirm={() => handleDelete(record.id)}
          >
            <Button type="link" danger size="small">删除</Button>
          </Popconfirm>
          </Can>
        </Space>
      ),
    },
  ], [handleDelete]);

  // 配额列配置
  const quotaColumns: ColumnsType<ResourceQuota> = useMemo(() => [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 70 },
    {
      title: '资源类型',
      dataIndex: 'resource_type',
      key: 'resource_type',
      width: 100,
      render: (v: string) => {
        const opt = RESOURCE_TYPE_OPTIONS.find((o) => o.value === v);
        return <Tag color="blue">{opt?.label ?? v}</Tag>;
      },
    },
    { title: '限制', dataIndex: 'limit', key: 'limit', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    { title: '已用', dataIndex: 'used', key: 'used', width: 90, render: (v: React.ReactNode) => <span className="mc-num">{v}</span> },
    {
      title: '使用率',
      key: 'usage',
      width: 140,
      render: (_: unknown, record: ResourceQuota) => {
        const pct = record.limit > 0 ? Math.round((record.used / record.limit) * 100) : 0;
        return <Progress percent={pct} size="small" />;
      },
    },
    {
      title: 'GPU分数限制',
      dataIndex: 'gpu_fraction_limit',
      key: 'gpu_fraction_limit',
      width: 110,
      render: (v: React.ReactNode) => v ?? '-',
    },
    {
      title: 'GPU显存限制(GB)',
      dataIndex: 'gpu_memory_limit_gb',
      key: 'gpu_memory_limit_gb',
      width: 130,
      render: (v: React.ReactNode) => v ?? '-',
    },
    { title: '单Pod最大CPU', dataIndex: 'max_pod_cpu', key: 'max_pod_cpu', width: 120, render: (v: React.ReactNode) => v ?? '-' },
    { title: '单Pod最小CPU', dataIndex: 'min_pod_cpu', key: 'min_pod_cpu', width: 120, render: (v: React.ReactNode) => v ?? '-' },
    { title: '单Pod最大内存', dataIndex: 'max_pod_memory', key: 'max_pod_memory', width: 130, render: (v: React.ReactNode) => v ?? '-' },
    { title: '单Pod最大GPU', dataIndex: 'max_pod_gpu', key: 'max_pod_gpu', width: 120, render: (v: React.ReactNode) => v ?? '-' },
    {
      title: '操作',
      key: 'action',
      width: 140,
      render: (_: unknown, record: ResourceQuota) => (
        <Space>
          <Can perm="tenant:write">
            <Button type="link" size="small" onClick={() => handleOpenEditQuota(record)}>编辑</Button>
          </Can>
          <Can perm="tenant:write">
            <Popconfirm title="删除该配额？" onConfirm={() => handleDeleteQuota(record.id)} okText="确认" cancelText="取消">
              <Button type="link" danger size="small">删除</Button>
            </Popconfirm>
          </Can>
        </Space>
      ),
    },
  ], [handleOpenEditQuota, handleDeleteQuota]);

  const state = renderState({
    isLoading,
    error,
    isEmpty: tenantsData.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 8,
    empty: (
      <EmptyState
        title="还没有租户"
        description="创建租户以划分命名空间与配额边界。"
        action={<Can perm="tenant:write"><Button type="primary" onClick={() => setIsModalVisible(true)}>创建租户</Button></Can>}
      />
    ),
  });

  const tabItems = [
    {
      key: 'tenants',
      label: '租户列表',
      children: state ?? (
        <ResponsiveTable
          columns={columns}
          dataSource={tenantsData}
          rowKey="id"
          pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
          scroll={{ x: 900 }}
        />
      ),
    },
    {
      key: 'quotas',
      label: '配额管理',
      children: (
        <div>
          <div style={{ marginBottom: 16 }}>
            <Space>
              <span>选择租户：</span>
              <Select
                placeholder="请选择租户"
                style={{ width: 240 }}
                value={selectedTenantId ?? undefined}
                onChange={(v) => setSelectedTenantId(v)}
                options={tenantsData.map((t) => ({ label: t.name, value: t.id }))}
              />
              {selectedTenantId && (
                <Can perm="tenant:write">
                  <Button type="primary" onClick={handleOpenCreateQuota}>新增配额</Button>
                </Can>
              )}
            </Space>
          </div>
          {selectedTenantId ? (
            <ResponsiveTable
              columns={quotaColumns}
              dataSource={quotasData}
              rowKey="id"
              pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
              scroll={{ x: 1500 }}
            />
          ) : (
            <EmptyState title="请选择租户" description="选择租户后可查看和管理其资源配额。" />
          )}
        </div>
      ),
    },
  ];

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">多租户管理</h1>
          <p className="mc-page-desc">共 {tenantsData.length} 个租户 · 按租户划分命名空间与配额边界</p>
        </div>
        <div className="mc-page-head-extra">
          <Can perm="tenant:write"><Button type="primary" onClick={() => setIsModalVisible(true)}>创建租户</Button></Can>
        </div>
      </div>

      <Card>
        <Tabs activeKey={activeTab} items={tabItems} onChange={(k) => setActiveTab(k as 'tenants' | 'quotas')} />
      </Card>

      <Modal className="mc-modal-full" title="创建租户" open={isModalVisible} onCancel={() => setIsModalVisible(false)} footer={null} destroyOnHidden>
        <Form form={form} onFinish={handleCreate} layout="vertical">
          <Form.Item name="name" label="名称" rules={[{ required: true, message: '请输入租户名称' }]}>
            <Input placeholder="例如：team-vision" />
          </Form.Item>
          <Form.Item name="description" label="描述">
            <Input.TextArea rows={3} placeholder="选填，便于后续追溯" />
          </Form.Item>
          <Form.Item name="gpu_quota" label="GPU配额" rules={[{ required: true, message: '请输入 GPU 配额' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="cpu_quota" label="CPU配额" rules={[{ required: true, message: '请输入 CPU 配额' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item name="memory_quota" label="内存配额(GB)" rules={[{ required: true, message: '请输入内存配额' }]}>
            <InputNumber min={0} style={{ width: '100%' }} />
          </Form.Item>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Can perm="tenant:write"><Button type="primary" htmlType="submit">创建</Button></Can>
              <Button onClick={() => setIsModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>

      {/* 配额 Modal */}
      <Modal
        title={editingQuota ? '编辑配额' : '新增配额'}
        open={quotaModalVisible}
        onCancel={() => setQuotaModalVisible(false)}
        footer={null}
        destroyOnHidden
        width={680}
      >
        <Form form={quotaForm} onFinish={handleSaveQuota} layout="vertical">
          <Row gutter={16}>
            <Col span={12}>
              <Form.Item name="resource_type" label="资源类型" rules={[{ required: true }]}>
                <Select options={RESOURCE_TYPE_OPTIONS} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="limit" label="限制值" rules={[{ required: true }]}>
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="gpu_fraction_limit" label="GPU分数限制">
                <InputNumber min={0} max={1} step={0.25} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={12}>
              <Form.Item name="gpu_memory_limit_gb" label="GPU显存限制(GB)">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="max_pod_cpu" label="单Pod最大CPU">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="min_pod_cpu" label="单Pod最小CPU">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="max_pod_memory" label="单Pod最大内存">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="min_pod_memory" label="单Pod最小内存">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="max_pod_gpu" label="单Pod最大GPU">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
            <Col span={8}>
              <Form.Item name="min_pod_gpu" label="单Pod最小GPU">
                <InputNumber min={0} style={{ width: '100%' }} />
              </Form.Item>
            </Col>
          </Row>
          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Can perm="tenant:write">
                <Button type="primary" htmlType="submit">{editingQuota ? '保存' : '创建'}</Button>
              </Can>
              <Button onClick={() => setQuotaModalVisible(false)}>取消</Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </div>
  );
};

export default MultiTenantManagement;
