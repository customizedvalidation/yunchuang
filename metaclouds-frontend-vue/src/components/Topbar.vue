<template>
  <header class="mc-topbar">
    <div class="mc-topbar-left">
      <button v-if="isMobile" class="mc-topbar-menu-btn" @click="$emit('menuClick')" aria-label="打开菜单">
        <el-icon><Menu /></el-icon>
      </button>
      <el-breadcrumb separator="/">
        <el-breadcrumb-item :to="{ path: '/dashboard' }">首页</el-breadcrumb-item>
        <el-breadcrumb-item>{{ currentTitle }}</el-breadcrumb-item>
      </el-breadcrumb>
    </div>
    <div class="mc-topbar-right">
      <el-tooltip content="刷新数据" placement="bottom">
        <button class="mc-topbar-icon-btn" @click="$emit('refresh')" aria-label="刷新">
          <el-icon><Refresh /></el-icon>
        </button>
      </el-tooltip>
      <el-dropdown trigger="click" @command="handleCommand">
        <div class="mc-topbar-user">
          <el-avatar :size="32" class="mc-topbar-avatar">{{ avatarText }}</el-avatar>
          <div class="mc-topbar-user-info">
            <span class="mc-topbar-username">{{ user?.username || '用户' }}</span>
            <span class="mc-topbar-role">{{ roleLabel }}</span>
          </div>
          <el-icon><ArrowDown /></el-icon>
        </div>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="profile">个人信息</el-dropdown-item>
            <el-dropdown-item command="logout" divided>退出登录</el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Menu, Refresh, ArrowDown } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { csrfHeaders } from '@/api/http'

defineEmits<{ (e: 'menuClick'): void; (e: 'refresh'): void }>()

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()

const user = computed(() => auth.user)
const isMobile = computed(() => window.innerWidth < 1024)

const avatarText = computed(() => {
  const name = user.value?.username || 'U'
  return name.charAt(0).toUpperCase()
})

const roleLabel = computed(() => {
  const role = user.value?.role
  if (role === 'admin') return '管理员'
  if (role === 'manager') return '运维经理'
  return '普通用户'
})

const titleMap: Record<string, string> = {
  '/dashboard': '仪表盘',
  '/cluster': '集群管理',
  '/resource': '资源管理',
  '/job': '作业管理',
  '/job/list': '作业列表',
  '/job/queue': '任务队列',
  '/job/history': '历史记录',
  '/monitoring': '监控告警',
  '/tenant': '多租户管理',
  '/acceleration': '加速套件',
  '/security': '安全管理',
  '/k8s': 'K8S 管理',
  '/k8s/nodes': '节点管理',
  '/k8s/pods': 'Pod 管理',
  '/k8s/services': '服务管理',
  '/gpus': 'GPU 设备',
  '/partitions': '分区管理',
  '/schedulers': '调度器',
  '/topology': '拓扑感知',
  '/datasets': '数据集',
}

const currentTitle = computed(() => titleMap[route.path] || 'Metaclouds')

function handleCommand(command: string) {
  if (command === 'logout') {
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
}
</script>

<style scoped>
.mc-topbar {
  height: 56px;
  background: var(--mc-surface);
  border-bottom: 1px solid var(--mc-line);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 20px;
  position: sticky;
  top: 0;
  z-index: 100;
}
.mc-topbar-left { display: flex; align-items: center; gap: 16px; }
.mc-topbar-menu-btn {
  background: none; border: none; cursor: pointer;
  font-size: 18px; color: var(--mc-text-2);
  padding: 6px; border-radius: 6px;
}
.mc-topbar-menu-btn:hover { background: var(--mc-surface-3); }
.mc-topbar-right { display: flex; align-items: center; gap: 12px; }
.mc-topbar-icon-btn {
  background: none; border: none; cursor: pointer;
  font-size: 16px; color: var(--mc-text-2);
  padding: 8px; border-radius: 8px;
  transition: all 0.2s;
}
.mc-topbar-icon-btn:hover { background: var(--mc-surface-3); color: var(--mc-brand); }
.mc-topbar-user {
  display: flex; align-items: center; gap: 10px;
  cursor: pointer; padding: 4px 8px; border-radius: 8px;
  transition: background 0.2s;
}
.mc-topbar-user:hover { background: var(--mc-surface-3); }
.mc-topbar-avatar {
  background: linear-gradient(135deg, #1677ff 0%, #0958d9 100%);
  color: #fff; font-weight: 600; font-size: 14px;
}
.mc-topbar-user-info { display: flex; flex-direction: column; line-height: 1.2; }
.mc-topbar-username { font-size: 13px; font-weight: 500; color: var(--mc-text-1); }
.mc-topbar-role { font-size: 11px; color: var(--mc-text-3); }
</style>
