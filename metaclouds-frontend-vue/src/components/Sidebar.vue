<template>
  <aside class="mc-sidebar" :class="{ collapsed }">
    <div class="mc-sidebar-logo">
      <div class="mc-sidebar-logo-icon">
        <el-icon><Cpu /></el-icon>
      </div>
      <div class="mc-sidebar-logo-text">
        <span class="mc-sidebar-logo-title">Metaclouds</span>
        <span class="mc-sidebar-logo-subtitle">算力调度平台</span>
      </div>
    </div>

    <button class="mc-sidebar-collapse-btn" @click="$emit('collapse', !collapsed)" :aria-label="collapsed ? '展开侧边栏' : '折叠侧边栏'">
      <el-icon><ArrowLeft v-if="!collapsed" /><ArrowRight v-else /></el-icon>
    </button>

    <nav class="mc-sidebar-menu" aria-label="主导航">
      <template v-for="group in visibleMenu" :key="group.label">
        <div class="mc-sidebar-group-title">{{ group.label }}</div>
        <template v-for="item in group.children" :key="item.key">
          <!-- 有子菜单 -->
          <template v-if="item.children && item.children.length">
            <div class="mc-sidebar-item" :class="{ active: isParentActive(item) }" @click="toggleSubmenu(item.key!)">
              <el-icon class="mc-sidebar-item-icon"><component :is="item.icon" /></el-icon>
              <div class="mc-sidebar-item-content">
                <div class="mc-sidebar-item-label">
                  <span>{{ item.label }}</span>
                  <span v-if="badgeMap[item.key!]" class="mc-sidebar-badge" :class="badgeColor(item.key!, badgeMap[item.key!])">{{ badgeMap[item.key!] }}</span>
                </div>
                <span v-if="item.description" class="mc-sidebar-item-desc">{{ item.description }}</span>
              </div>
              <el-icon :size="12" :style="{ transform: openKeys.includes(item.key!) ? 'rotate(90deg)' : '', transition: 'transform 0.2s' }"><ArrowRight /></el-icon>
            </div>
            <div v-show="openKeys.includes(item.key!)" class="mc-sidebar-submenu">
              <router-link
                v-for="child in item.children"
                :key="child.key"
                :to="child.key!"
                class="mc-sidebar-item"
                :class="{ active: route.path === child.key }"
              >
                <div class="mc-sidebar-item-content">
                  <div class="mc-sidebar-item-label">
                    <span>{{ child.label }}</span>
                    <span v-if="badgeMap[child.key!]" class="mc-sidebar-badge" :class="badgeColor(child.key!, badgeMap[child.key!])">{{ badgeMap[child.key!] }}</span>
                  </div>
                </div>
              </router-link>
            </div>
          </template>
          <!-- 叶子节点 -->
          <router-link
            v-else
            :to="item.key!"
            class="mc-sidebar-item"
            :class="{ active: route.path === item.key }"
          >
            <el-icon class="mc-sidebar-item-icon"><component :is="item.icon" /></el-icon>
            <div class="mc-sidebar-item-content">
              <div class="mc-sidebar-item-label">
                <span>{{ item.label }}</span>
                <span v-if="badgeMap[item.key!]" class="mc-sidebar-badge" :class="badgeColor(item.key!, badgeMap[item.key!])">{{ badgeMap[item.key!] }}</span>
              </div>
              <span v-if="item.description" class="mc-sidebar-item-desc">{{ item.description }}</span>
            </div>
          </router-link>
        </template>
      </template>
    </nav>

    <button class="mc-sidebar-logout" @click="handleLogout">
      <el-icon><SwitchButton /></el-icon>
      <span>退出登录</span>
    </button>
  </aside>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  Cpu, ArrowLeft, ArrowRight, SwitchButton,
  Odometer, Tickets, Cloudy, Lightning, Grid,
  Connection, Box, Share, House,
  Promotion, FolderOpened, Bell, UserFilled, Lock,
} from '@element-plus/icons-vue'
import type { MenuItem, UserRole, Job } from '@/types'
import { isRoleAllowed, readStoredRole } from '@/utils/auth'
import { jobApi, clusterApi, resourceApi, tenantApi, gpuApi, partitionApi, schedulerApi, datasetApi, monitoringApi } from '@/api'
import { csrfHeaders } from '@/api/http'

defineProps<{ collapsed: boolean }>()
defineEmits<{ (e: 'collapse', v: boolean): void }>()

const route = useRoute()
const router = useRouter()

// 菜单配置（与 React 版 Sidebar.tsx menuItems 对齐）
const menuItems: MenuItem[] = [
  {
    type: 'group', label: '总览',
    children: [
      { key: '/dashboard', icon: 'Odometer', label: '仪表盘', description: '算力总览' },
    ],
  },
  {
    type: 'group', label: '作业调度',
    children: [
      {
        key: '/job', icon: 'Tickets', label: '作业管理', description: '作业调度',
        children: [
          { key: '/job/list', label: '作业列表' },
          { key: '/job/queue', label: '任务队列' },
          { key: '/job/history', label: '历史记录' },
        ],
      },
    ],
  },
  {
    type: 'group', label: '基础资源',
    children: [
      { key: '/resource', icon: 'Cloudy', label: '资源管理', description: '资源分配' },
      { key: '/gpus', icon: 'Lightning', label: 'GPU 设备', description: '细粒度分配' },
      { key: '/partitions', icon: 'Grid', label: '分区管理', description: '分区配额' },
    ],
  },
  {
    type: 'group', label: '集群管理',
    children: [
      { key: '/cluster', icon: 'Connection', label: '集群管理', description: '高可用集群' },
      {
        key: '/k8s', icon: 'Box', label: 'K8S 管理', description: '容器编排',
        children: [
          { key: '/k8s/nodes', label: '节点管理' },
          { key: '/k8s/pods', label: 'Pod 管理' },
          { key: '/k8s/services', label: '服务管理' },
        ],
      },
      { key: '/schedulers', icon: 'Share', label: '调度器', description: 'Slurm/LSF' },
      { key: '/topology', icon: 'House', label: '拓扑感知', description: '网络拓扑' },
    ],
  },
  {
    type: 'group', label: '加速套件',
    children: [
      { key: '/acceleration', icon: 'Promotion', label: '加速套件', description: '推理加速' },
      { key: '/datasets', icon: 'FolderOpened', label: '数据集', description: 'Fluid 加速' },
    ],
  },
  {
    type: 'group', label: '系统治理',
    children: [
      { key: '/monitoring', icon: 'Bell', label: '监控告警', description: '实时监控' },
      { key: '/tenant', icon: 'UserFilled', label: '多租户', description: '租户配额', roles: ['admin', 'manager'] },
      { key: '/security', icon: 'Lock', label: '安全管理', description: '安全策略' },
    ],
  },
]

// 图标映射
const iconMap: Record<string, unknown> = {
  Odometer, Tickets, Cloudy, Lightning, Grid,
  Connection, Box, Share, House,
  Promotion, FolderOpened, Bell, UserFilled, Lock,
}

const currentRole = computed<UserRole | null>(() => readStoredRole())

// 按角色过滤菜单
const visibleMenu = computed(() => {
  const role = currentRole.value
  const result: MenuItem[] = []
  for (const group of menuItems) {
    if (group.type !== 'group') continue
    const children = (group.children || []).filter((item) => {
      if (item.roles && role && !isRoleAllowed(role, item.roles)) return false
      return true
    })
    if (children.length > 0) {
      result.push({ ...group, children })
    }
  }
  return result
})

// 默认展开全部子菜单
const openKeys = ref<string[]>([])
function collectOpenKeys() {
  const keys: string[] = []
  for (const group of visibleMenu.value) {
    for (const item of group.children || []) {
      if (item.children?.length && item.key) keys.push(item.key)
    }
  }
  openKeys.value = keys
}
onMounted(collectOpenKeys)
watch(visibleMenu, collectOpenKeys)

function toggleSubmenu(key: string) {
  const idx = openKeys.value.indexOf(key)
  if (idx >= 0) openKeys.value.splice(idx, 1)
  else openKeys.value.push(key)
}

function isParentActive(item: MenuItem): boolean {
  if (!item.key) return false
  return route.path === item.key || route.path.startsWith(item.key + '/')
}

// 徽标数据
const badgeMap = ref<Record<string, number>>({})

function badgeColor(key: string, count: number): string {
  if (!count) return ''
  if (key === '/monitoring') return count >= 5 ? 'danger' : 'warning'
  if (key === '/job/queue') return ''
  if (key === '/k8s/pods') return 'success'
  if (key === '/job/history') return 'muted'
  return ''
}

async function loadBadges() {
  try {
    const [jobs, clusters, resources, tenants, gpus, partitions, schedulers, datasets, alerts] = await Promise.all([
      jobApi.list().catch(() => [] as Job[]),
      clusterApi.list().catch(() => []),
      resourceApi.list().catch(() => []),
      tenantApi.list().catch(() => []),
      gpuApi.devices({}).catch(() => []),
      partitionApi.list({}).catch(() => []),
      schedulerApi.list().catch(() => []),
      datasetApi.list().catch(() => []),
      monitoringApi.alerts().catch(() => []),
    ])
    const jobsArr = jobs as Job[]
    badgeMap.value = {
      '/job': jobsArr.length,
      '/job/queue': jobsArr.filter((j) => j.status === 'pending').length,
      '/job/history': jobsArr.filter((j) => ['completed', 'failed', 'cancelled'].includes(j.status)).length,
      '/k8s/pods': jobsArr.filter((j) => j.status === 'running').length,
      '/cluster': (clusters as unknown[]).length,
      '/resource': (resources as unknown[]).length,
      '/tenant': (tenants as unknown[]).length,
      '/monitoring': (alerts as unknown[]).length,
      '/gpus': (gpus as unknown[]).length,
      '/partitions': (partitions as unknown[]).length,
      '/schedulers': (schedulers as unknown[]).length,
      '/datasets': (datasets as unknown[]).length,
    }
  } catch {
    // 徽标加载失败不影响页面
  }
}
onMounted(loadBadges)

// 退出登录
function handleLogout() {
  fetch('/api/v1/auth/logout', {
    method: 'POST',
    credentials: 'include',
    headers: { 'Content-Type': 'application/json', ...csrfHeaders() },
  })
    .catch(() => undefined)
    .finally(() => {
      localStorage.removeItem('user')
      localStorage.removeItem('auth_expiry')
      router.push('/login')
    })
}
</script>

<style src="@/styles/sidebar.css"></style>
