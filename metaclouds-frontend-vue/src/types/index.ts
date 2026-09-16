/**
 * 前端领域模型类型定义（Vue 版，与 React 版 src/types/index.ts 对齐）
 * 与后端 metaclouds-backend/models/*.go 的 json 标签一一对应。
 */

/** 通用审计字段：所有持久化实体都具备。 */
export interface BaseEntity {
  id: number
  created_at?: string
  updated_at?: string
}

/** 作业状态 */
export type JobStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled'

/** 作业类型 */
export type JobType = 'training' | 'inference' | 'batch'

/** 作业优先级：0=low, 1=medium, 2=high, 3=critical */
export type JobPriority = 0 | 1 | 2 | 3

/** GPU 厂商 */
export type GPUVendor = 'nvidia' | 'enflame' | 'moore_threads' | 'domestic_x'

export interface Job extends BaseEntity {
  cluster_id?: number
  tenant_id?: number
  user_id?: number
  name: string
  description?: string
  status: JobStatus
  type?: JobType
  priority?: JobPriority
  gpus?: number
  cpus?: number
  memory?: number
  duration?: number
  start_time?: string | null
  end_time?: string | null
  progress?: number
  output_path?: string
  error_msg?: string
  partition_id?: number
  scheduler_type?: 'k8s_native' | 'slurm' | 'lsf' | 'sge'
  scheduler_job_id?: string
  gpu_fraction?: number
  gpu_memory_gb?: number
  gpu_vendor?: GPUVendor
  qos?: string
  nodes_requested?: number
  node_selector?: string
  affinity?: string
  tolerations?: string
  elastic_enabled?: boolean
  min_gpus?: number
  max_gpus?: number
  scaling_policy?: string
  checkpoint_enabled?: boolean
  checkpoint_interval_minutes?: number
  max_retries?: number
  retry_count?: number
  fault_tolerance_level?: 'none' | 'node' | 'rack' | 'switch'
  topology_affinity?: 'node' | 'rack' | 'switch' | 'cluster'
  topology_anti_affinity?: boolean
  network_requirement?: 'rdma' | 'ethernet' | 'any'
}

export interface Cluster extends BaseEntity {
  name: string
  description?: string
  status: string
  nodes?: number
  gpus?: number
  cpus?: number
  memory?: number
  storage?: number
  network_type?: string
  location?: string
  gpu_vendors?: string
  scheduler_types?: string
  multi_cluster_enabled?: boolean
  federation_id?: string
}

export type ResourceType = 'gpu' | 'cpu' | 'memory' | 'storage' | 'network'

export interface Resource extends BaseEntity {
  cluster_id?: number
  type: ResourceType
  name: string
  status: string
  total?: number
  used?: number
  available?: number
  utilization?: number
  details?: string
  vendor?: GPUVendor
  gpu_model?: string
  vram_total_mb?: number
  vram_used_mb?: number
  vram_oversubscription_ratio?: number
  mig_enabled?: boolean
}

export interface GPUResource {
  gpuName: string
  type?: string
  status?: string
  total?: number
  used?: number
  available?: number
  utilization?: number
  details?: string
}

export interface GPUDevice extends BaseEntity {
  cluster_id?: number
  node_name: string
  vendor: GPUVendor
  model?: string
  index?: number
  total_memory_gb?: number
  allocatable_memory_gb?: number
  used_memory_gb?: number
  mig_enabled?: boolean
  mig_profiles?: string
  driver_version?: string
  cuda_version?: string
  status?: 'available' | 'allocated' | 'maintenance' | 'fault'
  utilization?: number
  temperature?: number
  power_draw?: number
  details?: string
}

export interface GPUAllocation extends BaseEntity {
  device_id: number
  device?: GPUDevice
  job_id?: number
  tenant_id?: number
  user_id?: number
  fraction: number
  memory_gb?: number
  mig_profile?: string
  status?: 'active' | 'released' | 'failed'
  started_at?: string | null
  ended_at?: string | null
}

export interface Partition extends BaseEntity {
  cluster_id: number
  cluster?: Cluster
  name: string
  description?: string
  priority?: number
  max_runtime_minutes?: number
  node_count?: number
  cpu_limit?: number
  gpu_count?: number
  gpu_vendor?: GPUVendor
  status?: 'active' | 'inactive' | 'maintenance' | 'drained'
  allow_sharing?: boolean
  default_qos?: string
  scheduler_type?: 'k8s_native' | 'slurm' | 'lsf' | 'sge'
  nodes?: string
  taints?: string
  labels?: string
}

export interface PartitionPermission extends BaseEntity {
  partition_id: number
  principal_type: 'user' | 'group'
  principal_id: number
  access_level: 'view' | 'submit' | 'admin'
}

export interface ResourceQuota extends BaseEntity {
  scope_type: 'tenant' | 'user' | 'partition' | 'node'
  scope_id: number
  resource_type: 'gpu' | 'cpu' | 'memory' | 'storage'
  limit: number
  used: number
  gpu_fraction_limit?: number
  gpu_memory_limit_gb?: number
  gpu_memory_used_gb?: number
  max_pod_cpu?: number
  min_pod_cpu?: number
  max_pod_memory?: number
  min_pod_memory?: number
  max_pod_gpu?: number
  min_pod_gpu?: number
  status?: string
}

export interface SchedulerIntegration extends BaseEntity {
  name: string
  type: 'slurm' | 'lsf' | 'sge' | 'k8s_native'
  endpoint?: string
  auth_type?: 'none' | 'ssh' | 'token' | 'mTLS'
  auth_config?: string
  version?: string
  status?: 'active' | 'inactive' | 'error'
  default_partition?: string
  default_qos?: string
  max_nodes?: number
  max_jobs?: number
  last_sync_at?: string | null
  details?: string
}

export interface NodeTopology extends BaseEntity {
  cluster_id?: number
  node_name: string
  rack_id?: string
  switch_id?: string
  pod_id?: string
  numa_nodes?: string
  gpu_topology?: string
  network_type?: 'roce' | 'ib' | 'ethernet'
  network_bandwidth_gbps?: number
  rdma_enabled?: boolean
  fault_domain?: string
}

export interface Dataset extends BaseEntity {
  name: string
  description?: string
  source_type: 'ceph' | 'nfs' | 's3' | 'glusterfs' | 'lustre' | 'beegfs'
  source_path?: string
  mount_path?: string
  size_gb?: number
  mount_options?: string
  access_mode?: string
  status?: string
  tenant_id?: number
}

export interface FluidCache extends BaseEntity {
  dataset_id: number
  dataset?: Dataset
  runtime_type?: 'alluxio' | 'jindofs'
  cache_capacity_gb?: number
  replicas?: number
  medium_type?: 'memory' | 'disk' | 'mixed'
  prefetch_enabled?: boolean
  prefetch_policy?: string
  compression_enabled?: boolean
  metadata_acceleration_enabled?: boolean
  status?: string
  cache_hit_rate?: number
}

export interface DistributedTrainingConfig extends BaseEntity {
  job_id: number
  parallel_strategy?: 'data' | 'model' | 'pipeline' | 'tensor' | 'hybrid'
  world_size?: number
  tensor_parallel_size?: number
  pipeline_stages?: number
  deepspeed_enabled?: boolean
  deepspeed_config_path?: string
  megatron_enabled?: boolean
  nccl_config?: string
  gradient_compression?: boolean
  communication_backend?: 'nccl' | 'gloo' | 'mpi'
}

export interface InferenceConfig extends BaseEntity {
  job_id: number
  backend?: 'tensorrt' | 'onnx_runtime' | 'pytorch' | 'tensorflow'
  precision?: 'fp32' | 'fp16' | 'int8' | 'int4'
  quantization_enabled?: boolean
  calibration_dataset?: string
  batch_size?: number
  dynamic_batching?: boolean
  max_batch_size?: number
  max_latency_ms?: number
  model_path?: string
  tensorrt_engine_path?: string
  gpu_utilization_target?: number
}

export interface Checkpoint extends BaseEntity {
  job_id: number
  job?: Job
  path: string
  step?: number
  epoch?: number
  size_mb?: number
  format?: 'pytorch' | 'tensorflow' | 'deepspeed' | 'megatron'
  is_latest?: boolean
  status?: 'in_progress' | 'completed' | 'failed' | 'corrupted'
}

export interface Tenant extends BaseEntity {
  name: string
  description?: string
  status: string
  gpu_quota?: number
  cpu_quota?: number
  memory_quota?: number
  storage_quota?: number
}

export type AlertLevel = 'info' | 'warning' | 'error' | 'critical'
export type AlertStatus = 'active' | 'resolved' | 'ignored'

export interface Alert extends BaseEntity {
  cluster_id?: number | null
  resource_id?: number | null
  job_id?: number | null
  type?: string
  level?: AlertLevel
  message?: string
  status?: AlertStatus
  resolved_at?: string | null
  details?: string
}

export interface AccelerationSuite extends BaseEntity {
  name: string
  description?: string
  type?: string
  version?: string
  status?: string
  enabled?: boolean
  details?: string
  config_json?: string
  category?: 'fluid_cache' | 'distributed_training' | 'inference' | 'communication'
  vendor?: GPUVendor
}

export interface SecurityPolicy extends BaseEntity {
  name: string
  description?: string
  type?: string
  status?: string
  enabled?: boolean
  rules?: string
  details?: string
}

export type UserRole = 'admin' | 'manager' | 'user'

export interface User {
  id: number
  username: string
  email?: string
  role: UserRole
  tenant_id?: number
  created_at?: string
  updated_at?: string
}

export interface LoginResponseData {
  token: string
  user: User
  expires_at: number
}

export interface RefreshTokenData {
  token: string
  user: User
  expires_at: number
}

export interface MetricsOverview {
  [key: string]: unknown
}

/** 后端统一响应信封 */
export interface ApiEnvelope<T> {
  success: boolean
  data: T
  message?: string
  code?: string
  timestamp?: number
}

/** 侧边栏菜单项 */
export interface MenuItem {
  key?: string
  label?: string
  description?: string
  icon?: string
  type?: 'group' | 'divider'
  children?: MenuItem[]
  roles?: UserRole[]
}
