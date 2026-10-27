<script setup lang="ts">
import { ref, onErrorCaptured } from 'vue'

const hasError = ref(false)

onErrorCaptured(() => {
  hasError.value = true
  return false
})

function reload() {
  window.location.reload()
}
</script>

<template>
  <div v-if="hasError" class="error-boundary">
    <h2>页面渲染出错</h2>
    <button type="button" @click="reload">刷新重试</button>
  </div>
  <slot v-else />
</template>

<style scoped>
.error-boundary {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 48px 16px;
  color: var(--mc-text-1);
}
.error-boundary h2 {
  margin: 0;
  font-size: 18px;
  font-weight: 600;
}
.error-boundary button {
  padding: 8px 20px;
  border: none;
  border-radius: 6px;
  background: var(--mc-brand);
  color: #fff;
  font-size: 14px;
  cursor: pointer;
}
</style>
