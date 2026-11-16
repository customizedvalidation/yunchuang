/**
 * K8S 管理页面测试
 *
 * 本页后端 /k8s/* 为 MockK8sClient，前端三个 Tab 借用其它资源语义，
 * 因此必须显著标注「非真实 Kubernetes 对象」，避免用户误读为已完成真实接入。
 * 这些测试锁死该诚实性契约，防止标注被误删后页面被当作真实 K8s 能力。
 */
import { describe, it, expect, vi } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createPinia } from 'pinia'
import { createRouter, createMemoryHistory } from 'vue-router'

vi.mock('@/api', () => ({
  jobApi: { list: vi.fn() },
  resourceApi: { list: vi.fn(), gpuResources: vi.fn() },
}))

import K8SManagement from '../K8SManagement.vue'
import { jobApi, resourceApi } from '@/api'

function mountPage() {
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [
      { path: '/k8s/nodes', component: K8SManagement },
      { path: '/k8s/pods', component: K8SManagement },
      { path: '/k8s/services', component: K8SManagement },
    ],
  })
  router.push('/k8s/nodes')
  return mount(K8SManagement, {
    global: { plugins: [createPinia(), router] },
  })
}

describe('K8SManagement 页面', () => {
  it('显著标注数据来源为演示/语义借用，不冒充真实 Kubernetes 对象', async () => {
    vi.mocked(resourceApi.gpuResources).mockResolvedValue([])
    vi.mocked(resourceApi.list).mockResolvedValue([])
    vi.mocked(jobApi.list).mockResolvedValue([])

    const wrapper = mountPage()
    await flushPromises()
    await flushPromises()

    const text = wrapper.text()
    expect(text).toContain('演示视图')
    expect(text).toContain('非真实 Kubernetes 对象')
    // 三个 Tab 的真实数据来源逐一说明，避免「节点/Pod/服务」标题造成误导
    expect(text).toContain('GPU 资源池')
    expect(text).toContain('运行中作业派生')
    expect(text).toContain('mock')
    expect(text).toContain('kube-rs')
  })
})
