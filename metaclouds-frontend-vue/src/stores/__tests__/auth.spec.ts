/**
 * auth store 测试
 *
 * 覆盖：
 * 1. 初始状态：无 token 时 isLoggedIn=false，role=null
 * 2. login：成功后写入 user / auth_expiry，isLoggedIn=true，role 正确
 * 3. logout：清理 user 与 auth_expiry，isLoggedIn=false
 * 4. 角色读取：从 localStorage 恢复
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setActivePinia, createPinia } from 'pinia'

// mock API 层：authApi
vi.mock('@/api', () => ({
  authApi: {
    login: vi.fn(),
    logout: vi.fn(),
    getCsrfToken: vi.fn(() => Promise.resolve({ csrf_token: 'csrf-xyz' })),
    refreshToken: vi.fn(),
  },
}))
// mock CSRF 缓存写入（http 模块副作用）
vi.mock('@/api/http', () => ({
  setCsrfTokenCache: vi.fn(),
}))

import { useAuthStore } from '../auth'
import { authApi } from '@/api'

describe('useAuthStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    vi.mocked(authApi.login).mockReset()
    vi.mocked(authApi.logout).mockReset()
  })

  it('初始未登录：isLoggedIn=false，role=null', () => {
    const store = useAuthStore()
    expect(store.isLoggedIn).toBe(false)
    expect(store.role).toBeNull()
    expect(store.user).toBeNull()
  })

  it('login 成功：写入 localStorage 并更新状态', async () => {
    vi.mocked(authApi.login).mockResolvedValue({
      token: 'jwt',
      user: { id: 1, username: 'admin', email: 'a@b.com', role: 'admin', tenant_id: 1 },
      expires_at: Math.floor(Date.now() / 1000) + 86400,
    })
    const store = useAuthStore()
    await store.login('admin', 'pass')

    expect(store.isLoggedIn).toBe(true)
    expect(store.user?.username).toBe('admin')
    expect(store.role).toBe('admin')
    // localStorage 已持久化
    const raw = localStorage.getItem('user')
    expect(raw).not.toBeNull()
    expect(JSON.parse(raw!).role).toBe('admin')
    expect(localStorage.getItem('auth_expiry')).not.toBeNull()
  })

  it('login 失败时抛出错误且不改变状态', async () => {
    vi.mocked(authApi.login).mockRejectedValue(new Error('401'))
    const store = useAuthStore()
    await expect(store.login('admin', 'wrong')).rejects.toThrow('401')
    expect(store.isLoggedIn).toBe(false)
  })

  it('logout：清理本地状态并调用后端登出', async () => {
    // 先登录
    vi.mocked(authApi.login).mockResolvedValue({
      token: 'jwt',
      user: { id: 1, username: 'admin', email: 'a@b.com', role: 'admin' },
      expires_at: Math.floor(Date.now() / 1000) + 86400,
    })
    vi.mocked(authApi.logout).mockResolvedValue(undefined)
    const store = useAuthStore()
    await store.login('admin', 'pass')
    expect(store.isLoggedIn).toBe(true)

    await store.logout()
    expect(authApi.logout).toHaveBeenCalled()
    expect(store.isLoggedIn).toBe(false)
    expect(store.user).toBeNull()
    expect(localStorage.getItem('user')).toBeNull()
  })

  it('logout 即使后端报错也清理本地状态', async () => {
    vi.mocked(authApi.logout).mockRejectedValue(new Error('network'))
    const store = useAuthStore()
    // 模拟已登录
    localStorage.setItem('user', JSON.stringify({ id: 1, username: 'u', role: 'user' }))
    await store.logout()
    expect(store.user).toBeNull()
    expect(localStorage.getItem('user')).toBeNull()
  })
})
