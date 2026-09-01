/**
 * 前端领域模型类型定义
 *
 * 本文件与各后端 model（metaclouds-backend/models/*.go）的 json 标签一一对应。
 * 此前全站缺少领域类型、组件内到处是 `any`，导致字段拼错、状态值写错
 * 都无法被 TypeScript 发现。新增字段时请同步更新此处。
 */
import type { ReactNode } from 'react';

/** 通用审计字段：所有持久化实体都具备。 */
export interface BaseEntity {
  id: number;
  created_at?: string;
  updated_at?: string;
}

/** 作业状态：pending, running, completed, failed, cancelled */
export type JobStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';

/** 作业类型：training, inference, batch */
export type JobType = 'training' | 'inference' | 'batch';

/** 作业优先级：0=low, 1=medium, 2=high, 3=critical */
export type JobPriority = 0 | 1 | 2 | 3;

export interface Job extends BaseEntity {
  cluster_id?: number;
  tenant_id?: number;
  user_id?: number;
  name: string;
  description?: string;
  status: JobStatus;
  type?: JobType;
  priority?: JobPriority;
  gpus?: number;
  cpus?: number;
  /** 内存（GB） */
  memory?: number;
  /** 预计运行时间（分钟） */
  duration?: number;
  start_time?: string | null;
  end_time?: string | null;
  /** 进度（0-100） */
  progress?: number;
  output_path?: string;
  error_msg?: string;
}

/** 集群状态：active, inactive, maintenance 等 */
export interface Cluster extends BaseEntity {
  name: string;
  description?: string;
  status: string;
  nodes?: number;
  gpus?: number;
  cpus?: number;
  /** 内存（GB） */
  memory?: number;
  /** 存储（TB） */
  storage?: number;
  network_type?: string;
  location?: string;
}

/** 资源类型：gpu, cpu, memory, storage, network */
export type ResourceType = 'gpu' | 'cpu' | 'memory' | 'storage' | 'network';

export interface Resource extends BaseEntity {
  cluster_id?: number;
  type: ResourceType;
  name: string;
  status: string;
  total?: number;
  used?: number;
  available?: number;
  utilization?: number;
  details?: string;
}

/** K8S GPU 节点资源（GET /k8s/gpu）：用于 GPU 资源管理与服务管理表格 */
export interface GPUResource {
  /** 节点/服务名 */
  gpuName: string;
  /** 卡型，如 A100/V100 */
  type?: string;
  status?: string;
  total?: number;
  used?: number;
  available?: number;
  /** 利用率（0-100） */
  utilization?: number;
  details?: string;
}

export interface Tenant extends BaseEntity {
  name: string;
  description?: string;
  status: string;
  gpu_quota?: number;
  cpu_quota?: number;
  /** 内存配额（GB） */
  memory_quota?: number;
  /** 存储配额（TB） */
  storage_quota?: number;
}

/** 告警级别：info, warning, error, critical */
export type AlertLevel = 'info' | 'warning' | 'error' | 'critical';

/** 告警状态：active, resolved, ignored */
export type AlertStatus = 'active' | 'resolved' | 'ignored';

export interface Alert extends BaseEntity {
  cluster_id?: number | null;
  resource_id?: number | null;
  job_id?: number | null;
  /** system, resource, job, security */
  type?: string;
  level?: AlertLevel;
  message?: string;
  status?: AlertStatus;
  resolved_at?: string | null;
  details?: string;
}

export interface AccelerationSuite extends BaseEntity {
  name: string;
  description?: string;
  /** data, training, inference */
  type?: string;
  version?: string;
  status?: string;
  enabled?: boolean;
  details?: string;
}

export interface SecurityPolicy extends BaseEntity {
  name: string;
  description?: string;
  /** access, network, data, system */
  type?: string;
  status?: string;
  enabled?: boolean;
  /** JSON 字符串格式的规则 */
  rules?: string;
  details?: string;
}

/** 用户角色，与后端 pkg/authz 的 Role 保持一致 */
export type UserRole = 'admin' | 'manager' | 'user';

export interface User {
  id: number;
  username: string;
  email?: string;
  role: UserRole;
  tenant_id?: number;
  created_at?: string;
  updated_at?: string;
}

/** 登录接口返回体 */
export interface LoginResponse {
  token: string;
  user: User;
}

/** 监控指标概览（GET /monitoring/metrics） */
export interface MetricsOverview {
  [key: string]: unknown;
}

/**
 * 侧边栏菜单项。
 *
 * 支持 antd Menu 的三种节点形态：分组标题（group）、分隔线（divider）、普通菜单项。
 * 普通菜单项可嵌套 children 形成子菜单。
 */
export interface MenuItem {
  /** 路由路径，同时作为 antd Menu 的唯一 key */
  key?: string;
  icon?: ReactNode;
  /** 未处理前是纯文本标签；渲染阶段会被替换为带徽标的 React 节点 */
  label?: React.ReactNode;
  /** 折叠态时隐藏的补充说明文字 */
  description?: string;
  type?: 'group' | 'divider';
  children?: MenuItem[];
}
