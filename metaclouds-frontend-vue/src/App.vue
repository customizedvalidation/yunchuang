<template>
  <router-view />
</template>

<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue'
import { useAuthStore } from '@/stores/auth'

const auth = useAuthStore()
let refreshTimer: ReturnType<typeof setInterval> | null = null

onMounted(() => {
  // 已登录会话启动静默续期检查（5min 间隔）
  if (auth.isLoggedIn) {
    void auth.silentRefresh()
    refreshTimer = setInterval(() => void auth.silentRefresh(), 5 * 60 * 1000)
  }
})

onUnmounted(() => {
  if (refreshTimer) clearInterval(refreshTimer)
})
</script>
