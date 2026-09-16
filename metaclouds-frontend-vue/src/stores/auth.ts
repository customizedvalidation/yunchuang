import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import { authApi } from '@/api'
import { setCsrfTokenCache } from '@/api/http'
import type { User, UserRole } from '@/types'
import { readStoredUser, readStoredRole } from '@/utils/auth'

export const useAuthStore = defineStore('auth', () => {
  const user = ref<User | null>(readStoredUser())
  const role = computed<UserRole | null>(() => user.value?.role ?? readStoredRole())

  const isLoggedIn = computed(() => !!user.value)

  async function login(username: string, password: string) {
    const result = await authApi.login({ username, password })
    const { user: u, expires_at } = result
    user.value = {
      id: u.id,
      username: u.username,
      email: u.email,
      role: u.role,
      tenant_id: u.tenant_id,
    }
    localStorage.setItem('user', JSON.stringify(user.value))
    localStorage.setItem('auth_expiry', String((expires_at ?? 0) * 1000))
    // 登录后拉取 CSRF 令牌
    try {
      const csrf = await authApi.getCsrfToken()
      setCsrfTokenCache(csrf.csrf_token)
    } catch {
      // 同源场景下 Cookie 可读，无需缓存
    }
    return result
  }

  async function logout() {
    try {
      await authApi.logout()
    } catch {
      // 登出幂等，忽略错误
    }
    user.value = null
    localStorage.removeItem('user')
    localStorage.removeItem('auth_expiry')
  }

  /** 静默续期：令牌临近过期时调用 refresh 换新令牌 */
  async function silentRefresh() {
    const expiresAtStr = localStorage.getItem('auth_expiry')
    if (!expiresAtStr) return
    const expiresAtMs = Number(expiresAtStr)
    const remainingMs = expiresAtMs - Date.now()
    if (remainingMs <= 0 || remainingMs > 60 * 60 * 1000) return
    try {
      const result = await authApi.refreshToken()
      const { user: u, expires_at } = result
      user.value = {
        id: u.id,
        username: u.username,
        email: u.email,
        role: u.role,
        tenant_id: u.tenant_id,
      }
      localStorage.setItem('user', JSON.stringify(user.value))
      localStorage.setItem('auth_expiry', String((expires_at ?? 0) * 1000))
    } catch {
      // 续期失败静默忽略
    }
  }

  return { user, role, isLoggedIn, login, logout, silentRefresh }
})
