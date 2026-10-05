import { vi } from 'vitest'
import { config } from '@vue/test-utils'
import { installElementPlus } from '@/plugins/element-plus'

// 全局注册 Element Plus（与 main.ts 同一入口的按需注册），
// 保证测试环境与生产环境组件注册范围一致，漏注册会在测试中提前暴露。
config.global.plugins = [installElementPlus]

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

// ECharts 在 jsdom 下无法初始化 canvas，统一 mock 掉，避免真实渲染。
// 业务代码经 @/theme/echarts 从 'echarts/core' 引入（按需注册），
// 两个模块 id 都要 mock：'echarts'（历史引用）与 'echarts/core'（当前入口）。
// vi.mock 会被提升到文件顶部执行，工厂内引用的变量必须经 vi.hoisted 定义。
const echartsMockFactory = vi.hoisted(
  () =>
    () => ({
      init: vi.fn(() => ({
        setOption: vi.fn(),
        resize: vi.fn(),
        dispose: vi.fn(),
      })),
      use: vi.fn(),
      registerTheme: vi.fn(),
    }),
)
vi.mock('echarts', echartsMockFactory)
vi.mock('echarts/core', echartsMockFactory)

// 每个用例前清理 localStorage，保证用例间隔离
beforeEach(() => {
  localStorage.clear()
})
