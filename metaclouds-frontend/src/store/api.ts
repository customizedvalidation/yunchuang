import { createApi, fetchBaseQuery } from '@reduxjs/toolkit/query/react';

const baseQuery = fetchBaseQuery({ 
  baseUrl: '/api/v1',
  timeout: 30000,
  prepareHeaders: (headers) => {
    const token = localStorage.getItem('token');
    if (token) {
      headers.set('Authorization', `Bearer ${token}`);
    }
    headers.set('Content-Type', 'application/json');
    headers.set('Accept', 'application/json');
    return headers;
  },
});

const baseQueryWithReauth = async (args: any, api: any, extraOptions: any) => {
  const result = await baseQuery(args, api, extraOptions);
  
  if (result.error && result.error.status === 401) {
    localStorage.removeItem('token');
    localStorage.removeItem('user');
    window.location.href = '/login';
  }
  
  return result;
};

export const apiSlice = createApi({
  reducerPath: 'api',
  baseQuery: baseQueryWithReauth,
  keepUnusedDataFor: 60,
  refetchOnMountOrArgChange: true,
  refetchOnFocus: false,
  refetchOnReconnect: true,
  endpoints: (builder) => ({
    // 认证相关
    login: builder.mutation({
      query: (credentials) => ({
        url: '/auth/login',
        method: 'POST',
        body: credentials,
      }),
    }),
    register: builder.mutation({
      query: (userData) => ({
        url: '/auth/register',
        method: 'POST',
        body: userData,
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