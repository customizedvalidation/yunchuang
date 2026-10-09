import { onBeforeUnmount, onMounted, ref } from 'vue'

export interface UseAutoRefreshOptions {
  /** 轮询周期（秒），默认 30 */
  intervalSeconds?: number
}

/**
 * 自动刷新 composable：统一管理倒计时、setInterval 轮询、页面回到前台的
 * visibilitychange 补刷，以及组件卸载时的定时器/监听清理。
 *
 * 行为与原各页面内联实现完全一致：挂载即开始轮询，卸载清理；
 * `refresh()` 重置倒计时并触发 refreshFn（手动刷新按钮与自动周期共用同一入口）。
 *
 * @param refreshFn 实际拉取数据的函数（如 loadJobs / loadQuotas）
 */
export function useAutoRefresh(refreshFn: () => void, options: UseAutoRefreshOptions = {}) {
  const intervalSeconds = options.intervalSeconds ?? 30
  const autoRefreshCountdown = ref(intervalSeconds)
  let countdownTimer: number | null = null

  function refresh() {
    autoRefreshCountdown.value = intervalSeconds
    refreshFn()
  }

  function stopAutoRefresh() {
    if (countdownTimer !== null) {
      window.clearInterval(countdownTimer)
      countdownTimer = null
    }
  }

  function startAutoRefresh() {
    stopAutoRefresh()
    autoRefreshCountdown.value = intervalSeconds
    countdownTimer = window.setInterval(() => {
      autoRefreshCountdown.value -= 1
      if (autoRefreshCountdown.value <= 0) {
        autoRefreshCountdown.value = intervalSeconds
        // 页面不可见时暂停拉取，回到前台后立即补一次刷新
        if (document.visibilityState === 'visible') refresh()
      }
    }, 1000)
  }

  function onVisibilityChange() {
    if (document.visibilityState === 'visible') {
      // 回到前台：立即刷新一次并重启周期
      refresh()
      startAutoRefresh()
    } else {
      stopAutoRefresh()
    }
  }

  onMounted(() => {
    document.addEventListener('visibilitychange', onVisibilityChange)
    startAutoRefresh()
  })

  onBeforeUnmount(() => {
    stopAutoRefresh()
    document.removeEventListener('visibilitychange', onVisibilityChange)
  })

  return { autoRefreshCountdown, refresh, startAutoRefresh, stopAutoRefresh }
}
