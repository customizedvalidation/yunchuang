/**
 * ResourceManagement 页面测试
 *
 * 覆盖：
 * 1. 页面标题渲染
 * 2. GPU / CPU 概览统计卡片渲染
 * 3. 30s 自动轮询提示「自动刷新」渲染
 *
 * 说明：
 * - 本页概览卡片为资源池（resourceApi）按厂商/类型的汇总，主题「资源」→ /resource 即当前页（自链），
 *   按规范第 4 节不包裹 RouterLink，因此无 to 目标 / 「点击查看详情」aria-label 断言。
 * - 统计数字已用 CountUp 渲染；jsdom 下 rAF 动画处于中间态，按规范第 3 节不断言数字文本。
 * - vi.mock('@/api') 覆盖页面从 '@/api' 导入的全部导出（resourceApi.list / update）。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises, RouterLinkStub } from '@vue/test-utils'
import { createPinia } from 'pinia'

vi.mock('@/api', () => ({
  resourceApi: { list: vi.fn(), update: vi.fn() },
}))

import ResourceManagement from '../ResourceManagement.vue'
import { resourceApi } from '@/api'

function mountPage() {
  // Can 组件按角色收敛操作按钮可见性，测试以 admin 身份挂载
  localStorage.setItem('user', JSON.stringify({ id: 1, username: 'admin', role: 'admin' }))
  return mount(ResourceManagement, {
    global: { plugins: [createPinia()], stubs: { RouterLink: RouterLinkStub } },
  })
}

describe('ResourceManagement 页面', () => {
  beforeEach(() => {
    vi.mocked(resourceApi.list).mockResolvedValue([
      { id: 1, name: 'gpu-nv-1', type: 'gpu', vendor: 'nvidia', total: 1, used: 0, available: 1, utilization: 0, status: 'available' },
      { id: 2, name: 'gpu-nv-2', type: 'gpu', vendor: 'nvidia', total: 1, used: 1, available: 0, utilization: 50, status: 'busy' },
      { id: 3, name: 'gpu-en-1', type: 'gpu', vendor: 'enflame', total: 1, used: 0, available: 1, utilization: 0, status: 'available' },
      { id: 4, name: 'cpu-pool-1', type: 'cpu', total: 128, used: 32, available: 96, utilization: 25, status: 'available' },
    ] as never)
  })

  it('渲染页面标题「基础资源管理」', async () => {
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.text()).toContain('基础资源管理')
  })

  it('渲染 GPU / CPU 概览统计卡片标题', async () => {
    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()
    const text = wrapper.text()
    expect(text).toContain('NVIDIA')
    expect(text).toContain('CPU 资源')
  })

  it('启用 30s 自动轮询并提示「自动刷新」', async () => {
    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()
    expect(wrapper.text()).toContain('自动刷新')
  })
})
