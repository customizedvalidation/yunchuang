import {
  createApi,
  fetchBaseQuery,
  type BaseQueryFn,
  type FetchArgs,
  type FetchBaseQueryError,
} from '@reduxjs/toolkit/query/react';

import type {
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
} from '../types';

// CSRF 双提交令牌：同源部署下从 csrf_token Cookie 读取；跨域部署下由 GET /auth/csrf
// 拉取后存入缓存。后端 NewCSRFProtect 校验「X-CSRF-Token 头 == csrf_token Cookie」一致。
let csrfTokenCache: string | null = null;

// setCsrfTokenCache 写入跨域场景下拉取到的 CSRF 令牌（同源场景直接读 Cookie，无需缓存）。
export function setCsrfTokenCache(token: string | null): void {
  csrfTokenCache = token;
}

function getCookie(name: string): string | null {
  if (typeof document === 'undefined') return null;
  const match = document.cookie.match(new RegExp('(^| )' + name + '=([^;]+)'));
  return match ? decodeURIComponent(match[2]) : null;
}

// getCsrfTokenValue 取当前可用的 CSRF 令牌：Cookie 优先，回退到跨域缓存。
function getCsrfTokenValue(): string | null {
  return getCookie('csrf_token') || csrfTokenCache;
}

// csrfHeaders 供裸 fetch（Sidebar/Topbar 退出按钮）注入 CSRF 头。
export function csrfHeaders(): Record<string, string> {
  const t = getCsrfTokenValue();
  return t ? { 'X-CSRF-Token': t } : {};
}

const baseQuery = fetchBaseQuery({
  baseUrl: '/api/v1',
  timeout: 30000,
  // 凭据随请求自动携带：登录后后端写入的 httpOnly access_token Cookie 由浏览器托管，
  // 前端不再用 JS 读取/存储 JWT（XSS 防护）。Bearer 头通道保留给非浏览器客户端。
  credentials: 'include',
  prepareHeaders: (headers) => {
    headers.set('Content-Type', 'application/json');
    headers.set('Accept', 'application/json');
    // 双提交令牌：状态变更请求携带 X-CSRF-Token 头，与 csrf_token Cookie 比对。
    const csrf = getCsrfTokenValue();
    if (csrf) headers.set('X-CSRF-Token', csrf);
    return headers;
  },
});

/**
 * 认证类端点返回 401 表示「凭证校验未通过」（如密码错误），是正常的业务结果；
 * 只有业务端点的 401 才代表会话失效。若不加区分，登录时输错密码会触发登出跳转，
 * 并因整页刷新而吞掉「用户名或密码错误」的提示。
 */
const AUTH_ENDPOINT_PATTERN = /^\/auth\//;

function isAuthEndpoint(args: string | FetchArgs): boolean {
  const url = typeof args === 'string' ? args : args.url;
  return AUTH_ENDPOINT_PATTERN.test(url);
}

const baseQueryWithReauth: BaseQueryFn<
  string | FetchArgs,
  unknown,
  FetchBaseQueryError
> = async (args, api, extraOptions) => {
  const result = await baseQuery(args, api, extraOptions);

  if (result.error && result.error.status === 401 && !isAuthEndpoint(args)) {
    // 会话失效：清理本地非敏感缓存（user/auth_expiry）并清除后端 httpOnly Cookie。
    localStorage.removeItem('user');
    localStorage.removeItem('auth_expiry');
    api.dispatch(apiSlice.endpoints.logout.initiate());
    // 已在登录页时不再跳转，避免并发 401 触发重复导航。
    if (window.location.pathname !== '/login') {
      window.location.href = '/login';
    }
  }

  return result;
};

/** 后端统一响应信封 { success, data, message, code, timestamp } */
export interface ApiEnvelope<T> {
  success: boolean;
  data: T;
  message?: string;
  code?: string;
  timestamp?: number;
}

/**
 * POST /auth/login 响应体中 data 的结构（与 services.LoginResponse 同形）。
 * 注意：token 仅用于向后兼容非浏览器客户端，前端不再读取/存储它，
 * 认证凭据改由后端写入的 httpOnly Cookie 承载。
 */
export interface LoginResponseData {
  token: string;
  user: {
    id: number;
    username: string;
    email: string;
    role: string;
    tenant_id: number;
    created_at?: string;
    updated_at?: string;
  };
  expires_at: number;
}

/**
 * POST /auth/refresh 响应体中 data 的结构。
 * 与 login 的 user 字段同源：后端 models.UserResponse（services.LoginResponse）。
 */
export interface RefreshTokenData {
  token: string;
  user: {
    id: number;
    username: string;
    email: string;
    role: string;
    tenant_id: number;
    created_at: string;
    updated_at: string;
  };
  expires_at: number;
}

export const apiSlice = createApi({
  reducerPath: 'api',
  baseQuery: baseQueryWithReauth,
  keepUnusedDataFor: 60,
  // 改为 false：避免每次路由切换都重拉全部 query（含常驻 Sidebar 的 5 个）。
  // 数据新鲜度由 60s 缓存 + 网络重连 refetch 保证；页面内仍有手动「刷新」按钮。
  refetchOnMountOrArgChange: false,
  refetchOnFocus: false,
  refetchOnReconnect: true,
  endpoints: (builder) => ({
    // 认证相关
    login: builder.mutation<ApiEnvelope<LoginResponseData>, { username: string; password: string }>({
      query: (credentials) => ({
        url: '/auth/login',
        method: 'POST',
        body: credentials,
      }),
      // 登录成功后拉取 CSRF 令牌（跨域部署下 Cookie 不可读，需经 GET /auth/csrf 取得）。
      onQueryStarted: async (_, { dispatch, queryFulfilled }) => {
        try {
          await queryFulfilled;
          dispatch(apiSlice.endpoints.getCsrfToken.initiate());
        } catch {
          // 登录失败无需处理
        }
      },
    }),
    // 登出：清除后端 httpOnly access_token Cookie（幂等）。前端在 401 兜底与退出按钮处调用。
    logout: builder.mutation<ApiEnvelope<void>, void>({
      query: () => ({
        url: '/auth/logout',
        method: 'POST',
      }),
    }),
    // 取 CSRF 双提交令牌：跨域部署下前端 JS 无法读 Cookie 时，带凭据 GET 取得后回传 X-CSRF-Token 头。
    getCsrfToken: builder.query<ApiEnvelope<{ csrf_token: string }>, void>({
      query: () => '/auth/csrf',
    }),
    register: builder.mutation({
      query: (userData) => ({
        url: '/auth/register',
        method: 'POST',
        body: userData,
      }),
    }),
    /**
     * 用「尚未过期」的令牌换取一份新令牌。
     *
     * 注意语义：该端点在后端同样挂在 JWTAuth 之后，令牌过期后调用只会 401，
     * 不存在"过期自救"路径。它的唯一用途是让活跃用户免于被 24h 有效期强制登出。
     * 落在 AUTH_ENDPOINT_PATTERN 内，因此失败产生的 401 不会触发整页跳登录。
     */
    refreshToken: builder.mutation<ApiEnvelope<RefreshTokenData>, void>({
      query: () => ({
        url: '/auth/refresh',
        method: 'POST',
      }),
    }),
    // 集群管理
    getClusters: builder.query({
      query: () => '/clusters',
    }),
    createCluster: builder.mutation({
      query: (clusterData) => ({
        url: '/clusters',
        method: 'POST',
        body: clusterData,
      }),
    }),
    updateCluster: builder.mutation({
      query: ({ id, ...clusterData }) => ({
        url: `/clusters/${id}`,
        method: 'PUT',
        body: clusterData,
      }),
    }),
    deleteCluster: builder.mutation({
      query: (id) => ({
        url: `/clusters/${id}`,
        method: 'DELETE',
      }),
    }),
    // 资源管理
    getResources: builder.query({
      query: () => '/resources',
    }),
    updateResource: builder.mutation({
      query: ({ id, ...resourceData }) => ({
        url: `/resources/${id}`,
        method: 'PUT',
        body: resourceData,
      }),
    }),
    // 作业管理
    getJobs: builder.query({
      query: () => '/jobs',
    }),
    createJob: builder.mutation({
      query: (jobData) => ({
        url: '/jobs',
        method: 'POST',
        body: jobData,
      }),
    }),
    cancelJob: builder.mutation({
      query: (id) => ({
        url: `/jobs/${id}/cancel`,
        method: 'POST',
      }),
    }),
    // 监控告警
    getMetrics: builder.query({
      query: () => '/monitoring/metrics',
    }),
    getAlerts: builder.query({
      query: () => '/monitoring/alerts',
    }),
    // 多租户管理
    getTenants: builder.query({
      query: () => '/tenants',
    }),
    createTenant: builder.mutation({
      query: (tenantData) => ({
        url: '/tenants',
        method: 'POST',
        body: tenantData,
      }),
    }),
    updateTenant: builder.mutation({
      query: ({ id, ...tenantData }) => ({
        url: `/tenants/${id}`,
        method: 'PUT',
        body: tenantData,
      }),
    }),
    deleteTenant: builder.mutation({
      query: (id) => ({
        url: `/tenants/${id}`,
        method: 'DELETE',
      }),
    }),
    // 加速套件管理
    getAccelerationSuites: builder.query({
      query: () => '/acceleration',
    }),
    updateAccelerationSuite: builder.mutation({
      query: ({ id, ...suiteData }) => ({
        url: `/acceleration/${id}`,
        method: 'PUT',
        body: suiteData,
      }),
    }),
    // 安全管理
    getSecurityPolicies: builder.query({
      query: () => '/security/policies',
    }),
    updateSecurityPolicy: builder.mutation({
      query: ({ id, ...policyData }) => ({
        url: `/security/policies/${id}`,
        method: 'PUT',
        body: policyData,
      }),
    }),
    // K8S管理
    submitJobToK8S: builder.mutation({
      query: (jobId) => ({
        url: `/jobs/${jobId}/submit`,
        method: 'POST',
      }),
    }),
    getK8SJobStatus: builder.query({
      query: (jobId) => ({
        url: `/jobs/${jobId}/status`,
      }),
    }),
    cancelK8SJob: builder.mutation({
      query: (jobId) => ({
        url: `/jobs/${jobId}/cancel`,
        method: 'POST',
      }),
    }),
    getK8SGPUResources: builder.query({
      query: () => '/resources/gpu',
    }),

    // ===== P0-1/P0-2: GPU 细粒度管理 =====
    getGPUDevices: builder.query<GPUDevice[], { cluster_id?: number; vendor?: string; status?: string }>({
      query: (params) => ({
        url: '/gpu/devices',
        params,
      }),
    }),
    getGPUDevice: builder.query<GPUDevice, number>({
      query: (id) => `/gpu/devices/${id}`,
    }),
    createGPUDevice: builder.mutation<GPUDevice, Partial<GPUDevice>>({
      query: (data) => ({
        url: '/gpu/devices',
        method: 'POST',
        body: data,
      }),
    }),
    updateGPUDevice: builder.mutation<GPUDevice, { id: number; data: Partial<GPUDevice> }>({
      query: ({ id, data }) => ({
        url: `/gpu/devices/${id}`,
        method: 'PUT',
        body: data,
      }),
    }),
    deleteGPUDevice: builder.mutation<void, number>({
      query: (id) => ({
        url: `/gpu/devices/${id}`,
        method: 'DELETE',
      }),
    }),
    getGPUAllocations: builder.query<GPUAllocation[], { job_id?: number }>({
      query: (params) => ({
        url: '/gpu/allocations',
        params,
      }),
    }),
    allocateGPU: builder.mutation<GPUAllocation, Partial<GPUAllocation>>({
      query: (data) => ({
        url: '/gpu/allocations',
        method: 'POST',
        body: data,
      }),
    }),
    releaseGPU: builder.mutation<void, number>({
      query: (id) => ({
        url: `/gpu/allocations/${id}/release`,
        method: 'POST',
      }),
    }),
    getGPUUtilization: builder.query<Record<string, unknown>, { cluster_id?: number }>({
      query: (params) => ({
        url: '/gpu/utilization',
        params,
      }),
    }),

    // ===== P0-4: 分区管理 =====
    getPartitions: builder.query<Partition[], { cluster_id?: number; status?: string }>({
      query: (params) => ({
        url: '/partitions',
        params,
      }),
    }),
    getPartition: builder.query<Partition, number>({
      query: (id) => `/partitions/${id}`,
    }),
    createPartition: builder.mutation<Partition, Partial<Partition>>({
      query: (data) => ({
        url: '/partitions',
        method: 'POST',
        body: data,
      }),
    }),
    updatePartition: builder.mutation<Partition, { id: number; data: Partial<Partition> }>({
      query: ({ id, data }) => ({
        url: `/partitions/${id}`,
        method: 'PUT',
        body: data,
      }),
    }),
    deletePartition: builder.mutation<void, number>({
      query: (id) => ({
        url: `/partitions/${id}`,
        method: 'DELETE',
      }),
    }),
    updatePartitionPriority: builder.mutation<Partition, { id: number; priority: number }>({
      query: ({ id, priority }) => ({
        url: `/partitions/${id}/priority`,
        method: 'PUT',
        body: { priority },
      }),
    }),
    updatePartitionMaxRuntime: builder.mutation<Partition, { id: number; max_runtime_minutes: number }>({
      query: ({ id, max_runtime_minutes }) => ({
        url: `/partitions/${id}/max-runtime`,
        method: 'PUT',
        body: { max_runtime_minutes },
      }),
    }),
    getPartitionPermissions: builder.query<PartitionPermission[], number>({
      query: (partitionId) => `/partitions/${partitionId}/permissions`,
    }),
    setPartitionPermission: builder.mutation<PartitionPermission, { partition_id: number; data: Partial<PartitionPermission> }>({
      query: ({ partition_id, data }) => ({
        url: `/partitions/${partition_id}/permissions`,
        method: 'POST',
        body: data,
      }),
    }),
    removePartitionPermission: builder.mutation<void, number>({
      query: (id) => ({
        url: `/partitions/permissions/${id}`,
        method: 'DELETE',
      }),
    }),

    // ===== P0-5: 资源配额 =====
    getQuotas: builder.query<ResourceQuota[], { scope_type?: string; scope_id?: number }>({
      query: (params) => ({
        url: '/quotas',
        params,
      }),
    }),
    getQuota: builder.query<ResourceQuota, number>({
      query: (id) => `/quotas/${id}`,
    }),
    createQuota: builder.mutation<ResourceQuota, Partial<ResourceQuota>>({
      query: (data) => ({
        url: '/quotas',
        method: 'POST',
        body: data,
      }),
    }),
    updateQuota: builder.mutation<ResourceQuota, { id: number; data: Partial<ResourceQuota> }>({
      query: ({ id, data }) => ({
        url: `/quotas/${id}`,
        method: 'PUT',
        body: data,
      }),
    }),
    deleteQuota: builder.mutation<void, number>({
      query: (id) => ({
        url: `/quotas/${id}`,
        method: 'DELETE',
      }),
    }),
    getQuotaUsage: builder.query<Record<string, unknown>, { scope_type: string; scope_id: number }>({
      query: (params) => ({
        url: '/quotas/usage',
        params,
      }),
    }),
    checkQuota: builder.mutation<{ allowed: boolean; remaining: number }, Record<string, unknown>>({
      query: (data) => ({
        url: '/quotas/check',
        method: 'POST',
        body: data,
      }),
    }),

    // ===== P0-3: 调度器集成 =====
    getSchedulerIntegrations: builder.query<SchedulerIntegration[], void>({
      query: () => '/schedulers',
    }),
    getSchedulerIntegration: builder.query<SchedulerIntegration, number>({
      query: (id) => `/schedulers/${id}`,
    }),
    createSchedulerIntegration: builder.mutation<SchedulerIntegration, Partial<SchedulerIntegration>>({
      query: (data) => ({
        url: '/schedulers',
        method: 'POST',
        body: data,
      }),
    }),
    updateSchedulerIntegration: builder.mutation<SchedulerIntegration, { id: number; data: Partial<SchedulerIntegration> }>({
      query: ({ id, data }) => ({
        url: `/schedulers/${id}`,
        method: 'PUT',
        body: data,
      }),
    }),
    deleteSchedulerIntegration: builder.mutation<void, number>({
      query: (id) => ({
        url: `/schedulers/${id}`,
        method: 'DELETE',
      }),
    }),
    getSchedulerQueues: builder.query<Record<string, unknown>[], number>({
      query: (id) => `/schedulers/${id}/queues`,
    }),
    getSchedulerNodes: builder.query<Record<string, unknown>[], number>({
      query: (id) => `/schedulers/${id}/nodes`,
    }),
    syncSchedulerJobs: builder.mutation<Record<string, unknown>, number>({
      query: (id) => ({
        url: `/schedulers/${id}/sync`,
        method: 'POST',
      }),
    }),
    getSchedulerHealth: builder.query<{ healthy: boolean }, number>({
      query: (id) => `/schedulers/${id}/health`,
    }),

    // ===== P1-1: 节点拓扑 =====
    getNodeTopologies: builder.query<NodeTopology[], { cluster_id?: number }>({
      query: (params) => ({
        url: '/topology/nodes',
        params,
      }),
    }),
    getNodeTopology: builder.query<NodeTopology, number>({
      query: (id) => `/topology/nodes/${id}`,
    }),
    createNodeTopology: builder.mutation<NodeTopology, Partial<NodeTopology>>({
      query: (data) => ({
        url: '/topology/nodes',
        method: 'POST',
        body: data,
      }),
    }),
    updateNodeTopology: builder.mutation<NodeTopology, { id: number; data: Partial<NodeTopology> }>({
      query: ({ id, data }) => ({
        url: `/topology/nodes/${id}`,
        method: 'PUT',
        body: data,
      }),
    }),
    deleteNodeTopology: builder.mutation<void, number>({
      query: (id) => ({
        url: `/topology/nodes/${id}`,
        method: 'DELETE',
      }),
    }),
    calculateTopologyScore: builder.mutation<Record<string, number>, Record<string, unknown>>({
      query: (data) => ({
        url: '/topology/score',
        method: 'POST',
        body: data,
      }),
    }),

    // ===== P1-5: 数据集与 Fluid 缓存 =====
    getDatasets: builder.query<Dataset[], void>({
      query: () => '/datasets',
    }),
    getDataset: builder.query<Dataset, number>({
      query: (id) => `/datasets/${id}`,
    }),
    createDataset: builder.mutation<Dataset, Partial<Dataset>>({
      query: (data) => ({
        url: '/datasets',
        method: 'POST',
        body: data,
      }),
    }),
    updateDataset: builder.mutation<Dataset, { id: number; data: Partial<Dataset> }>({
      query: ({ id, data }) => ({
        url: `/datasets/${id}`,
        method: 'PUT',
        body: data,
      }),
    }),
    deleteDataset: builder.mutation<void, number>({
      query: (id) => ({
        url: `/datasets/${id}`,
        method: 'DELETE',
      }),
    }),
    getFluidCaches: builder.query<FluidCache[], number>({
      query: (datasetId) => `/datasets/${datasetId}/fluid-caches`,
    }),
    createFluidCache: builder.mutation<FluidCache, { dataset_id: number; data: Partial<FluidCache> }>({
      query: ({ dataset_id, data }) => ({
        url: `/datasets/${dataset_id}/fluid-caches`,
        method: 'POST',
        body: data,
      }),
    }),
    updateFluidCache: builder.mutation<FluidCache, { cacheId: number; data: Partial<FluidCache> }>({
      query: ({ cacheId, data }) => ({
        url: `/fluid-caches/${cacheId}`,
        method: 'PUT',
        body: data,
      }),
    }),
    deleteFluidCache: builder.mutation<void, number>({
      query: (id) => ({
        url: `/fluid-caches/${id}`,
        method: 'DELETE',
      }),
    }),
    enableFluidCache: builder.mutation<void, number>({
      query: (id) => ({
        url: `/fluid-caches/${id}/enable`,
        method: 'POST',
      }),
    }),
    disableFluidCache: builder.mutation<void, number>({
      query: (id) => ({
        url: `/fluid-caches/${id}/disable`,
        method: 'POST',
      }),
    }),
    triggerPrefetch: builder.mutation<void, number>({
      query: (id) => ({
        url: `/fluid-caches/${id}/prefetch`,
        method: 'POST',
      }),
    }),

    // ===== P1-4: Checkpoint =====
    getCheckpoints: builder.query<Checkpoint[], { job_id?: number }>({
      query: (params) => ({
        url: '/checkpoints',
        params,
      }),
    }),
    getCheckpoint: builder.query<Checkpoint, number>({
      query: (id) => `/checkpoints/${id}`,
    }),
    createCheckpoint: builder.mutation<Checkpoint, Partial<Checkpoint>>({
      query: (data) => ({
        url: '/checkpoints',
        method: 'POST',
        body: data,
      }),
    }),
    deleteCheckpoint: builder.mutation<void, number>({
      query: (id) => ({
        url: `/checkpoints/${id}`,
        method: 'DELETE',
      }),
    }),
    getLatestCheckpoint: builder.query<Checkpoint, number>({
      query: (jobId) => `/checkpoints/latest/${jobId}`,
    }),
  }),
});

// 导出API hooks
export const {
  useLoginMutation,
  useRegisterMutation,
  useLogoutMutation,
  useGetCsrfTokenQuery,
  useRefreshTokenMutation,
  useGetClustersQuery,
  useCreateClusterMutation,
  useUpdateClusterMutation,
  useDeleteClusterMutation,
  useGetResourcesQuery,
  useUpdateResourceMutation,
  useGetJobsQuery,
  useCreateJobMutation,
  useCancelJobMutation,
  useGetMetricsQuery,
  useGetAlertsQuery,
  useGetTenantsQuery,
  useCreateTenantMutation,
  useUpdateTenantMutation,
  useDeleteTenantMutation,
  useGetAccelerationSuitesQuery,
  useUpdateAccelerationSuiteMutation,
  useGetSecurityPoliciesQuery,
  useUpdateSecurityPolicyMutation,
  useSubmitJobToK8SMutation,
  useGetK8SJobStatusQuery,
  useCancelK8SJobMutation,
  useGetK8SGPUResourcesQuery,
  // GPU
  useGetGPUDevicesQuery,
  useGetGPUDeviceQuery,
  useCreateGPUDeviceMutation,
  useUpdateGPUDeviceMutation,
  useDeleteGPUDeviceMutation,
  useGetGPUAllocationsQuery,
  useAllocateGPUMutation,
  useReleaseGPUMutation,
  useGetGPUUtilizationQuery,
  // 分区
  useGetPartitionsQuery,
  useGetPartitionQuery,
  useCreatePartitionMutation,
  useUpdatePartitionMutation,
  useDeletePartitionMutation,
  useUpdatePartitionPriorityMutation,
  useUpdatePartitionMaxRuntimeMutation,
  useGetPartitionPermissionsQuery,
  useSetPartitionPermissionMutation,
  useRemovePartitionPermissionMutation,
  // 配额
  useGetQuotasQuery,
  useGetQuotaQuery,
  useCreateQuotaMutation,
  useUpdateQuotaMutation,
  useDeleteQuotaMutation,
  useGetQuotaUsageQuery,
  useCheckQuotaMutation,
  // 调度器
  useGetSchedulerIntegrationsQuery,
  useGetSchedulerIntegrationQuery,
  useCreateSchedulerIntegrationMutation,
  useUpdateSchedulerIntegrationMutation,
  useDeleteSchedulerIntegrationMutation,
  useGetSchedulerQueuesQuery,
  useGetSchedulerNodesQuery,
  useSyncSchedulerJobsMutation,
  useGetSchedulerHealthQuery,
  // 拓扑
  useGetNodeTopologiesQuery,
  useGetNodeTopologyQuery,
  useCreateNodeTopologyMutation,
  useUpdateNodeTopologyMutation,
  useDeleteNodeTopologyMutation,
  useCalculateTopologyScoreMutation,
  // 数据集
  useGetDatasetsQuery,
  useGetDatasetQuery,
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
  // Checkpoint
  useGetCheckpointsQuery,
  useGetCheckpointQuery,
  useCreateCheckpointMutation,
  useDeleteCheckpointMutation,
  useGetLatestCheckpointQuery,
} = apiSlice;
