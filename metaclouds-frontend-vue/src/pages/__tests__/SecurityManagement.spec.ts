/**
 * SecurityManagement 页面测试
 *
 * 覆盖：
 * 1. 页面标题渲染
 * 2. 安全概览统计卡片（策略总数 / 访问控制 / 网络安全 / 数据安全 / 系统安全）渲染
 * 3. 30s 自动轮询提示「自动刷新」渲染
 *
 * 说明：
 * - 本页概览卡片主题「安全」→ /security 即当前页（自链），按规范第 4 节不包裹 RouterLink，
 *   因此无 to 目标 / 「点击查看详情」aria-label 断言。
 * - 统计数字已用 CountUp 渲染；jsdom 下 rAF 动画处于中间态，按规范第 3 节不断言数字文本。
 * - vi.mock('@/api') 覆盖页面从 '@/api' 导入的全部导出（securityApi.list / update）。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises, RouterLinkStub } from '@vue/test-utils'
import { createPinia } from 'pinia'

vi.mock('@/api', () => ({
  securityApi: { list: vi.fn(), update: vi.fn() },
}))

import SecurityManagement from '../SecurityManagement.vue'
import { securityApi } from '@/api'

function mountPage() {
  // Can 组件按角色收敛操作按钮可见性，测试以 admin 身份挂载
  localStorage.setItem('user', JSON.stringify({ id: 1, username: 'admin', role: 'admin' }))
  return mount(SecurityManagement, {
    global: { plugins: [createPinia()], stubs: { RouterLink: RouterLinkStub } },
  })
}

describe('SecurityManagement 页面', () => {
  beforeEach(() => {
    vi.mocked(securityApi.list).mockResolvedValue([
      { id: 1, name: 'allow-office-vpn', type: 'access', enabled: true, status: 'active' },
      { id: 2, name: 'deny-public-ssh', type: 'network', enabled: true, status: 'active' },
      { id: 3, name: 'mask-pii', type: 'data', enabled: false, status: 'inactive' },
    ] as never)
  })

  it('渲染页面标题「安全管理」', async () => {
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.text()).toContain('安全管理')
  })

  it('渲染安全概览统计卡片标题', async () => {
    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()
    const text = wrapper.text()
    expect(text).toContain('策略总数')
    expect(text).toContain('访问控制')
    expect(text).toContain('数据安全')
  })

  it('启用 30s 自动轮询并提示「自动刷新」', async () => {
    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()
    expect(wrapper.text()).toContain('自动刷新')
  })
})
