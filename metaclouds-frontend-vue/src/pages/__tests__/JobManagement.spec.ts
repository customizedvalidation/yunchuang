/**
 * JobManagement 页面测试
 *
 * 覆盖：
 * 1. 页面标题与统计信息渲染
 * 2. 作业表格渲染与数据行数
 * 3. Tab 筛选（任务队列只显示 pending，历史记录显示 completed/failed/cancelled）
 * 4. 新建作业对话框打开
 * 5. 空数据态
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia } from 'pinia'

// 用模块级可变变量控制当前路由路径（vi.mock 工厂必须是 hoistable）
let currentRoutePath = '/job/list'
const push = vi.fn()
vi.mock('vue-router', () => ({
  useRoute: () => ({ path: currentRoutePath }),
  useRouter: () => ({ push }),
}))

const jobsData = [
  { id: 1, name: 'llama3-finetune', type: 'training', status: 'running', gpus: 4, progress: 65 },
  { id: 2, name: 'gpt4-inference', type: 'inference', status: 'pending', gpus: 2, progress: 0 },
  { id: 3, name: 'batch-job', type: 'batch', status: 'completed', gpus: 1, progress: 100 },
  { id: 4, name: 'failed-job', type: 'training', status: 'failed', gpus: 2, progress: 30, error_msg: 'OOM' },
  { id: 5, name: 'cancelled-job', type: 'batch', status: 'cancelled', gpus: 1, progress: 10 },
]

vi.mock('@/api', () => ({
  jobApi: { list: vi.fn(), create: vi.fn(), cancel: vi.fn(), submitToK8S: vi.fn() },
  partitionApi: { list: vi.fn(() => Promise.resolve([])) },
  checkpointApi: { list: vi.fn(() => Promise.resolve([])) },
}))

import JobManagement from '../JobManagement.vue'
import { jobApi } from '@/api'

function mountJob(path = '/job/list') {
  currentRoutePath = path
  // Can 组件按角色收敛按钮可见性，测试以 admin 身份挂载以显示操作按钮
  localStorage.setItem('user', JSON.stringify({ id: 1, username: 'admin', role: 'admin' }))
  return mount(JobManagement, { global: { plugins: [createPinia()] } })
}

describe('JobManagement 页面', () => {
  beforeEach(() => {
    vi.mocked(jobApi.list).mockResolvedValue(jobsData as never)
    push.mockClear()
  })

  it('渲染页面标题"作业管理"', async () => {
    const wrapper = mountJob()
    await flushPromises()
    expect(wrapper.text()).toContain('作业管理')
  })

  it('作业列表 Tab 显示全部 5 个作业', async () => {
    const wrapper = mountJob('/job/list')
    await flushPromises()
    await flushPromises()
    expect(wrapper.text()).toContain('llama3-finetune')
    expect(wrapper.text()).toContain('gpt4-inference')
  })

  it('任务队列 Tab 只显示 pending 作业', async () => {
    const wrapper = mountJob('/job/queue')
    await flushPromises()
    await flushPromises()
    expect(wrapper.text()).toContain('gpt4-inference')
    expect(wrapper.text()).not.toContain('llama3-finetune')
  })

  it('历史记录 Tab 显示 completed/failed/cancelled', async () => {
    const wrapper = mountJob('/job/history')
    await flushPromises()
    await flushPromises()
    expect(wrapper.text()).toContain('batch-job')
    expect(wrapper.text()).toContain('failed-job')
    expect(wrapper.text()).not.toContain('llama3-finetune')
  })

  it('统计信息显示总数', async () => {
    const wrapper = mountJob()
    await flushPromises()
    expect(wrapper.text()).toContain('共 5 个作业')
  })

  it('点击"新建作业"打开对话框', async () => {
    const wrapper = mountJob()
    await flushPromises()
    const btn = wrapper.findAll('button').find((b) => b.text().includes('新建作业'))
    expect(btn).toBeTruthy()
    await btn!.trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain('创建作业')
  })

  it('空数据时显示空态文案', async () => {
    vi.mocked(jobApi.list).mockResolvedValue([])
    const wrapper = mountJob()
    await flushPromises()
    await flushPromises()
    expect(wrapper.text()).toContain('还没有作业')
  })
})
