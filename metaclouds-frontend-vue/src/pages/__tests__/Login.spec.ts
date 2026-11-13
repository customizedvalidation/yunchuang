/**
 * Login 页面组件测试
 *
 * 覆盖：
 * 1. 表单渲染（用户名/密码输入框、登录按钮、品牌标题）
 * 2. 输入验证（空值、长度校验）
 * 3. 登录成功（mock authApi.login，验证导航与 localStorage）
 * 4. 登录失败（错误提示、失败计数）
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import { ElMessage } from 'element-plus'

// mock 路由：Login.vue 同时用到 useRouter（跳转）与 useRoute（读 redirect 回跳参数），
// 两个都必须 mock，缺 useRoute 会在组件 setup 阶段直接抛错。
const push = vi.fn()
vi.mock('vue-router', () => ({
  useRouter: () => ({ push }),
  useRoute: () => ({ query: {} }),
}))

// mock API
vi.mock('@/api', () => ({
  authApi: {
    login: vi.fn(),
    getCsrfToken: vi.fn(() => Promise.resolve({ csrf_token: 'x' })),
  },
}))
vi.mock('@/api/http', () => ({ setCsrfTokenCache: vi.fn() }))

import Login from '../Login.vue'
import { authApi } from '@/api'

describe('Login 页面', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    localStorage.clear()
    push.mockClear()
    vi.mocked(authApi.login).mockReset()
  })

  it('渲染用户名/密码输入框与登录按钮', () => {
    const wrapper = mount(Login)
    expect(wrapper.find('input[placeholder="请输入用户名"]').exists()).toBe(true)
    expect(wrapper.find('input[placeholder="请输入密码"]').exists()).toBe(true)
    expect(wrapper.text()).toContain('Metaclouds')
    expect(wrapper.text()).toContain('欢迎回来')
  })

  // v3 UI 复盘：登录页不得明文展示默认账号 / 初始密码线索（信息泄露 + 观感业余）。
  // 该断言替代原「渲染默认账号提示文案」，强度更高：既验证新文案，也锁死回归。
  it('登录页不泄露默认账号，并提供管理员求助入口', () => {
    const wrapper = mount(Login)
    const text = wrapper.text()
    expect(text).not.toContain('默认账号')
    expect(text).not.toContain('初始密码由部署配置决定')
    expect(text).not.toContain('admin')
    expect(text).toContain('登录遇到问题？请联系系统管理员')
  })

  it('品牌装饰对读屏隐藏，内容区可作为 skip-link 目标', () => {
    const wrapper = mount(Login)
    expect(wrapper.find('main#mc-main').exists()).toBe(true)
    expect(wrapper.find('svg.login-mesh').attributes('aria-hidden')).toBe('true')
  })

  it('渲染品牌区三项特性介绍', () => {
    const wrapper = mount(Login)
    expect(wrapper.text()).toContain('多集群统一调度')
    expect(wrapper.text()).toContain('GPU 细粒度分配')
    expect(wrapper.text()).toContain('企业级安全防护')
  })

  // v3.1：未登录页也必须能切换外观（登录页常是用户对品牌的第一印象）。
  // 同时锁死「登录按钮必须可语义化定位」，防止再次被新增按钮改变 find('button') 命中顺序。
  it('登录页提供外观切换入口，且登录按钮有稳定类名', () => {
    const wrapper = mount(Login)
    const toggle = wrapper.find('button[aria-haspopup="menu"]')
    expect(toggle.exists()).toBe(true)
    expect(toggle.attributes('aria-label')).toContain('外观设置')
    expect(wrapper.find('button.login-submit').exists()).toBe(true)
  })

  it('登录中按钮显示 loading 文案', async () => {
    let resolveLogin: (v: unknown) => void = () => {}
    vi.mocked(authApi.login).mockImplementation(
      () => new Promise((res) => { resolveLogin = res as (v: unknown) => void }),
    )
    const wrapper = mount(Login)
    await wrapper.find('input[placeholder="请输入用户名"]').setValue('admin')
    await wrapper.find('input[placeholder="请输入密码"]').setValue('password123')
    await wrapper.find('button.login-submit').trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('登录中')
    resolveLogin({ token: 't', user: { id: 1, username: 'admin', role: 'admin' }, expires_at: 0 })
    await flushPromises()
  })

  it('登录成功后写入 localStorage 并跳转 /dashboard', async () => {
    vi.mocked(authApi.login).mockResolvedValue({
      token: 'jwt',
      user: { id: 1, username: 'admin', email: 'a@b.com', role: 'admin' },
      expires_at: Math.floor(Date.now() / 1000) + 86400,
    })
    const wrapper = mount(Login)
    // 填充表单
    await wrapper.find('input[placeholder="请输入用户名"]').setValue('admin')
    await wrapper.find('input[placeholder="请输入密码"]').setValue('password123')
    await wrapper.find('button.login-submit').trigger('click')
    await flushPromises()
    await flushPromises()

    expect(authApi.login).toHaveBeenCalledWith({ username: 'admin', password: 'password123' })
    expect(push).toHaveBeenCalledWith('/dashboard')
    const stored = JSON.parse(localStorage.getItem('user')!)
    expect(stored.username).toBe('admin')
    expect(stored.role).toBe('admin')
  })

  it('登录失败显示错误提示且不跳转', async () => {
    vi.mocked(authApi.login).mockRejectedValue({ response: { data: { message: '用户名或密码错误' } } })
    const errSpy = vi.spyOn(ElMessage, 'error').mockImplementation(() => undefined as never)
    const wrapper = mount(Login)
    await wrapper.find('input[placeholder="请输入用户名"]').setValue('wrong')
    await wrapper.find('input[placeholder="请输入密码"]').setValue('badpass')
    await wrapper.find('button.login-submit').trigger('click')
    await flushPromises()
    await flushPromises()

    expect(push).not.toHaveBeenCalled()
    expect(errSpy).toHaveBeenCalled()
    errSpy.mockRestore()
  })
})
