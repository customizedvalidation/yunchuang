<template>
  <!--
    品牌定制空态：沿用登录页的「算力网格」语言——点阵背景 + 空芯片托盘 +
    低饱和品牌蓝节点。全部颜色走 --mc-* 令牌，light / dark 自动适配。

    刻意不用 el-empty 的默认插画：那是通用电商风格，与本系统的
    基础设施语义和克制的视觉语言不一致。
  -->
  <div class="mc-empty" role="img" :aria-label="description || '暂无数据'">
    <svg
      class="mc-empty-art"
      :style="{ width: `${artWidth}px`, height: `${artHeight}px` }"
      viewBox="0 0 160 104"
      aria-hidden="true"
      focusable="false"
    >
      <defs>
        <pattern id="mc-empty-dots" width="12" height="12" patternUnits="userSpaceOnUse">
          <circle class="mc-empty-dot" cx="1.5" cy="1.5" r="1" />
        </pattern>
      </defs>
      <rect class="mc-empty-grid" x="6" y="6" width="148" height="92" rx="8" fill="url(#mc-empty-dots)" />
      <!-- 空芯片托盘 -->
      <rect class="mc-empty-tray" x="52" y="30" width="56" height="44" rx="8" />
      <rect class="mc-empty-slot" x="62" y="41" width="36" height="22" rx="4" />
      <!-- 引脚：与网格语言呼应，不画成写实电路 -->
      <path
        class="mc-empty-pin"
        d="M52 42h-8M52 62h-8M108 42h8M108 62h8M68 30v-8M92 30v-8M68 74v8M92 74v8"
        fill="none"
      />
      <!-- 品牌蓝：低饱和单点，避免空态抢视觉权重 -->
      <circle class="mc-empty-node" cx="80" cy="52" r="4" />
      <circle class="mc-empty-node-soft" cx="126" cy="26" r="3" />
    </svg>
    <p v-if="description" class="mc-empty-text">{{ description }}</p>
    <slot />
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'

const props = withDefaults(
  defineProps<{
    description?: string
    /** 与 el-empty 的 image-size 语义一致：插画高度（px） */
    size?: number
  }>(),
  { description: '暂无数据', size: 72 },
)

// 160:104 是 viewBox 比例，按高度等比缩放，保证不同 size 下不变形
const artHeight = computed(() => Math.max(48, Math.min(160, props.size)))
const artWidth = computed(() => Math.round((artHeight.value * 160) / 104))
</script>

<style scoped>
.mc-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--mc-space-3);
  padding: var(--mc-space-6) var(--mc-space-4);
  color: var(--mc-text-3);
  text-align: center;
}
.mc-empty-art { display: block; max-width: 100%; }
.mc-empty-grid { opacity: 0.5; }
.mc-empty-dot { fill: var(--mc-line-strong); opacity: 0.55; }
.mc-empty-tray { fill: var(--mc-surface); stroke: var(--mc-line-strong); stroke-width: 1.5; }
.mc-empty-slot {
  fill: transparent;
  stroke: var(--mc-line);
  stroke-width: 1.5;
  stroke-dasharray: 3 3;
}
.mc-empty-pin { stroke: var(--mc-line-strong); stroke-width: 1.5; stroke-linecap: round; }
.mc-empty-node { fill: var(--mc-brand); opacity: 0.3; }
.mc-empty-node-soft { fill: var(--mc-brand); opacity: 0.55; }
.mc-empty-text {
  margin: 0;
  font-size: var(--mc-fs-sm);
  line-height: var(--mc-lh-snug);
  color: var(--mc-text-3);
  max-width: 34ch;
}

@media (prefers-reduced-motion: reduce) {
  .mc-empty-art { transition: none; }
}
</style>
