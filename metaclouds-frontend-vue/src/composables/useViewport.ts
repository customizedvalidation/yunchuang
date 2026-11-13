/**
 * 响应式视口判定（供表格列优先级、布局折叠等使用）。
 *
 * 说明：不能直接用 window.innerWidth —— 它不是响应式的，窗口缩放不会触发重渲染。
 * 断点默认 1800px：本项目侧栏 240px + 内容区左右 padding 约 48px，
 * 1800px 窗口对应内容区约 1512px；超过它，12 列宽表（列宽合计 1560px）
 * 才可能完整铺开而不产生横向滚动。
 *
 * 返回值在 SSR / jsdom 下也能安全使用（默认按非紧凑处理）。
 */
import { onBeforeUnmount, onMounted, ref } from 'vue'

export function useViewport(breakpoint = 1800) {
  const isCompact = ref(
    typeof window !== 'undefined' ? window.innerWidth < breakpoint : false,
  )

  function onResize() {
    isCompact.value = window.innerWidth < breakpoint
  }

  onMounted(() => window.addEventListener('resize', onResize))
  onBeforeUnmount(() => window.removeEventListener('resize', onResize))

  return { isCompact }
}

export default useViewport
