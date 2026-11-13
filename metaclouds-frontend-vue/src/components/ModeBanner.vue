<template>
  <div v-if="simulated" class="mc-mode-banner" role="status" aria-live="polite" data-testid="mode-banner">
    <svg class="mc-mode-banner__icon" viewBox="0 0 20 20" fill="none" aria-hidden="true">
      <path d="M10 2.2 1.9 16.3a1 1 0 0 0 .87 1.5h14.46a1 1 0 0 0 .87-1.5L10 2.2Z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
      <path d="M10 7.6v4.2M10 14.6h.01" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
    </svg>
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
 *
 * 视觉：v3 起使用 MDS 语义令牌，橙黄仅作状态提示，不使用高饱和实心色块。
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
  z-index: var(--mc-z-modal);
  display: flex;
  align-items: center;
  gap: var(--mc-space-2);
  min-height: 36px;
  padding: var(--mc-space-1) var(--mc-space-6);
  background: var(--mc-warning-surface);
  border-bottom: 1px solid rgba(138, 90, 0, 0.18);
  color: var(--mc-warning-fg);
  font-size: var(--mc-fs-sm);
  line-height: var(--mc-lh-snug);
}
.mc-mode-banner__icon { width: 16px; height: 16px; flex: none; }
.mc-mode-banner__tag {
  flex: none;
  padding: 2px 8px;
  border-radius: var(--mc-radius-xs);
  background: var(--mc-warning-fg);
  color: #fff;
  font-size: var(--mc-fs-xs);
  font-weight: var(--mc-fw-semibold);
  letter-spacing: 0.02em;
}
.mc-mode-banner__text { flex: 1; }
@media (max-width: 640px) {
  .mc-mode-banner { padding-inline: var(--mc-space-4); }
}
</style>
