/**
 * Jest 配置 — 组件级测试体系
 *
 * 依赖说明（离线环境未安装，有网环境需执行）：
 *   npm install -D jest-environment-jsdom identity-obj-proxy msw
 *
 * - jest-environment-jsdom：提供浏览器 DOM 环境（React Testing Library 必需）
 * - identity-obj-proxy：CSS Module 类名代理（当前用自定义 styleMock 替代，安装后可切换）
 * - msw：API 请求 Mock（当前用 jest.mock 替代 RTK Query，安装后可切换为 MSW）
 */
export default {
  preset: 'ts-jest',
  // 组件测试需要 DOM 环境（document / window / HTMLElement 等）
  testEnvironment: 'jsdom',
  moduleNameMapper: {
    // CSS / LESS / SCSS 导入代理：离线环境未安装 identity-obj-proxy，
    // 使用自定义空对象 mock；安装 identity-obj-proxy 后可改为 'identity-obj-proxy'
    '\\.(css|less|scss|sass)$': '<rootDir>/src/test/__mocks__/styleMock.js',
    // 静态资源代理（图片 / 字体等）
    '\\.(jpg|jpeg|png|gif|svg|woff|woff2|ttf|eot)$': '<rootDir>/src/test/__mocks__/fileMock.js',
  },
  // 每个测试文件执行前加载 jest-dom 扩展匹配器 + 全局 mock
  setupFilesAfterEnv: ['<rootDir>/src/test/setup.ts'],
  testPathIgnorePatterns: ['<rootDir>/node_modules/', '<rootDir>/build/'],
  transform: {
    '^.+\\.(ts|tsx)$': ['ts-jest', {
      // 测试环境使用独立的 tsconfig 配置，放宽 noUnusedLocals 等限制
      tsconfig: {
        jsx: 'react-jsx',
        esModuleInterop: true,
        allowSyntheticDefaultImports: true,
        module: 'commonjs',
        target: 'ES2020',
        lib: ['ES2020', 'DOM', 'DOM.Iterable'],
        skipLibCheck: true,
        strict: true,
        moduleResolution: 'node',
        resolveJsonModule: true,
        isolatedModules: true,
      },
    }],
  },
  testMatch: ['**/__tests__/**/*.(ts|tsx)', '**/*.(test|spec).(ts|tsx)'],
  moduleFileExtensions: ['ts', 'tsx', 'js', 'jsx', 'json', 'node'],
  testEnvironmentOptions: {
    customExportConditions: ['node', 'node-addons'],
  },
  // 测试覆盖率配置（执行 npm test -- --coverage 时生效）
  collectCoverageFrom: [
    'src/**/*.{ts,tsx}',
    '!src/**/*.d.ts',
    '!src/main.tsx',
    '!src/test/**',
  ],
};
