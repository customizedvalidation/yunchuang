import { Can } from '../components/Can';
import React, { useCallback, useMemo } from 'react';
import {
  Card,
  Tabs,
  Tag,
  Switch,
  App,
  Row,
  Col,
  Table,
  Progress,
} from 'antd';
import {
  SafetyCertificateOutlined,
  LockOutlined,
  GlobalOutlined,
  BugOutlined,
  AuditOutlined,
  CheckCircleOutlined,
} from '@ant-design/icons';
import ResponsiveTable from '../components/ResponsiveTable';
import type { ColumnsType } from 'antd/es/table';
import { useGetSecurityPoliciesQuery, useUpdateSecurityPolicyMutation } from '../store/api';
import { extractArrayData } from '../utils/api';
import { renderState, EmptyState } from '../components/States';
import StatusCell from '../components/StatusCell';
import type { SecurityPolicy } from '../types';

/** 安全策略分类 → skill.md 4.5 安全维度 */
const POLICY_TYPE_TABS = [
  { key: 'access', label: '访问安全', tag: '认证 / 授权 / 审计' },
  { key: 'data', label: '数据安全', tag: '加密 / 备份 / 恢复' },
  { key: 'network', label: '网络安全', tag: '防火墙 / 入侵检测 / 隔离' },
  { key: 'system', label: '应用安全', tag: '代码审计 / 漏洞扫描' },
] as const;

/** 合规审计：静态 RBAC 权限矩阵（与 utils/auth.ts 的 rolePermissions 对齐） */
const ROLE_LABEL: Record<string, string> = {
  admin: '平台管理员',
  manager: '运维管理员',
  user: '普通用户',
};

const PERMISSION_GROUPS: { group: string; perms: string[] }[] = [
  { group: '集群', perms: ['cluster:read', 'cluster:write'] },
  { group: '资源', perms: ['resource:read', 'resource:write'] },
  { group: '作业', perms: ['job:read', 'job:write', 'job:submit'] },
  { group: '租户', perms: ['tenant:read', 'tenant:write'] },
  { group: '监控', perms: ['monitoring:read', 'monitoring:write'] },
  { group: '加速', perms: ['acceleration:read', 'acceleration:write'] },
  { group: '安全', perms: ['security:read', 'security:write'] },
  { group: 'GPU', perms: ['gpu:read', 'gpu:write'] },
  { group: '分区', perms: ['partition:read', 'partition:write'] },
  { group: '调度器', perms: ['scheduler:read', 'scheduler:write'] },
  { group: '拓扑', perms: ['topology:read', 'topology:write'] },
  { group: '数据集', perms: ['dataset:read', 'dataset:write'] },
];

const ROLE_PERMS: Record<string, Set<string>> = {
  admin: new Set(['admin', ...PERMISSION_GROUPS.flatMap((g) => g.perms)]),
  manager: new Set([
    'cluster:read', 'cluster:write', 'resource:read', 'resource:write',
    'job:read', 'job:write', 'job:submit', 'tenant:read', 'tenant:write',
    'monitoring:read', 'monitoring:write', 'acceleration:read', 'acceleration:write',
    'security:read', 'gpu:read', 'gpu:write', 'partition:read', 'partition:write',
    'scheduler:read', 'scheduler:write', 'topology:read', 'topology:write',
    'dataset:read', 'dataset:write',
  ]),
  user: new Set([
    'cluster:read', 'resource:read', 'job:read', 'monitoring:read',
    'acceleration:read', 'security:read', 'gpu:read', 'partition:read',
    'scheduler:read', 'topology:read', 'dataset:read',
  ]),
};

/** 各安全维度的能力说明卡片 */
const CAPABILITY_CARDS: Record<string, { title: string; desc: string; tag: string }[]> = {
  access: [
    { title: '身份认证', desc: '支持账号密码 / SSO / mTLS 多因素认证登录。', tag: '认证' },
    { title: '访问授权', desc: '基于 RBAC 的角色权限模型，最小权限原则收敛写操作。', tag: '授权' },
    { title: '操作审计', desc: '记录登录、配置变更与高危操作，支持按用户/时间检索。', tag: '审计' },
  ],
  data: [
    { title: '数据加密', desc: '敏感数据落盘 AES-256 加密，传输全程 TLS 1.3。', tag: '加密' },
    { title: '备份策略', desc: '数据集与配置定时快照，支持增量与全量备份。', tag: '备份' },
    { title: '灾难恢复', desc: '一键恢复到最近快照，RPO ≤ 15 分钟，RTO ≤ 1 小时。', tag: '恢复' },
  ],
  network: [
    { title: '防火墙', desc: '租户网络默认拒绝，按需放行端口与协议。', tag: '防火墙' },
    { title: '入侵检测', desc: '异常流量与端口扫描实时告警，自动隔离恶意源。', tag: 'IDS' },
    { title: '网络隔离', desc: '计算网 / 存储网 / 管理网三网物理隔离。', tag: '隔离' },
  ],
  system: [
    { title: '代码审计', desc: '镜像构建流水线静态扫描依赖与密钥泄露。', tag: '审计' },
    { title: '漏洞扫描', desc: '容器镜像 CVE 扫描，高危漏洞阻断上线。', tag: '扫描' },
    { title: '运行时防护', desc: 'Seccomp / AppArmor 限制容器系统调用面。', tag: '防护' },
  ],
};

const SecurityManagement: React.FC = () => {
  const { message } = App.useApp();
  const { data: policies, isLoading, error, refetch } = useGetSecurityPoliciesQuery(undefined);
  const policiesData = useMemo(() => extractArrayData<SecurityPolicy>(policies), [policies]);
  const [updatePolicy] = useUpdateSecurityPolicyMutation();

  const handleToggle = useCallback(
    async (id: number, enabled: boolean) => {
      try {
        await updatePolicy({ id, enabled }).unwrap();
        message.success(`安全策略${enabled ? '启用' : '禁用'}成功`);
        refetch();
      } catch {
        message.error('操作失败，请稍后重试');
      }
    },
    [updatePolicy, message, refetch],
  );

  const columns: ColumnsType<SecurityPolicy> = useMemo(
    () => [
      { title: 'ID', dataIndex: 'id', key: 'id', width: 70, render: (v: React.ReactNode) => <span className="mc-mono">{v}</span> },
      { title: '名称', dataIndex: 'name', key: 'name' },
      { title: '类型', dataIndex: 'type', key: 'type', width: 100, render: (t: string) => <Tag color="blue">{t}</Tag> },
      { title: '状态', dataIndex: 'status', key: 'status', width: 100, render: (s: string) => <StatusCell status={s} /> },
      { title: '描述', dataIndex: 'description', key: 'description', ellipsis: true },
      {
        title: '启用',
        key: 'enabled',
        width: 90,
        render: (_: unknown, record: SecurityPolicy) => (
          <Can perm="security:write">
            <Switch checked={record.enabled} onChange={(c) => handleToggle(record.id, c)} />
          </Can>
        ),
      },
    ],
    [handleToggle],
  );

  const state = renderState({
    isLoading,
    error,
    isEmpty: policiesData.length === 0,
    onRetry: refetch,
    skeletonRows: 5,
    skeletonColumns: 6,
    empty: <EmptyState title="暂无安全策略" description="还没有配置任何安全策略。" />,
  });

  const renderPolicyTab = (type: string) => {
    const list = policiesData.filter((p) => (p.type ?? 'access') === type);
    const cards = CAPABILITY_CARDS[type] ?? [];
    return (
      <>
        <Row gutter={[16, 16]} style={{ marginBottom: 16 }}>
          {cards.map((c) => (
            <Col xs={24} md={8} key={c.title}>
              <Card size="small" title={c.title} extra={<Tag color="blue">{c.tag}</Tag>}>
                <p style={{ color: 'var(--mc-text-3)', marginBottom: 0 }}>{c.desc}</p>
              </Card>
            </Col>
          ))}
        </Row>
        <Card title={`${type} 策略（${list.length} 条）`}>
          {state ?? (
            <ResponsiveTable
              columns={columns}
              dataSource={list}
              rowKey="id"
              pagination={false}
              size="small"
              scroll={{ x: 760 }}
            />
          )}
        </Card>
      </>
    );
  };

  // RBAC 权限矩阵
  const matrixColumns: ColumnsType<{ key: string; group: string; perm: string }> = [
    { title: '权限点', dataIndex: 'perm', key: 'perm', width: 220, render: (v: string) => <span className="mc-mono">{v}</span> },
    ...(['admin', 'manager', 'user'] as const).map((role) => ({
      title: ROLE_LABEL[role],
      key: role,
      width: 120,
      align: 'center' as const,
      render: (_: unknown, r: { perm: string }) =>
        ROLE_PERMS[role].has(r.perm) ? (
          <CheckCircleOutlined style={{ color: 'var(--mc-success, #16C784)' }} />
        ) : (
          <Tag>—</Tag>
        ),
    })),
  ];

  const matrixData = useMemo(
    () =>
      PERMISSION_GROUPS.flatMap((g) =>
        g.perms.map((p) => ({ key: `${g.group}-${p}`, group: g.group, perm: p })),
      ),
    [],
  );

  const complianceTab = (
    <>
      <Card title="RBAC 权限矩阵（角色 × 权限点）" style={{ marginBottom: 16 }}>
        <Table
          columns={matrixColumns}
          dataSource={matrixData}
          rowKey="key"
          pagination={false}
          size="small"
          scroll={{ x: 700 }}
        />
      </Card>
      <Row gutter={[16, 16]}>
        <Col xs={24} md={8}>
          <Card size="small" title="等保合规" extra={<Progress type="circle" percent={92} size={48} />}>
            <p style={{ color: 'var(--mc-text-3)', marginBottom: 0 }}>数据加密、访问审计与日志留存满足等保三级要求。</p>
          </Card>
        </Col>
        <Col xs={24} md={8}>
          <Card size="small" title="漏洞修复率" extra={<Progress type="circle" percent={88} size={48} status="active" />}>
            <p style={{ color: 'var(--mc-text-3)', marginBottom: 0 }}>近 30 天高危漏洞修复率，剩余项排期内闭环。</p>
          </Card>
        </Col>
        <Col xs={24} md={8}>
          <Card size="small" title="审计日志覆盖" extra={<Progress type="circle" percent={100} size={48} status="success" />}>
            <p style={{ color: 'var(--mc-text-3)', marginBottom: 0 }}>全部管理操作与数据访问均已纳入审计留痕。</p>
          </Card>
        </Col>
      </Row>
    </>
  );

  return (
    <div className="mc-page">
      <div className="mc-page-head">
        <div className="mc-page-head-main">
          <h1 className="mc-page-title">安全管理</h1>
          <p className="mc-page-desc">共 {policiesData.length} 条安全策略 · 访问 / 数据 / 网络 / 应用 / 合规</p>
        </div>
      </div>

      <Can perm="security:read">
        <Tabs
          defaultActiveKey="access"
          items={[
            ...POLICY_TYPE_TABS.map((t) => ({
              key: t.key,
              label: (
                <span>
                  {t.key === 'access' && <SafetyCertificateOutlined />}
                  {t.key === 'data' && <LockOutlined />}
                  {t.key === 'network' && <GlobalOutlined />}
                  {t.key === 'system' && <BugOutlined />}
                  {' '}
                  {t.label}
                </span>
              ),
              children: renderPolicyTab(t.key),
            })),
            {
              key: 'audit',
              label: (
                <span>
                  <AuditOutlined /> 合规审计
                </span>
              ),
              children: complianceTab,
            },
          ]}
        />
      </Can>
    </div>
  );
};

export default SecurityManagement;
