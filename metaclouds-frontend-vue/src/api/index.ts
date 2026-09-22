import http from './http'
import type {
  LoginResponseData,
  RefreshTokenData,
  Cluster,
  Resource,
  Job,
  Alert,
  Tenant,
  AccelerationSuite,
  SecurityPolicy,
  GPUDevice,
  GPUAllocation,
  Partition,
  PartitionPermission,
  ResourceQuota,
  SchedulerIntegration,
  NodeTopology,
  Dataset,
  FluidCache,
  Checkpoint,
  MetricsOverview,
  GPUResource,
} from '@/types'

// ==================== 认证 ====================
export const authApi = {
  login: (data: { username: string; password: string }) =>
    http.post<LoginResponseData>('/auth/login', data).then((r) => r.data),
  logout: () => http.post<void>('/auth/logout').then((r) => r.data),
  getCsrfToken: () =>
    http.get<{ csrf_token: string }>('/auth/csrf').then((r) => r.data),
  register: (data: Record<string, unknown>) =>
    http.post('/auth/register', data).then((r) => r.data),
  refreshToken: () =>
    http.post<RefreshTokenData>('/auth/refresh').then((r) => r.data),
}

// ==================== 集群管理 ====================
export const clusterApi = {
  list: () => http.get<Cluster[]>('/clusters').then((r) => r.data),
  create: (data: Partial<Cluster>) =>
    http.post<Cluster>('/clusters', data).then((r) => r.data),
  update: (id: number, data: Partial<Cluster>) =>
    http.put<Cluster>(`/clusters/${id}`, data).then((r) => r.data),
  remove: (id: number) => http.delete<void>(`/clusters/${id}`).then((r) => r.data),
}

// ==================== 资源管理 ====================
export const resourceApi = {
  list: () => http.get<Resource[]>('/resources').then((r) => r.data),
  update: (id: number, data: Partial<Resource>) =>
    http.put<Resource>(`/resources/${id}`, data).then((r) => r.data),
  gpuResources: () => http.get<GPUResource[]>('/resources/gpu').then((r) => r.data),
}

// ==================== 作业管理 ====================
export const jobApi = {
  list: () => http.get<Job[]>('/jobs').then((r) => r.data),
  create: (data: Partial<Job>) =>
    http.post<Job>('/jobs', data).then((r) => r.data),
  cancel: (id: number) =>
    http.post<void>(`/jobs/${id}/cancel`).then((r) => r.data),
  submitToK8S: (jobId: number) =>
    http.post<void>(`/jobs/${jobId}/submit`).then((r) => r.data),
  getK8SStatus: (jobId: number) =>
    http.get(`/jobs/${jobId}/status`).then((r) => r.data),
  cancelK8S: (jobId: number) =>
    http.post<void>(`/jobs/${jobId}/cancel`).then((r) => r.data),
}

// ==================== 监控告警 ====================
export const monitoringApi = {
  metrics: () => http.get<MetricsOverview>('/monitoring/metrics').then((r) => r.data),
  // 聚合 13 个业务指标（含 total_gpus / allocated_gpus），供 Dashboard KPI 使用。
  dashboard: () => http.get<MetricsOverview>('/monitoring/dashboard').then((r) => r.data),
  alerts: () => http.get<Alert[]>('/monitoring/alerts').then((r) => r.data),
}

// ==================== 多租户 ====================
export const tenantApi = {
  list: () => http.get<Tenant[]>('/tenants').then((r) => r.data),
  create: (data: Partial<Tenant>) =>
    http.post<Tenant>('/tenants', data).then((r) => r.data),
  update: (id: number, data: Partial<Tenant>) =>
    http.put<Tenant>(`/tenants/${id}`, data).then((r) => r.data),
  remove: (id: number) => http.delete<void>(`/tenants/${id}`).then((r) => r.data),
}

// ==================== 加速套件 ====================
export const accelerationApi = {
  list: () => http.get<AccelerationSuite[]>('/acceleration').then((r) => r.data),
  update: (id: number, data: Partial<AccelerationSuite>) =>
    http.put<AccelerationSuite>(`/acceleration/${id}`, data).then((r) => r.data),
}

// ==================== 安全管理 ====================
export const securityApi = {
  list: () => http.get<SecurityPolicy[]>('/security/policies').then((r) => r.data),
  update: (id: number, data: Partial<SecurityPolicy>) =>
    http.put<SecurityPolicy>(`/security/policies/${id}`, data).then((r) => r.data),
}

// ==================== GPU 细粒度管理 ====================
export const gpuApi = {
  devices: (params?: { cluster_id?: number; vendor?: string; status?: string; page_size?: number }) =>
    http.get<GPUDevice[]>('/gpu/devices', { params }).then((r) => r.data),
  device: (id: number) => http.get<GPUDevice>(`/gpu/devices/${id}`).then((r) => r.data),
  createDevice: (data: Partial<GPUDevice>) =>
    http.post<GPUDevice>('/gpu/devices', data).then((r) => r.data),
  updateDevice: (id: number, data: Partial<GPUDevice>) =>
    http.put<GPUDevice>(`/gpu/devices/${id}`, data).then((r) => r.data),
  deleteDevice: (id: number) =>
    http.delete<void>(`/gpu/devices/${id}`).then((r) => r.data),
  allocations: (params?: { job_id?: number }) =>
    http.get<GPUAllocation[]>('/gpu/allocations', { params }).then((r) => r.data),
  allocate: (data: Partial<GPUAllocation>) =>
    http.post<GPUAllocation>('/gpu/allocations', data).then((r) => r.data),
  release: (id: number) =>
    http.post<void>(`/gpu/allocations/${id}/release`).then((r) => r.data),
  utilization: (params?: { cluster_id?: number }) =>
    http.get<Record<string, unknown>>('/gpu/utilization', { params }).then((r) => r.data),
}

// ==================== 分区管理 ====================
export const partitionApi = {
  list: (params?: { cluster_id?: number; status?: string }) =>
    http.get<Partition[]>('/partitions', { params }).then((r) => r.data),
  get: (id: number) => http.get<Partition>(`/partitions/${id}`).then((r) => r.data),
  create: (data: Partial<Partition>) =>
    http.post<Partition>('/partitions', data).then((r) => r.data),
  update: (id: number, data: Partial<Partition>) =>
    http.put<Partition>(`/partitions/${id}`, data).then((r) => r.data),
  remove: (id: number) => http.delete<void>(`/partitions/${id}`).then((r) => r.data),
  updatePriority: (id: number, priority: number) =>
    http.put<Partition>(`/partitions/${id}/priority`, { priority }).then((r) => r.data),
  updateMaxRuntime: (id: number, max_runtime_minutes: number) =>
    http
      .put<Partition>(`/partitions/${id}/max-runtime`, { max_runtime_minutes })
      .then((r) => r.data),
  permissions: (partitionId: number) =>
    http.get<PartitionPermission[]>(`/partitions/${partitionId}/permissions`).then((r) => r.data),
  setPermission: (partition_id: number, data: Partial<PartitionPermission>) =>
    http
      .post<PartitionPermission>(`/partitions/${partition_id}/permissions`, data)
      .then((r) => r.data),
  removePermission: (id: number) =>
    http.delete<void>(`/partitions/permissions/${id}`).then((r) => r.data),
}

// ==================== 资源配额 ====================
export const quotaApi = {
  list: (params?: { scope_type?: string; scope_id?: number }) =>
    http.get<ResourceQuota[]>('/quotas', { params }).then((r) => r.data),
  get: (id: number) => http.get<ResourceQuota>(`/quotas/${id}`).then((r) => r.data),
  create: (data: Partial<ResourceQuota>) =>
    http.post<ResourceQuota>('/quotas', data).then((r) => r.data),
  update: (id: number, data: Partial<ResourceQuota>) =>
    http.put<ResourceQuota>(`/quotas/${id}`, data).then((r) => r.data),
  remove: (id: number) => http.delete<void>(`/quotas/${id}`).then((r) => r.data),
  usage: (params: { scope_type: string; scope_id: number }) =>
    http.get<Record<string, unknown>>('/quotas/usage', { params }).then((r) => r.data),
  check: (data: Record<string, unknown>) =>
    http.post<{ allowed: boolean; remaining: number }>('/quotas/check', data).then((r) => r.data),
}

// ==================== 调度器集成 ====================
export const schedulerApi = {
  list: () => http.get<SchedulerIntegration[]>('/schedulers').then((r) => r.data),
  get: (id: number) =>
    http.get<SchedulerIntegration>(`/schedulers/${id}`).then((r) => r.data),
  create: (data: Partial<SchedulerIntegration>) =>
    http.post<SchedulerIntegration>('/schedulers', data).then((r) => r.data),
  update: (id: number, data: Partial<SchedulerIntegration>) =>
    http.put<SchedulerIntegration>(`/schedulers/${id}`, data).then((r) => r.data),
  remove: (id: number) => http.delete<void>(`/schedulers/${id}`).then((r) => r.data),
  queues: (id: number) =>
    http.get<Record<string, unknown>[]>(`/schedulers/${id}/queues`).then((r) => r.data),
  nodes: (id: number) =>
    http.get<Record<string, unknown>[]>(`/schedulers/${id}/nodes`).then((r) => r.data),
  syncJobs: (id: number) =>
    http.post<Record<string, unknown>>(`/schedulers/${id}/sync`).then((r) => r.data),
  health: (id: number) =>
    http.get<{ healthy: boolean }>(`/schedulers/${id}/health`).then((r) => r.data),
}

// ==================== 节点拓扑 ====================
export const topologyApi = {
  list: (params?: { cluster_id?: number; page_size?: number }) =>
    http.get<NodeTopology[]>('/topology/nodes', { params }).then((r) => r.data),
  get: (id: number) => http.get<NodeTopology>(`/topology/nodes/${id}`).then((r) => r.data),
  create: (data: Partial<NodeTopology>) =>
    http.post<NodeTopology>('/topology/nodes', data).then((r) => r.data),
  update: (id: number, data: Partial<NodeTopology>) =>
    http.put<NodeTopology>(`/topology/nodes/${id}`, data).then((r) => r.data),
  remove: (id: number) =>
    http.delete<void>(`/topology/nodes/${id}`).then((r) => r.data),
  calculateScore: (data: Record<string, unknown>) =>
    http.post<Record<string, number>>('/topology/score', data).then((r) => r.data),
}

// ==================== 数据集与 Fluid 缓存 ====================
export const datasetApi = {
  list: () => http.get<Dataset[]>('/datasets').then((r) => r.data),
  get: (id: number) => http.get<Dataset>(`/datasets/${id}`).then((r) => r.data),
  create: (data: Partial<Dataset>) =>
    http.post<Dataset>('/datasets', data).then((r) => r.data),
  update: (id: number, data: Partial<Dataset>) =>
    http.put<Dataset>(`/datasets/${id}`, data).then((r) => r.data),
  remove: (id: number) => http.delete<void>(`/datasets/${id}`).then((r) => r.data),
  fluidCaches: (datasetId: number) =>
    http.get<FluidCache[]>(`/datasets/${datasetId}/fluid-caches`).then((r) => r.data),
  createFluidCache: (dataset_id: number, data: Partial<FluidCache>) =>
    http
      .post<FluidCache>(`/datasets/${dataset_id}/fluid-caches`, data)
      .then((r) => r.data),
  updateFluidCache: (cacheId: number, data: Partial<FluidCache>) =>
    http.put<FluidCache>(`/fluid-caches/${cacheId}`, data).then((r) => r.data),
  deleteFluidCache: (id: number) =>
    http.delete<void>(`/fluid-caches/${id}`).then((r) => r.data),
  enableFluidCache: (id: number) =>
    http.post<void>(`/fluid-caches/${id}/enable`).then((r) => r.data),
  disableFluidCache: (id: number) =>
    http.post<void>(`/fluid-caches/${id}/disable`).then((r) => r.data),
  triggerPrefetch: (id: number) =>
    http.post<void>(`/fluid-caches/${id}/prefetch`).then((r) => r.data),
}

// ==================== Checkpoint ====================
export const checkpointApi = {
  list: (params?: { job_id?: number }) =>
    http.get<Checkpoint[]>('/checkpoints', { params }).then((r) => r.data),
  get: (id: number) => http.get<Checkpoint>(`/checkpoints/${id}`).then((r) => r.data),
  create: (data: Partial<Checkpoint>) =>
    http.post<Checkpoint>('/checkpoints', data).then((r) => r.data),
  remove: (id: number) => http.delete<void>(`/checkpoints/${id}`).then((r) => r.data),
  latest: (jobId: number) =>
    http.get<Checkpoint>(`/checkpoints/latest/${jobId}`).then((r) => r.data),
}
