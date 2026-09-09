import { Can } from '../components/Can';
import React, { useCallback, useMemo, useState } from 'react';
import { Card } from 'antd';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import { Tag, Switch, App, Button, Space, Modal, Form, Input, Select, Row, Col } from 'antd';
import { useNavigate } from 'react-router-dom';
import { useGetAccelerationSuitesQuery, useUpdateAccelerationSuiteMutation } from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { AccelerationSuite, GPUVendor } from '../types';

const CATEGORY_OPTIONS = [
  { label: 'Fluid 缓存', value: 'fluid_cache' },
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

/** 加速套件表单值 */
interface SuiteFormValues {
  config_json?: string;
  category?: 'fluid_cache' | 'distributed_training' | 'inference' | 'communication';
  vendor?: GPUVendor;
}

const AccelerationSuiteManagement: React.FC = () => {
  const { message } = App.useApp();
  const navigate = useNavigate();
  const { data: suites, isLoading, error, refetch } = useGetAccelerationSuitesQuery(undefined);
  const suitesData = extractArrayData<AccelerationSuite>(suites);
  const [updateSuite] = useUpdateAccelerationSuiteMutation();

  const [editModalVisible, setEditModalVisible] = useState(false);
  const [editingSuite, setEditingSuite] = useState<AccelerationSuite | null>(null);
  const [form] = Form.useForm<SuiteFormValues>();

  // useCallback：稳定回调引用，避免 Switch onChange 传递新函数导致子组件重渲染
  const handleToggle = useCallback(async (id: number, enabled: boolean) => {
    try {
      await updateSuite({ id, enabled }).unwrap();
      message.success(`加速套件${enabled ? '启用' : '禁用'}成功`);
      refetch();
    } catch {
      message.error('操作失败，请稍后重试');
    }
  }, [updateSuite, message, refetch]);

  // 打开编辑
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

  // 保存编辑
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

  // 列配置用 useMemo 缓存
  const columns: ColumnsType<AccelerationSuite> = useMemo(() => [
    { title: 'ID', dataIndex: 'id', key: 'id', width: 80, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
    { title: '名称', dataIndex: 'name', key: 'name' },
    { title: '类型', dataIndex: 'type', key: 'type', render: (type: string) => <Tag>{type}</Tag> },
    {
      title: '类别',
      dataIndex: 'category',
      key: 'category',
      width: 120,
      render: (v: string) => {
        const opt = CATEGORY_OPTIONS.find((o) => o.value === v);
        return opt ? <Tag color="blue">{opt.label}</Tag> : '-';
      },
    },
    {
      title: '厂商',
      dataIndex: 'vendor',
      key: 'vendor',
      width: 100,
      render: (v: GPUVendor) => (v ? <Tag color="green">{v}</Tag> : '-'),
    },
    { title: '版本', dataIndex: 'version', key: 'version', width: 100 },
    { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (status: string) => <StatusCell status={status} /> },
    { title: '描述', dataIndex: 'description', key: 'description' },
    {
      title: '启用', key: 'enabled', width: 90,
      render: (_: unknown, record: AccelerationSuite) => (
        <Can perm="acceleration:write">
          <Switch checked={record.enabled} onChange={(checked) => handleToggle(record.id, checked)} />
        </Can>
      ),
    },
    {
      title: '操作',
      key: 'action',
      width: 100,
      render: (_: unknown, record: AccelerationSuite) => (
        <Can perm="acceleration:write">
          <Button type="link" size="small" onClick={() => handleOpenEdit(record)}>
            编辑
          </Button>
        </Can>
      ),
    },
  ], [handleToggle, handleOpenEdit]);

  const state = renderState({
    isLoading,
    error,
    isEmpty: suitesData.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 7,
    empty: <EmptyState title="暂无加速套件" description="还没有登记任何加速套件。" />,
  });

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">加速套件管理</h1>
          <p className="mc-page-desc">共 {suitesData.length} 个加速套件 · 启停推理/训练加速能力</p>
        </div>
        <div className="mc-page-head-extra">
          <Button onClick={() => navigate('/datasets')}>数据集与缓存</Button>
        </div>
      </div>

      <Card>
        {state ?? (
          <ResponsiveTable
            columns={columns}
            dataSource={suitesData}
            rowKey="id"
            pagination={{ pageSize: 10, showTotal: (t) => `共 ${t} 条` }}
            scroll={{ x: 1100 }}
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
    </div>
  );
};

export default AccelerationSuiteManagement;
