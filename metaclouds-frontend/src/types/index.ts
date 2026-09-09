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

/** GPU 厂商：NVIDIA / 燧原 / 摩尔线程 / 国产 X */
export type GPUVendor = 'nvidia' | 'enflame' | 'moore_threads' | 'domestic_x';

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
  /** P0 差距补齐：分区与调度器 */
  partition_id?: number;
  scheduler_type?: 'k8s_native' | 'slurm' | 'lsf' | 'sge';
  scheduler_job_id?: string;
  /** P0 差距补齐：GPU 细粒度 */
  gpu_fraction?: number;
  gpu_memory_gb?: number;
  gpu_vendor?: GPUVendor;
  qos?: string;
  nodes_requested?: number;
  node_selector?: string;
  affinity?: string;
  tolerations?: string;
  /** P1 差距补齐：弹性训练 */
  elastic_enabled?: boolean;
  min_gpus?: number;
  max_gpus?: number;
  scaling_policy?: string;
  /** P1 差距补齐：容错训练 */
  checkpoint_enabled?: boolean;
  checkpoint_interval_minutes?: number;
  max_retries?: number;
  retry_count?: number;
  fault_tolerance_level?: 'none' | 'node' | 'rack' | 'switch';
  /** P1 差距补齐：拓扑感知 */
  topology_affinity?: 'node' | 'rack' | 'switch' | 'cluster';
  topology_anti_affinity?: boolean;
  network_requirement?: 'rdma' | 'ethernet' | 'any';
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
  /** P0 差距补齐：多 GPU 厂商与调度器聚合 */
  gpu_vendors?: string;
  scheduler_types?: string;
  /** P2 预留：多集群联邦 */
  multi_cluster_enabled?: boolean;
  federation_id?: string;
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
  /** P0 差距补齐：多 GPU 厂商与显存细粒度 */
  vendor?: GPUVendor;
  gpu_model?: string;
  vram_total_mb?: number;
  vram_used_mb?: number;
  vram_oversubscription_ratio?: number;
  mig_enabled?: boolean;
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

/** GPU 设备：单张物理 GPU 卡的细粒度模型（P0-1 / P0-2） */
export interface GPUDevice extends BaseEntity {
  cluster_id?: number;
  node_name: string;
  vendor: GPUVendor;
  model?: string;
  index?: number;
  total_memory_gb?: number;
  allocatable_memory_gb?: number;
  used_memory_gb?: number;
  mig_enabled?: boolean;
  mig_profiles?: string;
  driver_version?: string;
  cuda_version?: string;
  status?: 'available' | 'allocated' | 'maintenance' | 'fault';
  utilization?: number;
  temperature?: number;
  power_draw?: number;
  details?: string;
}

/** GPU 分配记录：作业对 GPU 设备的细粒度占用（P0-1） */
export interface GPUAllocation extends BaseEntity {
  device_id: number;
  device?: GPUDevice;
  job_id?: number;
  tenant_id?: number;
  user_id?: number;
  /** 分配分数：1.0 / 0.5 / 0.25 */
  fraction: number;
  memory_gb?: number;
  mig_profile?: string;
  status?: 'active' | 'released' | 'failed';
  started_at?: string | null;
  ended_at?: string | null;
}

/** 分区：Slurm 风格的计算分区（P0-4） */
export interface Partition extends BaseEntity {
  cluster_id: number;
  cluster?: Cluster;
  name: string;
  description?: string;
  priority?: number;
  max_runtime_minutes?: number;
  node_count?: number;
  cpu_limit?: number;
  gpu_count?: number;
  gpu_vendor?: GPUVendor;
  status?: 'active' | 'inactive' | 'maintenance' | 'drained';
  allow_sharing?: boolean;
  default_qos?: string;
  scheduler_type?: 'k8s_native' | 'slurm' | 'lsf' | 'sge';
  nodes?: string;
  taints?: string;
  labels?: string;
}

/** 分区权限：用户/用户组对分区的访问级别（P0-4） */
export interface PartitionPermission extends BaseEntity {
  partition_id: number;
  principal_type: 'user' | 'group';
  principal_id: number;
  access_level: 'view' | 'submit' | 'admin';
}

/** 资源配额：多维度（租户/用户/分区/节点）统一配额模型（P0-5） */
export interface ResourceQuota extends BaseEntity {
  scope_type: 'tenant' | 'user' | 'partition' | 'node';
  scope_id: number;
  resource_type: 'gpu' | 'cpu' | 'memory' | 'storage';
  limit: number;
  used: number;
  gpu_fraction_limit?: number;
  gpu_memory_limit_gb?: number;
  gpu_memory_used_gb?: number;
  max_pod_cpu?: number;
  min_pod_cpu?: number;
  max_pod_memory?: number;
  min_pod_memory?: number;
  max_pod_gpu?: number;
  min_pod_gpu?: number;
  status?: string;
}

/** 调度器集成：外部 Slurm/LSF/SGE/K8s 调度器配置（P0-3） */
export interface SchedulerIntegration extends BaseEntity {
  name: string;
  type: 'slurm' | 'lsf' | 'sge' | 'k8s_native';
  endpoint?: string;
  auth_type?: 'none' | 'ssh' | 'token' | 'mTLS';
  auth_config?: string;
  version?: string;
  status?: 'active' | 'inactive' | 'error';
  default_partition?: string;
  default_qos?: string;
  max_nodes?: number;
  max_jobs?: number;
  last_sync_at?: string | null;
  details?: string;
}

/** 节点拓扑：机架/交换机/NUMA/GPU NVLink 层级（P1-1） */
export interface NodeTopology extends BaseEntity {
  cluster_id?: number;
  node_name: string;
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

/** 数据集：Fluid 数据源（P1-5） */
export interface Dataset extends BaseEntity {
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

/** Fluid 缓存：Alluxio/JindoFS 分布式缓存配置（P1-5） */
export interface FluidCache extends BaseEntity {
  dataset_id: number;
  dataset?: Dataset;
  runtime_type?: 'alluxio' | 'jindofs';
  cache_capacity_gb?: number;
  replicas?: number;
  medium_type?: 'memory' | 'disk' | 'mixed';
  prefetch_enabled?: boolean;
  prefetch_policy?: string;
  compression_enabled?: boolean;
  metadata_acceleration_enabled?: boolean;
  status?: string;
  cache_hit_rate?: number;
}

/** 分布式训练配置：并行策略 / DeepSpeed / Megatron / NCCL（P1-6） */
export interface DistributedTrainingConfig extends BaseEntity {
  job_id: number;
  parallel_strategy?: 'data' | 'model' | 'pipeline' | 'tensor' | 'hybrid';
  world_size?: number;
  tensor_parallel_size?: number;
  pipeline_stages?: number;
  deepspeed_enabled?: boolean;
  deepspeed_config_path?: string;
  megatron_enabled?: boolean;
  nccl_config?: string;
  gradient_compression?: boolean;
  communication_backend?: 'nccl' | 'gloo' | 'mpi';
}

/** 推理配置：TensorRT/ONNX 后端、量化、批处理（P1-7） */
export interface InferenceConfig extends BaseEntity {
  job_id: number;
  backend?: 'tensorrt' | 'onnx_runtime' | 'pytorch' | 'tensorflow';
  precision?: 'fp32' | 'fp16' | 'int8' | 'int4';
  quantization_enabled?: boolean;
  calibration_dataset?: string;
  batch_size?: number;
  dynamic_batching?: boolean;
  max_batch_size?: number;
  max_latency_ms?: number;
  model_path?: string;
  tensorrt_engine_path?: string;
  gpu_utilization_target?: number;
}

/** Checkpoint：训练检查点管理（P1-4） */
export interface Checkpoint extends BaseEntity {
  job_id: number;
  job?: Job;
  path: string;
  step?: number;
  epoch?: number;
  size_mb?: number;
  format?: 'pytorch' | 'tensorflow' | 'deepspeed' | 'megatron';
  is_latest?: boolean;
  status?: 'in_progress' | 'completed' | 'failed' | 'corrupted';
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
  /** P1 差距补齐：加速套件细分配置 */
  config_json?: string;
  category?: 'fluid_cache' | 'distributed_training' | 'inference' | 'communication';
  vendor?: GPUVendor;
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
  /**
   * 允许看到该入口的角色，须与后端 authz.rolePermissions 对齐。
   * 缺省表示不限制（所有角色可见）。
   */
  roles?: UserRole[];
}
