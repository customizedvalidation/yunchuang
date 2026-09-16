import { vi } from 'vitest'
import { config } from '@vue/test-utils'
import ElementPlus from 'element-plus'

// 全局注册 Element Plus，与 main.ts 保持一致，便于在组件测试中直接使用 el-* 组件
config.global.plugins = [ElementPlus]

// jsdom 不 matchMedia，Element Plus / ECharts 部分组件依赖它
if (!window.matchMedia) {
  window.matchMedia = vi.fn().mockImplementation((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }))
}

// jsdom 不支持 ResizeObserver，ECharts / Element Plus 部分组件依赖
if (!window.ResizeObserver) {
  class ResizeObserverStub {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  ;(window as unknown as { ResizeObserver: typeof ResizeObserver }).ResizeObserver =
    ResizeObserverStub as unknown as typeof ResizeObserver
}

// ECharts 在 jsdom 下无法初始化 canvas，统一 mock 掉，避免真实渲染
vi.mock('echarts', () => ({
  init: vi.fn(() => ({
    setOption: vi.fn(),
    resize: vi.fn(),
    dispose: vi.fn(),
  })),
}))

// 每个用例前清理 localStorage，保证用例间隔离
beforeEach(() => {
  localStorage.clear()
})
