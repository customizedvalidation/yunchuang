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
            <div
              class="mc-sidebar-item"
              :class="{ active: isParentActive(item) }"
              role="button"
              tabindex="0"
              :aria-expanded="openKeys.includes(item.key!)"
              @click="toggleSubmenu(item.key!)"
              @keydown.enter="toggleSubmenu(item.key!)"
              @keydown.space.prevent="toggleSubmenu(item.key!)"
            >
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
import { ref, computed, onMounted, onUnmounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import {
  Cpu, ArrowLeft, ArrowRight, SwitchButton,
  Odometer, Tickets, Cloudy, Lightning, Grid,
  Connection, Box, Share, House,
  Promotion, FolderOpened, Bell, UserFilled, Lock,
} from '@element-plus/icons-vue'
import type { UserRole, Job } from '@/types'
import { navConfig, type NavItem, type NavGroup } from '@/nav'
import { isRoleAllowed } from '@/utils/auth'
import { jobApi, clusterApi, resourceApi, tenantApi, gpuApi, partitionApi, schedulerApi, datasetApi, monitoringApi } from '@/api'
import { useAuthStore } from '@/stores/auth'

// 注：图标名称 → 组件映射保留（iconMap），与 navConfig 的 icon 字符串名一一对应。
const iconMap: Record<string, unknown> = {
  Odometer, Tickets, Cloudy, Lightning, Grid,
  Connection, Box, Share, House,
  Promotion, FolderOpened, Bell, UserFilled, Lock,
}

defineProps<{ collapsed: boolean }>()
defineEmits<{ (e: 'collapse', v: boolean): void }>()

const auth = useAuthStore()
const route = useRoute()
const router = useRouter()

// 菜单数据统一收口到 @/nav 的 navConfig（与 Topbar 面包屑、router 角色守卫同源）

// 角色来源统一为 Pinia store（与路由守卫单一真相一致）
const currentRole = computed<UserRole | null>(() => auth.role)

// 按角色过滤菜单（与 navConfig 同源）
const visibleMenu = computed<NavGroup[]>(() => {
  const role = currentRole.value
  const result: NavGroup[] = []
  for (const group of navConfig) {
    const children = group.children.filter((item) => {
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

function isParentActive(item: NavItem): boolean {
  if (!item.key) return false
  return route.path === item.key || route.path.startsWith(item.key + '/')
}

// 徽标数据
const badgeMap = ref<Record<string, number>>({})

function badgeColor(key: string, count: number): string {
  if (!count) return ''
  if (key === '/monitoring') return count >= 5 ? 'danger' : 'warning'
  if (key === '/job/queue') return ''
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
      gpuApi.devices({ page_size: 1000 }).catch(() => []),
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
      // 注：/k8s/pods 不再展示“运行中作业数”徽标——后端无真实 Pod 接口，
      // 曾用运行中作业数填充会误导用户以为这是 Pod 数量，故移除该误导徽标。
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

// 徽标：挂载即拉取一次，并每 30s 定时刷新（失败不影响页面），卸载清理定时器
const BADGE_REFRESH_MS = 30_000
let badgeTimer: number | null = null
onMounted(() => {
  void loadBadges()
  badgeTimer = window.setInterval(() => void loadBadges(), BADGE_REFRESH_MS)
})
onUnmounted(() => {
  if (badgeTimer !== null) window.clearInterval(badgeTimer)
})

// 退出登录
function handleLogout() {
  // 统一走 store：清 httpOnly Cookie + 清空内存中的 user，
  // 旧实现只删 localStorage，user 残留导致登出后 isLoggedIn 仍为 true。
  void auth.logout().finally(() => router.push('/login'))
}
</script>

<style src="@/styles/sidebar.css"></style>
