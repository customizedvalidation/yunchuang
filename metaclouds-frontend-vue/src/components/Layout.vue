<template>
  <div class="mc-app-shell" :class="{ 'is-mobile-open': mobileOpen }">
    <Sidebar :collapsed="collapsed" @collapse="collapsed = $event" />
    <div class="mc-app-main">
      <Topbar @menu-click="mobileOpen = true" @refresh="refreshPage" />
      <main class="mc-app-content">
        <router-view v-slot="{ Component }">
          <transition name="fade" mode="out-in">
            <!-- 顶层模块 key：同级子路由（/job/list↔/job/queue↔/job/history、
                 /k8s/nodes↔/k8s/pods↔/k8s/services）共享 key → 切换不重挂载，
                 保留分页/搜索/轮询状态；跨顶层模块 key 不同 → 仍重挂载。 -->
            <component :is="Component" :key="layoutKey" />
          </transition>
        </router-view>
      </main>
    </div>
    <div class="mc-app-overlay" @click="mobileOpen = false" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'
import Sidebar from './Sidebar.vue'
import Topbar from './Topbar.vue'

const route = useRoute()
const collapsed = ref(false)
const mobileOpen = ref(false)

// 顶层模块 key：取路由路径首段（如 'job' / 'k8s' / 'dashboard'），
// 保证同级子路由切换时组件不重挂载。
const layoutKey = computed(() => {
  const seg = route.path.split('/').filter(Boolean)[0]
  return seg || 'root'
})

// Esc 关闭移动端浮层
function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape') mobileOpen.value = false
}
onMounted(() => window.addEventListener('keydown', onKey))
onUnmounted(() => window.removeEventListener('keydown', onKey))

// 刷新当前页面（重新挂载）
function refreshPage() {
  // 通过 key 变化触发组件重新挂载
  window.location.reload()
}
</script>

<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.15s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }
</style>
