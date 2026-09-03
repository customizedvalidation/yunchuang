import {
  createApi,
  fetchBaseQuery,
  type BaseQueryFn,
  type FetchArgs,
  type FetchBaseQueryError,
} from '@reduxjs/toolkit/query/react';

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
} = apiSlice;