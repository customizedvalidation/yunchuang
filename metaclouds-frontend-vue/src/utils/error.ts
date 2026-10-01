import axios from 'axios'

/**
 * 后端错误码 → 用户可读中文文案。
 * 未命中时回退到后端 message，再回退到按 HTTP 状态码的兜底文案。
 */
const CODE_MESSAGE_MAP: Record<string, string> = {
  INVALID_CREDENTIALS: '用户名或密码错误',
  UNAUTHORIZED: '登录状态已失效，请重新登录',
  INVALID_TOKEN: '登录状态已失效，请重新登录',
  FORBIDDEN: '当前账号没有执行该操作的权限',
  PERMISSION_DENIED: '当前账号没有执行该操作的权限',
  NOT_FOUND: '请求的资源不存在',
  CONFLICT: '资源冲突，可能已存在同名记录',
  ALREADY_EXISTS: '该记录已存在',
  VALIDATION_ERROR: '请求参数不合法，请检查后重试',
  BAD_REQUEST: '请求参数不合法，请检查后重试',
  RATE_LIMITED: '操作过于频繁，请稍后再试',
  INTERNAL_SERVER_ERROR: '服务内部错误，请稍后重试',
  SERVICE_UNAVAILABLE: '服务暂时不可用，请稍后重试',
  DATABASE_ERROR: '数据服务异常，请稍后重试',
}

/** 按 HTTP 状态码的兜底文案 */
function messageByStatus(status?: number): string {
  switch (status) {
    case 400:
      return '请求参数不合法，请检查后重试'
    case 401:
      return '登录状态已失效，请重新登录'
    case 403:
      return '当前账号没有执行该操作的权限'
    case 404:
      return '请求的资源不存在'
    case 409:
      return '资源冲突，可能已存在同名记录'
    case 422:
      return '请求参数校验未通过'
    case 429:
      return '操作过于频繁，请稍后再试'
    default:
      return '服务异常，请稍后重试'
  }
}

/**
 * 将任意异常映射为可直接展示给用户的中文文案。
 *
 * - 网络层失败（无 response）→ 离线提示，不暴露底层错误
 * - 4xx → 优先用后端业务码文案，其次 HTTP 状态兜底
 * - 5xx → 统一"服务异常"，不回显内部细节
 */
export function toUserMessage(e: unknown): string {
  if (axios.isAxiosError(e)) {
    const data = e.response?.data as { message?: string; code?: string } | undefined
    if (!e.response) {
      // 超时 / 断网 / CORS 失败
      return e.code === 'ECONNABORTED'
        ? '请求超时，请检查网络后重试'
        : '网络连接异常，请检查网络后重试'
    }
    const mapped = data?.code ? CODE_MESSAGE_MAP[data.code] : undefined
    if (mapped) return mapped
    // 5xx 不回显后端原始 message（可能含内部细节）
    if (e.response.status >= 500) return messageByStatus(e.response.status)
    if (data?.message) return data.message
    return messageByStatus(e.response.status)
  }
  if (e instanceof Error && e.message) return e.message
  return '操作失败，请稍后重试'
}
