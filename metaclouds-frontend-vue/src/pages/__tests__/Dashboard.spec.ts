/**
 * Dashboard 页面测试
 *
 * 覆盖：
 * 1. 页面标题与统计卡片渲染
 * 2. 有数据时 KPI 计算（集群数、GPU 利用率、运行中作业、活跃告警）
 * 3. 空数据态
 * 4. 算力总览动态化：KPI 卡片渲染为可点击链接（跳转目标正确）+ 图表头"查看详情"链接
 *
 * echarts 已在 setup.ts 中 mock，避免 jsdom canvas 问题。
 * vue-router 的 RouterLink 在无真实 router 的 mount 下 stub 为 <a>，
 * 便于断言 href（跳转目标）。
 */
import { describe, it, expect, vi } from 'vitest'
import { mount, flushPromises, RouterLinkStub } from '@vue/test-utils'
import { createPinia } from 'pinia'

// mock 全部 API
vi.mock('@/api', () => ({
  clusterApi: { list: vi.fn() },
  resourceApi: { list: vi.fn(), gpuResources: vi.fn() },
  jobApi: { list: vi.fn() },
  monitoringApi: { alerts: vi.fn(), metrics: vi.fn(), dashboard: vi.fn() },
  gpuApi: { devices: vi.fn() },
  partitionApi: { list: vi.fn() },
  schedulerApi: { list: vi.fn() },
  tenantApi: { list: vi.fn() },
}))

import Dashboard from '../Dashboard.vue'
import { clusterApi, resourceApi, jobApi, monitoringApi, gpuApi, partitionApi, schedulerApi, tenantApi } from '@/api'

function mountDashboard() {
  // RouterLink 为 vue-router 全局注册组件，用 @vue/test-utils 内置 RouterLinkStub
  // stub 渲染为 <a>，可通过 props('to') 断言跳转目标
  return mount(Dashboard, {
    global: { plugins: [createPinia()], stubs: { RouterLink: RouterLinkStub } },
  })
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
    vi.mocked(monitoringApi.dashboard).mockResolvedValue({})
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
    vi.mocked(monitoringApi.dashboard).mockResolvedValue({})

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
    vi.mocked(monitoringApi.dashboard).mockResolvedValue({})
    const wrapper = mountDashboard()
    await flushPromises()
    await flushPromises()
    expect(wrapper.text()).toContain('暂无资源数据')
    expect(wrapper.text()).toContain('还没有作业')
  })

  it('算力总览动态化：KPI 卡片渲染为可点击链接且跳转目标正确', async () => {
    vi.mocked(clusterApi.list).mockResolvedValue([{ id: 1, name: 'c1' }] as never)
    vi.mocked(resourceApi.list).mockResolvedValue([])
    vi.mocked(jobApi.list).mockResolvedValue([{ id: 1, status: 'running' }] as never)
    vi.mocked(monitoringApi.alerts).mockResolvedValue([])
    vi.mocked(gpuApi.devices).mockResolvedValue([])
    vi.mocked(partitionApi.list).mockResolvedValue([])
    vi.mocked(schedulerApi.list).mockResolvedValue([])
    vi.mocked(tenantApi.list).mockResolvedValue([])
    vi.mocked(monitoringApi.dashboard).mockResolvedValue({})
    const wrapper = mountDashboard()
    await flushPromises()
    await flushPromises()

    const links = wrapper.findAllComponents(RouterLinkStub)
    // 6 张无角色限制的 KPI 卡 + 3 个图表头"查看详情" + 1 个告警"查看全部" = 10。
    // 第 7 张租户卡在无 role（未登录/未知角色）时为静态 div，不给可点入口。
    expect(links.length).toBe(10)
    const cardLinks = links.filter((l) => l.classes().includes('kpi-card'))
    expect(cardLinks.length).toBe(6)
    const tos = cardLinks.map((l) => (l.props('to') as string) || '')
    expect(tos).toContain('/cluster')
    expect(tos).toContain('/gpus')
    expect(tos).toContain('/job')
    expect(tos).toContain('/monitoring')
    expect(tos).toContain('/partitions')
    expect(tos).toContain('/schedulers')
    // 无权限角色不得出现 /tenant 可点入口（否则点击被守卫弹回，属伪装链接）
    expect(tos).not.toContain('/tenant')
    const staticTenantCard = wrapper.find('.kpi-card--static')
    expect(staticTenantCard.exists()).toBe(true)
    expect(staticTenantCard.text()).toContain('租户数量')
    // KPI 卡片带 aria-label（键盘/读屏可访问）
    expect(cardLinks[0].attributes('aria-label')).toContain('点击查看详情')
  })

  it('admin 角色可见可点的租户 KPI 卡，且不渲染伪装的今日/近 7 天时间切换', async () => {
    vi.mocked(clusterApi.list).mockResolvedValue([])
    vi.mocked(resourceApi.list).mockResolvedValue([])
    vi.mocked(jobApi.list).mockResolvedValue([])
    vi.mocked(monitoringApi.alerts).mockResolvedValue([])
    vi.mocked(gpuApi.devices).mockResolvedValue([])
    vi.mocked(partitionApi.list).mockResolvedValue([])
    vi.mocked(schedulerApi.list).mockResolvedValue([])
    vi.mocked(tenantApi.list).mockResolvedValue([])
    vi.mocked(monitoringApi.dashboard).mockResolvedValue({})

    // 在挂载前写入 localStorage，让 auth store 初始化即读到 admin 角色
    // （与真实登录态一致的来源，避免直接改 store 内部 ref）
    localStorage.setItem('user', JSON.stringify({
      id: 1, username: 'admin', role: 'admin', email: 'a@b.c', tenant_id: 1,
    }))
    const wrapper = mountDashboard()
    await flushPromises()

    const cardLinks = wrapper.findAllComponents(RouterLinkStub).filter((l) => l.classes().includes('kpi-card'))
    const tos = cardLinks.map((l) => (l.props('to') as string) || '')
    expect(tos).toContain('/tenant')
    expect(wrapper.find('.kpi-card--static').exists()).toBe(false)

    // 后端 GET /monitoring/dashboard 无时间参数，原三态切换只改文案 → 必须已移除
    expect(wrapper.text()).not.toContain('近 7 天')
    expect(wrapper.text()).not.toContain('近 30 天')
    expect(wrapper.text()).toContain('实时快照')
  })

  it('图表与告警卡片头部提供"查看详情/查看全部"链接', async () => {
    vi.mocked(clusterApi.list).mockResolvedValue([{ id: 1, name: 'c1' }] as never)
    vi.mocked(resourceApi.list).mockResolvedValue([])
    vi.mocked(jobApi.list).mockResolvedValue([{ id: 1, status: 'running' }] as never)
    vi.mocked(monitoringApi.alerts).mockResolvedValue([])
    vi.mocked(gpuApi.devices).mockResolvedValue([])
    vi.mocked(partitionApi.list).mockResolvedValue([])
    vi.mocked(schedulerApi.list).mockResolvedValue([])
    vi.mocked(tenantApi.list).mockResolvedValue([])
    vi.mocked(monitoringApi.dashboard).mockResolvedValue({})
    const wrapper = mountDashboard()
    await flushPromises()
    await flushPromises()

    const moreLinks = wrapper
      .findAllComponents(RouterLinkStub)
      .filter((l) => l.classes().includes('chart-card-more'))
    const moreTos = moreLinks.map((l) => (l.props('to') as string) || '')
    expect(moreTos).toContain('/gpus')
    expect(moreTos).toContain('/job')
    expect(moreTos).toContain('/monitoring')
    // 自动刷新倒计时提示渲染
    expect(wrapper.text()).toContain('自动刷新')
  })
})
