<template>
  <div v-if="simulated" class="mc-mode-banner" role="status" data-testid="mode-banner">
    <span class="mc-mode-banner__tag">演示模式</span>
    <span class="mc-mode-banner__text">{{ notice }}</span>
  </div>
</template>

<script setup lang="ts">
/**
 * 常驻执行模式横幅（P0-1）。
 *
 * 后端 `/api/v1/system/mode` 是唯一真源：作业执行体为模拟、实耗为合成值、
 * K8s 数据来自 mock 时，必须让用户一眼看到，避免把演示数据当成真实算力交付。
 * 端点不可用时静默降级（不阻断页面），但会在控制台留下告警便于排查。
 */
import { onMounted, ref } from 'vue'

interface ModeEnvelope {
  success: boolean
  data?: {
    simulated: boolean
    notice: string
    execution_mode: string
    k8s_data_source: string
    usage_samples_synthetic: boolean
  }
}

const simulated = ref(false)
const notice = ref('')

onMounted(async () => {
  try {
    const res = await fetch('/api/v1/system/mode', { credentials: 'same-origin' })
    if (!res.ok) {
      console.warn('[mode] /system/mode unavailable:', res.status)
      return
    }
    const body = (await res.json()) as ModeEnvelope
    if (body.success && body.data?.simulated) {
      simulated.value = true
      notice.value = body.data.notice
    }
  } catch (err) {
    console.warn('[mode] failed to read execution mode:', err)
  }
})
</script>

<style scoped>
.mc-mode-banner {
  position: sticky;
  top: 0;
  z-index: 2000;
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 34px;
  padding: 6px 16px;
  background: #fff7e6;
  border-bottom: 1px solid #ffd591;
  color: #874d00;
  font-size: 13px;
  line-height: 1.5;
}
.mc-mode-banner__tag {
  flex: none;
  padding: 1px 8px;
  border-radius: 4px;
  background: #fa8c16;
  color: #fff;
  font-weight: 600;
  letter-spacing: 0.5px;
}
.mc-mode-banner__text {
  flex: 1;
}
</style>
