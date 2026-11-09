/**
 * P0-1 回归测试：后端声明处于模拟执行时，前端必须展示常驻演示模式横幅。
 * 这条断言的意义在于防止「后端老实上报、前端没人展示」的半截修复。
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import ModeBanner from '../ModeBanner.vue'

function stubMode(body: unknown, ok = true, status = 200) {
  vi.stubGlobal(
    'fetch',
    vi.fn(() =>
      Promise.resolve({
        ok,
        status,
        json: () => Promise.resolve(body),
      }),
    ),
  )
}

describe('ModeBanner', () => {
  beforeEach(() => {
    vi.spyOn(console, 'warn').mockImplementation(() => {})
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    vi.restoreAllMocks()
  })

  it('模拟模式下展示横幅与后端说明文案', async () => {
    stubMode({
      success: true,
      data: {
        simulated: true,
        notice: '演示模式：作业执行为模拟；K8s 集群数据来自内置 mock。',
        execution_mode: 'simulated',
        k8s_data_source: 'mock',
        usage_samples_synthetic: true,
      },
    })

    const wrapper = mount(ModeBanner)
    await flushPromises()

    const banner = wrapper.find('[data-testid="mode-banner"]')
    expect(banner.exists()).toBe(true)
    expect(banner.text()).toContain('演示模式')
    expect(banner.text()).toContain('作业执行为模拟')
    expect(fetch).toHaveBeenCalledWith('/api/v1/system/mode', {
      credentials: 'same-origin',
    })
  })

  it('非模拟模式不展示横幅', async () => {
    stubMode({
      success: true,
      data: {
        simulated: false,
        notice: '演示模拟已关闭',
        execution_mode: 'disabled',
        k8s_data_source: 'unavailable',
        usage_samples_synthetic: false,
      },
    })

    const wrapper = mount(ModeBanner)
    await flushPromises()
    expect(wrapper.find('[data-testid="mode-banner"]').exists()).toBe(false)
  })

  it('模式端点不可用时静默降级，不渲染也不抛异常', async () => {
    stubMode({}, false, 503)

    const wrapper = mount(ModeBanner)
    await flushPromises()
    expect(wrapper.find('[data-testid="mode-banner"]').exists()).toBe(false)
    expect(console.warn).toHaveBeenCalled()
  })

  it('网络异常时不阻断页面', async () => {
    vi.stubGlobal('fetch', vi.fn(() => Promise.reject(new Error('network down'))))
    const wrapper = mount(ModeBanner)
    await flushPromises()
    expect(wrapper.find('[data-testid="mode-banner"]').exists()).toBe(false)
  })
})
