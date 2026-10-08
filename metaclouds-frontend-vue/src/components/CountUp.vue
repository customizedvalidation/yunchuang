<template>
  <span class="mc-countup" :aria-label="`${formatted}`">{{ formatted }}</span>
</template>

<script setup lang="ts">
/**
 * CountUp — 数字滚动动画组件
 *
 * 监听 value 变化，从当前显示值 easeOutExpo 缓动滚动到新值。
 * - 首次挂载时从 0 滚动到初始值（形成"数据加载"动效）
 * - 数据自动刷新时从旧值平滑过渡到新值（动态呈现核心）
 * - 测试环境（jsdom 无 rAF 时序依赖）也安全：无 rAF 时直接落到终值
 */
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'

const props = withDefaults(
  defineProps<{
    value: number
    duration?: number
    /** 小数位数；默认整数 */
    decimals?: number
  }>(),
  { duration: 1200, decimals: 0 },
)

const display = ref(0)
const formatted = computed(() =>
  props.decimals > 0
    ? display.value.toFixed(props.decimals)
    : Math.round(display.value).toLocaleString('en-US'),
)

let rafId = 0
let started = false

function easeOutExpo(t: number): number {
  return t === 1 ? 1 : 1 - Math.pow(2, -10 * t)
}

function animate(from: number, to: number, duration: number) {
  cancelAnimationFrame(rafId)
  const start = performance.now()
  const step = (now: number) => {
    const t = Math.min((now - start) / duration, 1)
    display.value = from + (to - from) * easeOutExpo(t)
    if (t < 1) {
      rafId = requestAnimationFrame(step)
    } else {
      display.value = to
    }
  }
  rafId = requestAnimationFrame(step)
}

function sync() {
  // 测试环境无 rAF 时直接落终值，保证断言稳定
  if (typeof requestAnimationFrame === 'undefined') {
    display.value = props.value
    return
  }
  animate(display.value || 0, props.value, props.duration)
}

onMounted(() => {
  started = true
  sync()
})

watch(
  () => props.value,
  () => {
    if (started) sync()
  },
)

onBeforeUnmount(() => cancelAnimationFrame(rafId))
</script>
