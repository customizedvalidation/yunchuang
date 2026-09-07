/**
 * Jest 全局测试设置
 *
 * 在每个测试文件执行前运行，配置：
 * 1. jest-dom 扩展匹配器（toBeInTheDocument / toHaveTextContent 等）
 * 2. localStorage / sessionStorage mock（node/jsdom 环境补全）
 * 3. 浏览器 API mock（matchMedia / ResizeObserver / IntersectionObserver）
 * 4. MSW Mock Service Worker（离线环境未安装，保留配置注释）
 *
 * 注意：原 src/setupTests.ts 仅包含 localStorage mock，已合并到此文件。
 * 原文件保留不删除（硬约束），jest.config.js 的 setupFilesAfterEnv 已指向本文件。
 */
import '@testing-library/jest-dom';

// ===== localStorage / sessionStorage mock =====
// jsdom 环境已提供 localStorage，但部分版本实现不完整；这里做兜底补全。
class LocalStorageMock {
  private store: Record<string, string> = {};

  getItem(key: string): string | null {
    return Object.prototype.hasOwnProperty.call(this.store, key) ? this.store[key] : null;
  }

  setItem(key: string, value: string): void {
    this.store[key] = String(value);
  }

  removeItem(key: string): void {
    delete this.store[key];
  }

  clear(): void {
    this.store = {};
  }

  key(index: number): string | null {
    return Object.keys(this.store)[index] ?? null;
  }

  get length(): number {
    return Object.keys(this.store).length;
  }
}

// 仅在 localStorage 不可用时注入 mock（jsdom 通常已提供）
if (typeof globalThis.localStorage === 'undefined') {
  (globalThis as unknown as { localStorage: Storage }).localStorage =
    new LocalStorageMock() as unknown as Storage;
}
if (typeof globalThis.sessionStorage === 'undefined') {
  (globalThis as unknown as { sessionStorage: Storage }).sessionStorage =
    new LocalStorageMock() as unknown as Storage;
}

// ===== 浏览器 API mock =====

// matchMedia：antd / ResponsiveTable 等组件使用 Grid.useBreakpoint() 依赖 matchMedia
if (typeof window !== 'undefined' && !window.matchMedia) {
  window.matchMedia = (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  });
}

// ResizeObserver：ResponsiveChart 使用 ResizeObserver 监听容器尺寸
if (typeof globalThis.ResizeObserver === 'undefined') {
  class ResizeObserverMock {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  (globalThis as unknown as { ResizeObserver: typeof ResizeObserver }).ResizeObserver =
    ResizeObserverMock;
}

// IntersectionObserver：部分 antd 组件可能使用
if (typeof globalThis.IntersectionObserver === 'undefined') {
  class IntersectionObserverMock {
    observe() {}
    unobserve() {}
    disconnect() {}
    takeRecords() {
      return [];
    }
  }
  (globalThis as unknown as { IntersectionObserver: unknown }).IntersectionObserver =
    IntersectionObserverMock;
}

// ===== MSW Mock Service Worker（可选启用）=====
// 离线环境未安装 msw，有网环境执行：npm install -D msw
// 安装后取消下方注释，即可用 MSW 拦截真实 API 请求（替代 jest.mock 方案）：
//
// import { setupServer } from 'msw/node';
// import { handlers } from './mocks/handlers';
// export const server = setupServer(...handlers);
// beforeAll(() => server.listen({ onUnhandledRequest: 'error' }));
// afterEach(() => server.resetHandlers());
// afterAll(() => server.close());

// ===== 全局测试清理 =====
// 每个测试后清理 localStorage，避免测试间状态污染
afterEach(() => {
  globalThis.localStorage?.clear();
  globalThis.sessionStorage?.clear();
});
