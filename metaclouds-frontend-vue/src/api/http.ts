import axios, { type AxiosInstance, type InternalAxiosRequestConfig } from 'axios'
import type { ApiEnvelope } from '@/types'

/** CSRF 双提交令牌缓存（跨域场景下从 GET /auth/csrf 取得） */
let csrfTokenCache: string | null = null

export function setCsrfTokenCache(token: string | null): void {
  csrfTokenCache = token
}

function getCookie(name: string): string | null {
  if (typeof document === 'undefined') return null
  const match = document.cookie.match(new RegExp('(^| )' + name + '=([^;]+)'))
  return match ? decodeURIComponent(match[2]) : null
}

function getCsrfTokenValue(): string | null {
  return getCookie('csrf_token') || csrfTokenCache
}

export function csrfHeaders(): Record<string, string> {
  const t = getCsrfTokenValue()
  return t ? { 'X-CSRF-Token': t } : {}
}

/** 认证类端点的 401 是业务结果（如密码错误），不触发登出跳转 */
const AUTH_ENDPOINT_PATTERN = /^\/auth\//

function isAuthEndpoint(url: string): boolean {
  return AUTH_ENDPOINT_PATTERN.test(url)
}

const http: AxiosInstance = axios.create({
  baseURL: '/api/v1',
  timeout: 30000,
  withCredentials: true,
  headers: {
    'Content-Type': 'application/json',
    Accept: 'application/json',
  },
})

// 请求拦截器：注入 CSRF 头
http.interceptors.request.use(
  (config: InternalAxiosRequestConfig) => {
    const csrf = getCsrfTokenValue()
    if (csrf) {
      config.headers.set('X-CSRF-Token', csrf)
    }
    return config
  },
  (error) => Promise.reject(error),
)

// 响应拦截器：统一解包信封 + 401 会话失效处理
http.interceptors.response.use(
  (response) => {
    const data = response.data
    // 统一解包后端响应信封 { success, data, message, code, timestamp }
    if (data && typeof data === 'object' && 'success' in data) {
      const envelope = data as ApiEnvelope<unknown>
      response.data = envelope.data
    }
    return response
  },
  (error) => {
    const status = error?.response?.status
    const url = error?.config?.url || ''
    // 业务端点 401 = 会话失效：清理本地缓存并跳登录
    if (status === 401 && !isAuthEndpoint(url)) {
      localStorage.removeItem('user')
      localStorage.removeItem('auth_expiry')
      // 调用登出清除后端 httpOnly Cookie（幂等）
      fetch('/api/v1/auth/logout', {
        method: 'POST',
        credentials: 'include',
        headers: { 'Content-Type': 'application/json', ...csrfHeaders() },
      }).catch(() => undefined)
      if (window.location.pathname !== '/login') {
        window.location.href = '/login'
      }
    }
    return Promise.reject(error)
  },
)

export default http
