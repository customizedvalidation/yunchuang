/**
 * AccelerationSuiteManagement 页面测试
 *
 * 覆盖：
 * 1. 页面标题渲染
 * 2. 分类统计卡片（数据加速 / 分布式训练 / 推理加速 / 通信优化）渲染
 * 3. 30s 自动轮询提示「自动刷新」渲染
 *
 * 说明：
 * - 本页统计卡片为「分类筛选切换」按钮，且主题「加速」→ /acceleration 即当前页（自链），
 *   按规范第 4 节不包裹 RouterLink，因此无 to 目标 / 「点击查看详情」aria-label 断言。
 * - 统计数字已用 CountUp 渲染；jsdom 下 rAF 动画处于中间态，按规范第 3 节不断言数字文本。
 * - vi.mock('@/api') 覆盖页面从 '@/api' 导入的全部导出（accelerationApi.list / update）。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises, RouterLinkStub } from '@vue/test-utils'
import { createPinia } from 'pinia'

vi.mock('@/api', () => ({
  accelerationApi: { list: vi.fn(), update: vi.fn() },
}))

import AccelerationSuiteManagement from '../AccelerationSuiteManagement.vue'
import { accelerationApi } from '@/api'

function mountPage() {
  // Can 组件按角色收敛操作按钮可见性，测试以 admin 身份挂载
  localStorage.setItem('user', JSON.stringify({ id: 1, username: 'admin', role: 'admin' }))
  return mount(AccelerationSuiteManagement, {
    global: { plugins: [createPinia()], stubs: { RouterLink: RouterLinkStub } },
  })
}

describe('AccelerationSuiteManagement 页面', () => {
  beforeEach(() => {
    vi.mocked(accelerationApi.list).mockResolvedValue([
      { id: 1, name: 'fluid-cache-1', category: 'fluid_cache', enabled: true, type: 'data' },
      { id: 2, name: 'fluid-cache-2', category: 'fluid_cache', enabled: false, type: 'data' },
      { id: 3, name: 'trt-infer-1', category: 'inference', enabled: true, type: 'inference' },
    ] as never)
  })

  it('渲染页面标题「加速套件管理」', async () => {
    const wrapper = mountPage()
    await flushPromises()
    expect(wrapper.text()).toContain('加速套件管理')
  })

  it('渲染分类统计卡片标题', async () => {
    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()
    const text = wrapper.text()
    expect(text).toContain('数据加速')
    expect(text).toContain('分布式训练')
    expect(text).toContain('推理加速')
    expect(text).toContain('通信优化')
  })

  it('启用 30s 自动轮询并提示「自动刷新」', async () => {
    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()
    expect(wrapper.text()).toContain('自动刷新')
  })
})
