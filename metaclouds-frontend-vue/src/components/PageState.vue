<template>
  <!-- 加载态：骨架屏保持布局尺寸稳定，避免内容跳变（CLS），比转圈图标更克制 -->
  <div v-if="loading" class="page-state-skeleton" role="status" aria-busy="true">
    <span class="mc-sr-only">加载中...</span>
    <div class="mc-skeleton mc-skeleton--title" />
    <div class="mc-skeleton-grid mc-mt">
      <div v-for="n in 3" :key="n" class="mc-skeleton mc-skeleton--card" />
    </div>
    <div class="mc-mt">
      <div v-for="n in 4" :key="n" class="mc-skeleton mc-skeleton--row" />
    </div>
  </div>
  <div v-else-if="error" class="page-state" role="alert">
    <el-icon class="page-state-icon" style="color: var(--mc-danger)"><WarningFilled /></el-icon>
    <div class="page-state-text">{{ error }}</div>
    <div class="page-state-hint">数据暂时无法加载，请稍后重试或联系系统管理员。</div>
    <el-button v-if="retryText" type="primary" plain size="small" @click="$emit('retry')">
      {{ retryText }}
    </el-button>
  </div>
  <div v-else-if="isEmpty" class="page-state">
    <McEmpty :description="emptyText" :size="72" />
  </div>
  <slot v-else />
</template>

<script setup lang="ts">
import McEmpty from '@/components/McEmpty.vue'
import { computed } from 'vue'
import { WarningFilled } from '@element-plus/icons-vue'

const props = withDefaults(defineProps<{
  loading?: boolean
  error?: string
  data?: unknown
  emptyText?: string
  retryText?: string
}>(), {
  loading: false,
  error: '',
  emptyText: '暂无数据',
  retryText: '重试',
})

defineEmits<{ (e: 'retry'): void }>()

const isEmpty = computed(() => {
  if (props.data === null || props.data === undefined) return true
  if (Array.isArray(props.data)) return props.data.length === 0
  if (typeof props.data === 'object') return Object.keys(props.data as object).length === 0
  return false
})
</script>

<style scoped>
.page-state-skeleton { padding: var(--mc-space-2) 0; }
</style>
