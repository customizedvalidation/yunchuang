import { ref, type Ref } from 'vue'
import { toUserMessage } from './error'

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
      // 统一映射为用户可读文案，避免把 axios 英文原文（如
      // "Request failed with status code 500"）直接抛给用户。
      error.value = toUserMessage(e)
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
      error.value = toUserMessage(e)
      throw e
    } finally {
      loading.value = false
    }
  }

  return { loading, error, mutate }
}
