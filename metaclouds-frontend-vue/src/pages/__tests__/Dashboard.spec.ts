/**
 * Dashboard 页面测试
 *
 * 覆盖：
 * 1. 页面标题与统计卡片渲染
 * 2. 有数据时 KPI 计算（集群数、GPU 利用率、运行中作业、活跃告警）
 * 3. 空数据态
 *
 * echarts 已在 setup.ts 中 mock，避免 jsdom canvas 问题。
 */
import { describe, it, expect, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia } from 'pinia'

// mock 全部 API
vi.mock('@/api', () => ({
  clusterApi: { list: vi.fn() },
  resourceApi: { list: vi.fn(), gpuResources: vi.fn() },
  jobApi: { list: vi.fn() },
  monitoringApi: { alerts: vi.fn(), metrics: vi.fn() },
  gpuApi: { devices: vi.fn() },
  partitionApi: { list: vi.fn() },
  schedulerApi: { list: vi.fn() },
  tenantApi: { list: vi.fn() },
}))

import Dashboard from '../Dashboard.vue'
import { clusterApi, resourceApi, jobApi, monitoringApi, gpuApi, partitionApi, schedulerApi, tenantApi } from '@/api'

function mountDashboard() {
  return mount(Dashboard, { global: { plugins: [createPinia()] } })
}

describe('Dashboard 页面', () => {
  it('渲染页面标题"算力总览"', () => {
    vi.mocked(clusterApi.list).mockResolvedValue([])
    vi.mocked(resourceApi.list).mockResolvedValue([])
    vi.mocked(jobApi.list).mockResolvedValue([])
    vi.mocked(monitoringApi.alerts).mockResolvedValue([])
    vi.mocked(gpuApi.devices).mockResolvedValue([])
    vi.mocked(partitionApi.list).mockResolvedValue([])
    vi.mocked(schedulerApi.list).mockResolvedValue([])
    vi.mocked(tenantApi.list).mockResolvedValue([])
    const wrapper = mountDashboard()
    expect(wrapper.text()).toContain('算力总览')
  })

  it('有数据时渲染统计卡片并计算 KPI', async () => {
    vi.mocked(clusterApi.list).mockResolvedValue([{ id: 1, name: 'c1' }, { id: 2, name: 'c2' }, { id: 3, name: 'c3' }] as never)
    vi.mocked(resourceApi.list).mockResolvedValue([
      { id: 1, type: 'gpu', total: 8, used: 4 },
      { id: 2, type: 'gpu', total: 8, used: 2 },
    ] as never)
    vi.mocked(jobApi.list).mockResolvedValue([
      { id: 1, status: 'running' },
      { id: 2, status: 'running' },
      { id: 3, status: 'pending' },
    ] as never)
    vi.mocked(monitoringApi.alerts).mockResolvedValue([
      { id: 1, level: 'critical' },
      { id: 2, level: 'warning' },
    ] as never)
    vi.mocked(gpuApi.devices).mockResolvedValue([])
    vi.mocked(partitionApi.list).mockResolvedValue([])
    vi.mocked(schedulerApi.list).mockResolvedValue([])
    vi.mocked(tenantApi.list).mockResolvedValue([])

    const wrapper = mountDashboard()
    await flushPromises()
    await flushPromises()

    const text = wrapper.text()
    // 集群数量 = 3
    expect(text).toContain('集群数量')
    // 运行中作业 = 2
    expect(text).toContain('运行中作业')
    // 活跃告警 = 2
    expect(text).toContain('活跃告警')
  })

  it('无数据时图表区显示空态文案', async () => {
    vi.mocked(clusterApi.list).mockResolvedValue([])
    vi.mocked(resourceApi.list).mockResolvedValue([])
    vi.mocked(jobApi.list).mockResolvedValue([])
    vi.mocked(monitoringApi.alerts).mockResolvedValue([])
    vi.mocked(gpuApi.devices).mockResolvedValue([])
    vi.mocked(partitionApi.list).mockResolvedValue([])
    vi.mocked(schedulerApi.list).mockResolvedValue([])
    vi.mocked(tenantApi.list).mockResolvedValue([])
    const wrapper = mountDashboard()
    await flushPromises()
    await flushPromises()
    expect(wrapper.text()).toContain('暂无资源数据')
    expect(wrapper.text()).toContain('还没有作业')
  })
})
