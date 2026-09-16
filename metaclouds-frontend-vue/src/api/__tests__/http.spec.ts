/**
 * API 层测试
 *
 * 覆盖：验证各资源 API 正确调用 http 实例的方法与端点路径，
 *       以及统一响应解包后的返回值（.then(r => r.data)）。
 * 通过 mock http 默认导出，避免真实网络。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest'
import http from '../http'

vi.mock('../http', () => {
  const handler: Record<string, (...args: unknown[]) => unknown> = {}
  return {
    default: {
      get: vi.fn((url: string, cfg?: unknown) => handler.get?.(url, cfg) ?? Promise.resolve({ data: undefined })),
      post: vi.fn((url: string, data?: unknown) => handler.post?.(url, data) ?? Promise.resolve({ data: undefined })),
      put: vi.fn((url: string, data?: unknown) => handler.put?.(url, data) ?? Promise.resolve({ data: undefined })),
      delete: vi.fn((url: string) => handler.delete?.(url) ?? Promise.resolve({ data: undefined })),
      __handler: handler,
    },
    setCsrfTokenCache: vi.fn(),
    csrfHeaders: vi.fn(() => ({})),
  }
})

// 由于上面的 mock 替换了 '../http'，这里动态导入被测模块以拿到 mock 后的 http
import * as apiIndex from '../index'

function handler() {
  return (http as unknown as { __handler: Record<string, (...a: unknown[]) => unknown> }).__handler
}

describe('API 端点封装', () => {
  beforeEach(() => {
    const h = handler()
    for (const k of Object.keys(h)) delete h[k]
    vi.mocked(http.get).mockClear()
    vi.mocked(http.post).mockClear()
    vi.mocked(http.put).mockClear()
    vi.mocked(http.delete).mockClear()
  })

  it('authApi.login POST /auth/login 并解包 data', async () => {
    handler().post = vi.fn(() => Promise.resolve({ data: { user: { id: 1 }, expires_at: 123 } }))
    const result = await apiIndex.authApi.login({ username: 'admin', password: 'pw' })
    expect(http.post).toHaveBeenCalledWith('/auth/login', { username: 'admin', password: 'pw' })
    expect(result.user.id).toBe(1)
  })

  it('clusterApi.list GET /clusters 返回数组', async () => {
    handler().get = vi.fn(() => Promise.resolve({ data: [{ id: 1 }] }))
    const list = await apiIndex.clusterApi.list()
    expect(http.get).toHaveBeenCalledWith('/clusters')
    expect(list).toHaveLength(1)
  })

  it('gpuApi.devices 透传查询参数', async () => {
    handler().get = vi.fn(() => Promise.resolve({ data: [] }))
    await apiIndex.gpuApi.devices({ vendor: 'nvidia', status: 'available' })
    expect(http.get).toHaveBeenCalledWith('/gpu/devices', {
      params: { vendor: 'nvidia', status: 'available' },
    })
  })

  it('gpuApi.allocate POST /gpu/allocations', async () => {
    handler().post = vi.fn(() => Promise.resolve({ data: { id: 9 } }))
    const r = await apiIndex.gpuApi.allocate({ device_id: 1, fraction: 0.5 })
    expect(http.post).toHaveBeenCalledWith('/gpu/allocations', { device_id: 1, fraction: 0.5 })
    expect(r.id).toBe(9)
  })

  it('gpuApi.release POST /gpu/allocations/:id/release', async () => {
    handler().post = vi.fn(() => Promise.resolve({ data: undefined }))
    await apiIndex.gpuApi.release(5)
    expect(http.post).toHaveBeenCalledWith('/gpu/allocations/5/release')
  })

  it('partitionApi.setPermission POST 权限', async () => {
    handler().post = vi.fn(() => Promise.resolve({ data: {} }))
    await apiIndex.partitionApi.setPermission(2, { principal_type: 'user', principal_id: 7, access_level: 'view' })
    expect(http.post).toHaveBeenCalledWith('/partitions/2/permissions', expect.objectContaining({ principal_id: 7 }))
  })

  it('quotaApi.create POST /quotas 并解包', async () => {
    handler().post = vi.fn(() => Promise.resolve({ data: { id: 3, scope_type: 'tenant' } }))
    const r = await apiIndex.quotaApi.create({ scope_type: 'tenant', scope_id: 1, resource_type: 'gpu', limit: 8, used: 0 })
    expect(http.post).toHaveBeenCalledWith('/quotas', expect.objectContaining({ resource_type: 'gpu' }))
    expect(r.scope_type).toBe('tenant')
  })

  it('datasetApi.triggerPrefetch POST /fluid-caches/:id/prefetch', async () => {
    handler().post = vi.fn(() => Promise.resolve({ data: undefined }))
    await apiIndex.datasetApi.triggerPrefetch(42)
    expect(http.post).toHaveBeenCalledWith('/fluid-caches/42/prefetch')
  })

  it('checkpointApi.latest GET /checkpoints/latest/:jobId', async () => {
    handler().get = vi.fn(() => Promise.resolve({ data: { id: 1, path: '/ckpt' } }))
    const r = await apiIndex.checkpointApi.latest(10)
    expect(http.get).toHaveBeenCalledWith('/checkpoints/latest/10')
    expect(r.path).toBe('/ckpt')
  })

  it('schedulerApi.syncJobs POST /schedulers/:id/sync', async () => {
    handler().post = vi.fn(() => Promise.resolve({ data: {} }))
    await apiIndex.schedulerApi.syncJobs(3)
    expect(http.post).toHaveBeenCalledWith('/schedulers/3/sync')
  })
})
