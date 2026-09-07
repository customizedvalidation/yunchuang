/**
 * 测试工具函数 — 自定义 render
 *
 * 封装 @testing-library/react 的 render，自动包裹应用所需的全部 Provider：
 * 1. Redux Provider（包含 apiSlice reducer + middleware）
 * 2. MemoryRouter（路由测试，避免依赖真实浏览器 URL）
 * 3. ThemeModeProvider + DensityProvider（主题上下文）
 * 4. antd ConfigProvider（中文 locale + 主题令牌）
 * 5. AntApp（antd 5.x App 组件，提供 message/notification 上下文）
 *
 * 用法：
 *   import { render, screen, waitFor } from '../test/test-utils';
 *   render(<Login />);
 *   expect(screen.getByText('登录')).toBeInTheDocument();
 */
import React, { type ReactElement } from 'react';
import { render as rtlRender, type RenderOptions, type RenderResult } from '@testing-library/react';
import { Provider } from 'react-redux';
import { configureStore } from '@reduxjs/toolkit';
import { MemoryRouter } from 'react-router-dom';
import { ConfigProvider, App as AntApp } from 'antd';
import zhCN from 'antd/locale/zh_CN';

import { apiSlice } from '../store/api';
import { ThemeModeProvider } from '../theme/ThemeModeContext';
import { DensityProvider } from '../theme/DensityContext';
import { getThemeConfig } from '../theme/antdTheme';

/**
 * 创建测试用 Redux store
 *
 * 与生产 store 结构一致（apiSlice reducer + middleware），
 * 但不启用 setupListeners（测试环境不需要 WebSocket/重连监听）。
 * 可通过 preloadedState 注入初始状态。
 */
export function createTestStore(preloadedState?: Record<string, unknown>) {
  return configureStore({
    reducer: {
      [apiSlice.reducerPath]: apiSlice.reducer,
    } as Record<string, typeof apiSlice.reducer>,
    middleware: (getDefaultMiddleware) =>
      getDefaultMiddleware().concat(apiSlice.middleware),
    preloadedState,
  });
}

export interface WrapperProps {
  children: React.ReactNode;
  /** 初始路由路径，默认 '/' */
  route?: string;
  /** 预加载的 Redux 状态 */
  preloadedState?: Record<string, unknown>;
}

/**
 * 全局 Provider 包装器
 *
 * 所有测试组件都会被此包装器包裹，确保组件能访问到：
 * - Redux store（useGetXxxQuery / useXxxMutation 等 RTK Query hooks）
 * - 路由上下文（useNavigate / useLocation / useParams）
 * - 主题上下文（useThemeMode / useDensity）
 * - antd 组件上下文（App.useApp / ConfigProvider）
 */
export function AllProviders({ children, route = '/', preloadedState }: WrapperProps) {
  const store = createTestStore(preloadedState);
  const themeConfig = getThemeConfig('light', 'compact');

  return (
    <Provider store={store}>
      <MemoryRouter initialEntries={[route]}>
        <ThemeModeProvider>
          <DensityProvider>
            <ConfigProvider theme={themeConfig} locale={zhCN}>
              <AntApp>{children}</AntApp>
            </ConfigProvider>
          </DensityProvider>
        </ThemeModeProvider>
      </MemoryRouter>
    </Provider>
  );
}

/**
 * 自定义 render
 *
 * 与 @testing-library/react 的 render API 完全一致，但自动包裹 AllProviders。
 * 返回值包含 render 的全部属性（container / rerender / unmount 等），
 * 额外返回 store 便于测试中手动 dispatch action 或检查状态。
 */
interface CustomRenderResult extends RenderResult {
  store: ReturnType<typeof createTestStore>;
}

export function render(
  ui: ReactElement,
  options?: Omit<RenderOptions, 'wrapper'> & { route?: string; preloadedState?: Record<string, unknown> },
): CustomRenderResult {
  const { route, preloadedState, ...rtlOptions } = options ?? {};
  const store = createTestStore(preloadedState);

  const result = rtlRender(ui, {
    wrapper: ({ children }) => (
      <Provider store={store}>
        <MemoryRouter initialEntries={[route ?? '/']}>
          <ThemeModeProvider>
            <DensityProvider>
              <ConfigProvider theme={getThemeConfig('light', 'compact')} locale={zhCN}>
                <AntApp>{children}</AntApp>
              </ConfigProvider>
            </DensityProvider>
          </ThemeModeProvider>
        </MemoryRouter>
      </Provider>
    ),
    ...rtlOptions,
  });

  return { ...result, store };
}

// 重新导出 @testing-library/react 的常用工具，测试文件只需从 test-utils 导入
export {
  screen,
  fireEvent,
  waitFor,
  waitForElementToBeRemoved,
  within,
  cleanup,
  act,
} from '@testing-library/react';

// 重新导出 userEvent（模拟真实用户交互，优先于 fireEvent）
export { default as userEvent } from '@testing-library/user-event';
