import { lazy, Suspense, useEffect, useMemo } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate, Outlet } from 'react-router-dom';
import { Provider } from 'react-redux';
import { configureStore } from '@reduxjs/toolkit';
import { setupListeners } from '@reduxjs/toolkit/query';
import { ConfigProvider, Spin, App as AntApp } from 'antd';
import zhCN from 'antd/locale/zh_CN';

// 导入ErrorBoundary组件（不使用懒加载）
import ErrorBoundary from './components/ErrorBoundary';

// 导入页面组件（使用懒加载）
const Login = lazy(() => import('./pages/Login'));
const Dashboard = lazy(() => import('./pages/Dashboard'));
const ClusterManagement = lazy(() => import('./pages/ClusterManagement'));
const ResourceManagement = lazy(() => import('./pages/ResourceManagement'));
const JobManagement = lazy(() => import('./pages/JobManagement'));
const MonitoringAlert = lazy(() => import('./pages/MonitoringAlert'));
const MultiTenantManagement = lazy(() => import('./pages/MultiTenantManagement'));
const AccelerationSuiteManagement = lazy(() => import('./pages/AccelerationSuiteManagement'));
const SecurityManagement = lazy(() => import('./pages/SecurityManagement'));
const K8SManagement = lazy(() => import('./pages/K8SManagement'));

// 导入PrivateRoute组件
const PrivateRoute = lazy(() => import('./components/PrivateRoute'));

// 页面框架（侧边栏 + 顶栏 + 内容区）。原先由 PrivateRoute 内部渲染，
// 上提为独立布局路由后，PrivateRoute 才是纯守卫，角色守卫才能嵌套使用。
const LayoutRoute = lazy(() => import('./components/Layout'));

// 导入API服务
import { apiSlice, useRefreshTokenMutation } from './store/api';

// 设计系统：主题模式 + antd 令牌
import { ThemeModeProvider, useThemeMode } from './theme/ThemeModeContext';
import { DensityProvider, useDensity } from './theme/DensityContext';
import { getThemeConfig } from './theme/antdTheme';
import { CommandPaletteProvider } from './components/CommandPalette';

// 配置Redux store
const store = configureStore({
  reducer: {
    [apiSlice.reducerPath]: apiSlice.reducer,
  },
  middleware: (getDefaultMiddleware) =>
    getDefaultMiddleware().concat(apiSlice.middleware),
});

// 设置API listeners
setupListeners(store.dispatch);

/** 令牌剩余有效期低于该阈值（1h）时主动续期 */
const TOKEN_REFRESH_THRESHOLD_MS = 60 * 60 * 1000;
/** 续期检查间隔（5min） */
const TOKEN_REFRESH_CHECK_INTERVAL_MS = 5 * 60 * 1000;

/**
 * 静默续期：令牌临近过期时调用 POST /auth/refresh 换取新令牌并刷新本地非敏感缓存，
 * 关闭"24h 到期整页登出"的缺口。
 *
 * 该端点与业务端点一样要求 Cookie 中的令牌未过期，所以这是"有效期内换新"而非"过期后自救"；
 * 一旦真的过期，refresh 会失败，此时安静忽略，交由统一的业务端点 401 逻辑跳登录。
 * 续期判据来自本地存储的 auth_expiry（后端 expires_at 换算的毫秒时间戳，非敏感），
 * 不再读取 JWT 本身。
 */
function useSilentTokenRefresh(): void {
  const [refreshToken] = useRefreshTokenMutation();

  useEffect(() => {
    const refreshIfNeeded = async (): Promise<void> => {
      const expiresAtStr = localStorage.getItem('auth_expiry');
      if (!expiresAtStr) {
        return;
      }
      const expiresAtMs = Number(expiresAtStr);
      const remainingMs = expiresAtMs - Date.now();
      // 已过期：refresh 端点同样要求令牌有效，此时调用必然 401，直接放弃。
      if (remainingMs <= 0) {
        return;
      }
      if (remainingMs > TOKEN_REFRESH_THRESHOLD_MS) {
        return;
      }

      try {
        const result = await refreshToken().unwrap();
        const { user, expires_at } = result.data;
        localStorage.setItem(
          'user',
          JSON.stringify({
            username: user?.username ?? '',
            email: user?.email ?? '',
            role: user?.role ?? '',
          }),
        );
        localStorage.setItem('auth_expiry', String((expires_at ?? 0) * 1000));
      } catch {
        // 续期失败（含令牌已过期）静默忽略，不做整页跳转。
      }
    };

    void refreshIfNeeded();
    const timer = setInterval(() => void refreshIfNeeded(), TOKEN_REFRESH_CHECK_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [refreshToken]);
}

/** 消费主题模式并注入 antd 主题令牌 */
const ThemedApp = () => {
  useSilentTokenRefresh();

  const { mode } = useThemeMode();
  const { density } = useDensity();
  const themeConfig = useMemo(() => getThemeConfig(mode, density), [mode, density]);

  return (
    <ConfigProvider theme={themeConfig} locale={zhCN}>
      <AntApp>
        <Router future={{ v7_startTransition: true, v7_relativeSplatPath: true }}>
          <CommandPaletteProvider>
            <Suspense
            fallback={
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  minHeight: '100vh',
                  background: 'var(--mc-bg)',
                }}
              >
                <Spin size="large" />
              </div>
            }
          >
            <Routes>
              <Route path="/login" element={<Login />} />
              <Route element={<PrivateRoute />}>
                <Route element={<LayoutRoute><Outlet /></LayoutRoute>}>
                  <Route path="/dashboard" element={<Dashboard />} />
                  <Route path="/cluster" element={<ClusterManagement />} />
                  <Route path="/resource" element={<ResourceManagement />} />
                  <Route path="/job" element={<JobManagement />} />
                  <Route path="/job/list" element={<JobManagement />} />
                  <Route path="/job/queue" element={<JobManagement />} />
                  <Route path="/job/history" element={<JobManagement />} />
                  <Route path="/monitoring" element={<MonitoringAlert />} />
                  {/* 多租户管理收敛为 admin/manager：对齐后端 tenant:read。
                      用嵌套路由而非单点判断，后续若新增 /tenant/:id 等子路由会自动被覆盖。 */}
                  <Route element={<PrivateRoute roles={['admin', 'manager']} />}>
                    <Route path="/tenant" element={<MultiTenantManagement />} />
                  </Route>
                  <Route path="/acceleration" element={<AccelerationSuiteManagement />} />
                  <Route path="/security" element={<SecurityManagement />} />
                  <Route path="/k8s" element={<K8SManagement />} />
                  <Route path="/k8s/nodes" element={<K8SManagement />} />
                  <Route path="/k8s/pods" element={<K8SManagement />} />
                  <Route path="/k8s/services" element={<K8SManagement />} />
                </Route>
              </Route>
              <Route path="/" element={<Navigate to="/login" replace />} />
              <Route path="*" element={<Navigate to="/dashboard" replace />} />
            </Routes>
          </Suspense>
        </CommandPaletteProvider>
      </Router>
      </AntApp>
    </ConfigProvider>
  );
};

function App() {
  return (
    <ErrorBoundary>
      <Provider store={store}>
      <ThemeModeProvider>
        <DensityProvider>
          <ThemedApp />
        </DensityProvider>
      </ThemeModeProvider>
      </Provider>
    </ErrorBoundary>
  );
}

export default App;
