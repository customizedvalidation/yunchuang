import { createRouter, createWebHistory, type RouteRecordRaw } from 'vue-router'
import { useAuthStore } from '@/stores/auth'

const routes: RouteRecordRaw[] = [
  { path: '/login', name: 'Login', component: () => import('@/pages/Login.vue') },
  {
    path: '/',
    component: () => import('@/components/Layout.vue'),
    meta: { requiresAuth: true },
    children: [
      { path: '', redirect: '/dashboard' },
      { path: 'dashboard', name: 'Dashboard', component: () => import('@/pages/Dashboard.vue') },
      { path: 'cluster', name: 'Cluster', component: () => import('@/pages/ClusterManagement.vue') },
      { path: 'resource', name: 'Resource', component: () => import('@/pages/ResourceManagement.vue') },
      { path: 'job', name: 'Job', component: () => import('@/pages/JobManagement.vue') },
      { path: 'job/list', name: 'JobList', component: () => import('@/pages/JobManagement.vue') },
      { path: 'job/queue', name: 'JobQueue', component: () => import('@/pages/JobManagement.vue') },
      { path: 'job/history', name: 'JobHistory', component: () => import('@/pages/JobManagement.vue') },
      { path: 'monitoring', name: 'Monitoring', component: () => import('@/pages/MonitoringAlert.vue') },
      {
        path: 'tenant',
        name: 'Tenant',
        component: () => import('@/pages/MultiTenantManagement.vue'),
        meta: { roles: ['admin', 'manager'] as const },
      },
      { path: 'acceleration', name: 'Acceleration', component: () => import('@/pages/AccelerationSuiteManagement.vue') },
      { path: 'security', name: 'Security', component: () => import('@/pages/SecurityManagement.vue') },
      { path: 'k8s', name: 'K8S', component: () => import('@/pages/K8SManagement.vue') },
      { path: 'k8s/nodes', name: 'K8SNodes', component: () => import('@/pages/K8SManagement.vue') },
      { path: 'k8s/pods', name: 'K8SPods', component: () => import('@/pages/K8SManagement.vue') },
      { path: 'k8s/services', name: 'K8SServices', component: () => import('@/pages/K8SManagement.vue') },
      { path: 'gpus', name: 'GPUs', component: () => import('@/pages/GPUManagement.vue') },
      { path: 'partitions', name: 'Partitions', component: () => import('@/pages/PartitionManagement.vue') },
      { path: 'schedulers', name: 'Schedulers', component: () => import('@/pages/SchedulerManagement.vue') },
      { path: 'topology', name: 'Topology', component: () => import('@/pages/TopologyManagement.vue') },
      { path: 'datasets', name: 'Datasets', component: () => import('@/pages/DatasetManagement.vue') },
    ],
  },
  { path: '/:pathMatch(.*)*', redirect: '/dashboard' },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

router.beforeEach((to) => {
  const auth = useAuthStore()
  if (to.meta.requiresAuth && !auth.isLoggedIn) {
    return { path: '/login', query: { redirect: to.fullPath } }
  }
  // 角色守卫
  const roles = to.meta.roles as readonly string[] | undefined
  if (roles && auth.role && !roles.includes(auth.role)) {
    return { path: '/dashboard' }
  }
  return true
})

export default router
