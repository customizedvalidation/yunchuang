import { ref, type Ref } from 'vue'

/**
 * 通用异步数据获取 composable（三态守卫：loading / error / data）
 * 对应 React 版 RTK Query 的 useXxxQuery hook 模式。
 */
export function useFetch<T>(fn: () => Promise<T>, immediate = true) {
  const data = ref<T | null>(null) as Ref<T | null>
  const loading = ref(false)
  const error = ref('')

  async function execute() {
    loading.value = true
    error.value = ''
    try {
      data.value = await fn()
    } catch (e) {
      const msg = e instanceof Error ? e.message : '加载失败'
      error.value = msg
      data.value = null
    } finally {
      loading.value = false
    }
  }

  if (immediate) void execute()

  return { data, loading, error, execute, refetch: execute }
}

/**
 * 通用 mutation composable（对应 React 版 useXxxMutation）
 */
export function useMutation<T, A = void>(fn: (args: A) => Promise<T>) {
  const loading = ref(false)
  const error = ref('')

  async function mutate(args: A): Promise<T> {
    loading.value = true
    error.value = ''
    try {
      return await fn(args)
    } catch (e) {
      const msg = e instanceof Error ? e.message : '操作失败'
      error.value = msg
      throw e
    } finally {
      loading.value = false
    }
  }

  return { loading, error, mutate }
}
