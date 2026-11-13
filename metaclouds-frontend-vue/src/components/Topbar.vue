<template>
  <header class="mc-topbar">
    <div class="mc-topbar-left">
      <button type="button" v-if="isMobile" class="mc-topbar-icon-btn" @click="$emit('menuClick')" aria-label="打开菜单">
        <el-icon><Menu /></el-icon>
      </button>
      <el-breadcrumb separator="/" aria-label="面包屑导航">
        <el-breadcrumb-item :to="{ path: '/dashboard' }">首页</el-breadcrumb-item>
        <el-breadcrumb-item aria-current="page">{{ currentTitle }}</el-breadcrumb-item>
      </el-breadcrumb>
    </div>
    <div class="mc-topbar-right">
      <el-tooltip content="刷新数据" placement="bottom">
        <button type="button" class="mc-topbar-icon-btn" @click="$emit('refresh')" aria-label="刷新数据">
          <el-icon><Refresh /></el-icon>
        </button>
      </el-tooltip>
      <ThemeToggle />
      <el-dropdown trigger="click" @command="handleCommand">
        <button type="button" class="mc-topbar-user" aria-label="账户菜单" aria-haspopup="menu">
          <el-avatar :size="30" class="mc-topbar-avatar">{{ avatarText }}</el-avatar>
          <span class="mc-topbar-user-info">
            <span class="mc-topbar-username">{{ user?.username || '用户' }}</span>
            <span class="mc-topbar-role">{{ roleLabel }}</span>
          </span>
          <el-icon class="mc-topbar-caret"><ArrowDown /></el-icon>
        </button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item command="profile">账户信息</el-dropdown-item>
            <el-dropdown-item command="logout" divided>退出登录</el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>

    <!-- 「账户信息」此前是点击无响应的死链接，现补齐为真实只读信息弹层 -->
    <el-dialog v-model="profileVisible" title="账户信息" width="440px" align-center>
      <el-descriptions :column="1" border>
        <el-descriptions-item label="用户名">{{ user?.username || '—' }}</el-descriptions-item>
        <el-descriptions-item label="角色">{{ roleLabel }}</el-descriptions-item>
        <el-descriptions-item label="邮箱">{{ user?.email || '未填写' }}</el-descriptions-item>
        <el-descriptions-item label="租户 ID">{{ user?.tenant_id ?? '默认租户' }}</el-descriptions-item>
        <el-descriptions-item label="会话有效期">{{ sessionExpiry }}</el-descriptions-item>
      </el-descriptions>
      <p class="mc-profile-note">登录凭据由浏览器以 HttpOnly Cookie 保存，页面不会读取或展示令牌。</p>
      <template #footer>
        <el-button @click="profileVisible = false">关闭</el-button>
      </template>
    </el-dialog>
  </header>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Menu, Refresh, ArrowDown } from '@element-plus/icons-vue'
import { useAuthStore } from '@/stores/auth'
import { getNavTitle } from '@/nav'
import ThemeToggle from '@/components/ThemeToggle.vue'

defineEmits<{ (e: 'menuClick'): void; (e: 'refresh'): void }>()

const route = useRoute()
const router = useRouter()
const auth = useAuthStore()

const user = computed(() => auth.user)
const profileVisible = ref(false)

// 响应式视口宽度：window.innerWidth 本身非响应式，需监听 resize 才能在窗口缩放时更新 isMobile
const windowWidth = ref(window.innerWidth)
function onResize() {
  windowWidth.value = window.innerWidth
}
onMounted(() => window.addEventListener('resize', onResize))
onUnmounted(() => window.removeEventListener('resize', onResize))
const isMobile = computed(() => windowWidth.value < 1024)

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

const sessionExpiry = computed(() => {
  const raw = localStorage.getItem('auth_expiry')
  const ms = raw ? Number(raw) : 0
  if (!Number.isFinite(ms) || ms <= 0) return '未知'
  return new Date(ms).toLocaleString('zh-CN', { hour12: false })
})

// 面包屑标题统一由 navConfig 生成（与 Sidebar 菜单、router 角色守卫同源）
const currentTitle = computed(() => getNavTitle(route.path))

function handleCommand(command: string) {
  if (command === 'profile') {
    profileVisible.value = true
    return
  }
  if (command === 'logout') {
    // 统一走 store：清 httpOnly Cookie + 清空内存中的 user，
    // 旧实现只删 localStorage，user 残留导致登出后 isLoggedIn 仍为 true。
    void auth.logout().finally(() => router.push('/login'))
  }
}
</script>

<style scoped>
.mc-topbar {
  height: 60px;
  background: var(--mc-surface);
  border-bottom: 1px solid var(--mc-line);
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--mc-space-6);
  position: sticky;
  top: 0;
  z-index: var(--mc-z-sticky);
}
.mc-topbar-left { display: flex; align-items: center; gap: var(--mc-space-4); min-width: 0; }
.mc-topbar-right { display: flex; align-items: center; gap: var(--mc-space-2); }
.mc-topbar-icon-btn {
  background: none; border: none; cursor: pointer;
  font-size: 17px; color: var(--mc-text-2);
  width: 34px; height: 34px;
  display: inline-flex; align-items: center; justify-content: center;
  border-radius: var(--mc-radius-sm);
  transition: background var(--mc-t-fast) var(--mc-ease), color var(--mc-t-fast) var(--mc-ease);
}
.mc-topbar-icon-btn:hover { background: var(--mc-surface-3); color: var(--mc-brand-text); }
.mc-topbar-user {
  display: inline-flex; align-items: center; gap: var(--mc-space-2);
  cursor: pointer; padding: var(--mc-space-1) var(--mc-space-2);
  background: transparent; border: 1px solid transparent;
  border-radius: var(--mc-radius-sm);
  transition: background var(--mc-t-fast) var(--mc-ease), border-color var(--mc-t-fast) var(--mc-ease);
}
.mc-topbar-user:hover { background: var(--mc-surface-3); border-color: var(--mc-line); }
.mc-topbar-avatar {
  background: var(--mc-brand-grad);
  color: #fff; font-weight: var(--mc-fw-semibold); font-size: 13px;
  flex-shrink: 0;
}
.mc-topbar-user-info { display: flex; flex-direction: column; line-height: 1.3; text-align: left; }
.mc-topbar-username { font-size: var(--mc-fs-sm); font-weight: var(--mc-fw-medium); color: var(--mc-text-1); }
.mc-topbar-role { font-size: var(--mc-fs-xs); color: var(--mc-text-3); }
.mc-topbar-caret { font-size: 12px; color: var(--mc-text-3); }
.mc-profile-note { margin-top: var(--mc-space-4); font-size: var(--mc-fs-sm); line-height: var(--mc-lh-snug); color: var(--mc-text-3); }

@media (max-width: 640px) {
  .mc-topbar { padding: 0 var(--mc-space-4); }
  .mc-topbar-user-info { display: none; }
}
</style>
