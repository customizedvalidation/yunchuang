<template>
  <div v-if="loading" class="page-state">
    <el-icon class="page-state-icon is-loading"><Loading /></el-icon>
    <div class="page-state-text">加载中...</div>
  </div>
  <div v-else-if="error" class="page-state">
    <el-icon class="page-state-icon" style="color: var(--mc-danger)"><WarningFilled /></el-icon>
    <div class="page-state-text">{{ error }}</div>
    <el-button v-if="retryText" type="primary" plain size="small" style="margin-top: 12px" @click="$emit('retry')">
      {{ retryText }}
    </el-button>
  </div>
  <div v-else-if="isEmpty" class="page-state">
    <el-icon class="page-state-icon"><Box /></el-icon>
    <div class="page-state-text">{{ emptyText }}</div>
  </div>
  <slot v-else />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Loading, WarningFilled, Box } from '@element-plus/icons-vue'

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
