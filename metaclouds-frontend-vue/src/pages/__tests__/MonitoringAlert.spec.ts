/**
 * MonitoringAlert 页面测试
 *
 * 覆盖（按 DYNAMIZATION_SPEC 第 7 节最小行为测试）：
 * 1. 页面标题与 KPI 卡片标题渲染
 * 2. 动态化：集群总数/在线节点 → /cluster，GPU 利用率 → /gpus（RouterLink 包裹 + aria-label）
 * 3. 活跃告警主题即本页（/monitoring）→ 自链，不包 RouterLink
 * 4. 自动刷新提示渲染
 *
 * echarts 已在 setup.ts 中 mock；不断言 CountUp 数字（动画中间态）。
 */
import { describe, it, expect, vi } from 'vitest'
import { mount, flushPromises, RouterLinkStub } from '@vue/test-utils'
import { createPinia } from 'pinia'

// mock 覆盖页面从 '@/api' 导入的全部导出（monitoringApi）
vi.mock('@/api', () => ({
  monitoringApi: {
    metrics: vi.fn(),
    alerts: vi.fn(),
    resolveAlert: vi.fn(),
    acknowledgeAlert: vi.fn(),
  },
}))

import MonitoringAlert from '../MonitoringAlert.vue'
import { monitoringApi } from '@/api'

function mountPage() {
  return mount(MonitoringAlert, {
    global: { plugins: [createPinia()], stubs: { RouterLink: RouterLinkStub } },
  })
}

function mockBase() {
  vi.mocked(monitoringApi.metrics).mockResolvedValue({
    cluster_total: 3,
    online_nodes: 5,
    gpu_utilization: 62,
  } as never)
  vi.mocked(monitoringApi.alerts).mockResolvedValue([
    { id: 1, level: 'critical', status: 'active', type: 'system', message: 'CPU 温度过高' },
    { id: 2, level: 'warning', status: 'resolved', type: 'resource', message: '内存使用率偏高' },
  ] as never)
}

describe('MonitoringAlert 页面', () => {
  it('渲染页面标题"监控告警中心"与 KPI 卡片标题', async () => {
    mockBase()
    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()

    expect(wrapper.text()).toContain('监控告警中心')
    expect(wrapper.text()).toContain('集群总数')
    expect(wrapper.text()).toContain('在线节点')
    expect(wrapper.text()).toContain('GPU 利用率')
    expect(wrapper.text()).toContain('活跃告警')
  })

  it('可跳转 KPI 卡片渲染为 RouterLink 且目标正确', async () => {
    mockBase()
    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()

    const cardLinks = wrapper
      .findAllComponents(RouterLinkStub)
      .filter((l) => l.classes().includes('kpi-card'))
    // 集群总数、在线节点、GPU 利用率 = 3 张可点击卡；活跃告警为自链静态卡
    expect(cardLinks.length).toBe(3)
    const tos = cardLinks.map((l) => (l.props('to') as string) || '')
    expect(tos).toContain('/cluster')
    expect(tos).toContain('/gpus')
    // 可点击卡片带 aria-label（含「点击查看详情」）
    expect(cardLinks[0].attributes('aria-label')).toContain('点击查看详情')
  })

  it('渲染自动刷新提示', async () => {
    mockBase()
    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()

    expect(wrapper.text()).toContain('自动刷新')
  })
})
