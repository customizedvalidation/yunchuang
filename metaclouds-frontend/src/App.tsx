import { lazy, Suspense, useMemo } from 'react';
import { BrowserRouter as Router, Routes, Route, Navigate } from 'react-router-dom';
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

// 导入API服务
import { apiSlice } from './store/api';

// 设计系统：主题模式 + antd 令牌
import { ThemeModeProvider, useThemeMode } from './theme/ThemeModeContext';
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

/** 消费主题模式并注入 antd 主题令牌 */
const ThemedApp = () => {
  const { mode } = useThemeMode();
  const themeConfig = useMemo(() => getThemeConfig(mode), [mode]);

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
                <Route path="/dashboard" element={<Dashboard />} />
                <Route path="/cluster" element={<ClusterManagement />} />
                <Route path="/resource" element={<ResourceManagement />} />
                <Route path="/job" element={<JobManagement />} />
                <Route path="/job/list" element={<JobManagement />} />
                <Route path="/job/queue" element={<JobManagement />} />
                <Route path="/job/history" element={<JobManagement />} />
                <Route path="/monitoring" element={<MonitoringAlert />} />
                <Route path="/tenant" element={<MultiTenantManagement />} />
                <Route path="/acceleration" element={<AccelerationSuiteManagement />} />
                <Route path="/security" element={<SecurityManagement />} />
                <Route path="/k8s" element={<K8SManagement />} />
                <Route path="/k8s/nodes" element={<K8SManagement />} />
                <Route path="/k8s/pods" element={<K8SManagement />} />
                <Route path="/k8s/services" element={<K8SManagement />} />
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
          <ThemedApp />
        </ThemeModeProvider>
      </Provider>
    </ErrorBoundary>
  );
}

export default App;
