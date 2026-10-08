/**
 * GPUManagement 页面测试
 *
 * 覆盖（按 DYNAMIZATION_SPEC 第 7 节最小行为测试）：
 * 1. 页面标题与统计卡片标题渲染
 * 2. 动态化：5 张 GPU 统计卡均指向本页自身（/gpus）→ 不包 RouterLink（自链保持静态 + CountUp）
 * 3. 自动刷新提示渲染
 *
 * 不断言 CountUp 数字（动画中间态）。
 */
import { describe, it, expect, vi } from 'vitest'
import { mount, flushPromises, RouterLinkStub } from '@vue/test-utils'
import { createPinia } from 'pinia'

// mock 覆盖页面从 '@/api' 导入的全部导出（gpuApi / clusterApi）
vi.mock('@/api', () => ({
  gpuApi: {
    devices: vi.fn(),
    allocations: vi.fn(),
    updateDevice: vi.fn(),
    createDevice: vi.fn(),
    deleteDevice: vi.fn(),
    allocate: vi.fn(),
    release: vi.fn(),
  },
  clusterApi: { list: vi.fn() },
}))

import GPUManagement from '../GPUManagement.vue'
import { gpuApi, clusterApi } from '@/api'

function mountPage() {
  return mount(GPUManagement, {
    global: { plugins: [createPinia()], stubs: { RouterLink: RouterLinkStub } },
  })
}

function mockBase() {
  vi.mocked(gpuApi.devices).mockResolvedValue([
    { id: 1, node_name: 'gpu-01', vendor: 'nvidia', model: 'A100', status: 'available' },
    { id: 2, node_name: 'gpu-02', vendor: 'nvidia', model: 'A100', status: 'allocated' },
    { id: 3, node_name: 'gpu-03', vendor: 'enflame', model: 'S60', status: 'maintenance' },
    { id: 4, node_name: 'gpu-04', vendor: 'moore_threads', model: 'MTT', status: 'fault' },
  ] as never)
  vi.mocked(gpuApi.allocations).mockResolvedValue([] as never)
  vi.mocked(clusterApi.list).mockResolvedValue([{ id: 1, name: 'c1' }] as never)
}

describe('GPUManagement 页面', () => {
  it('渲染页面标题"GPU 细粒度管理"与统计卡片标题', async () => {
    mockBase()
    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()

    expect(wrapper.text()).toContain('GPU 细粒度管理')
    expect(wrapper.text()).toContain('总设备数')
    expect(wrapper.text()).toContain('可用')
    expect(wrapper.text()).toContain('已分配')
    expect(wrapper.text()).toContain('维护中')
    expect(wrapper.text()).toContain('故障')
    expect(wrapper.text()).toContain('按厂商分布')
  })

  it('GPU 统计卡均为自链目标（/gpus），不包裹 RouterLink', async () => {
    mockBase()
    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()

    // 本页所有统计卡主题即 GPU → 目标为 /gpus（本页自身），按规范保持静态卡片
    expect(wrapper.findAllComponents(RouterLinkStub).length).toBe(0)
  })

  it('渲染自动刷新提示', async () => {
    mockBase()
    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()

    expect(wrapper.text()).toContain('自动刷新')
  })
})
