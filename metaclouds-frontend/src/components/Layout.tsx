import React, { useEffect, useState } from 'react';
import { Layout } from 'antd';
import Sidebar from './Sidebar';
import Topbar from './Topbar/Topbar';
import { SIDEBAR_WIDTH, SIDEBAR_BREAKPOINTS } from '../theme/sidebar';

const { Content } = Layout;

interface LayoutProps {
  children: React.ReactNode;
}

/** 依据视口宽度计算内容区左边距（移动端侧边栏为浮层，不占位） */
const getContentMargin = (collapsed: boolean, viewportWidth: number): number => {
  if (viewportWidth <= SIDEBAR_BREAKPOINTS.MOBILE) return 0;
  if (collapsed) return SIDEBAR_WIDTH.COLLAPSED;
  if (viewportWidth <= SIDEBAR_BREAKPOINTS.TABLET) return SIDEBAR_WIDTH.TABLET;
  return SIDEBAR_WIDTH.DEFAULT;
};

const LayoutComponent: React.FC<LayoutProps> = ({ children }) => {
  const [collapsed, setCollapsed] = useState(false);
  const [viewportWidth, setViewportWidth] = useState(() =>
    typeof window === 'undefined' ? 1440 : window.innerWidth,
  );

  // 视口变化时重新计算布局（移动端浮层 → 桌面占位 的切换依赖此状态）
  useEffect(() => {
    let frame = 0;
    const handleResize = () => {
      if (frame) return;
      frame = window.requestAnimationFrame(() => {
        frame = 0;
        setViewportWidth(window.innerWidth);
      });
    };
    window.addEventListener('resize', handleResize);
    return () => {
      window.removeEventListener('resize', handleResize);
      if (frame) window.cancelAnimationFrame(frame);
    };
  }, []);

  // 窄屏默认收起侧边栏，避免遮挡内容
  useEffect(() => {
    if (viewportWidth <= SIDEBAR_BREAKPOINTS.TABLET) setCollapsed(true);
  }, [viewportWidth]);

  const marginLeft = getContentMargin(collapsed, viewportWidth);

  return (
    <Layout style={{ minHeight: '100vh', background: 'var(--mc-bg)' }}>
      <Sidebar collapsed={collapsed} onCollapse={setCollapsed} />
      <Layout
        style={{
          minHeight: '100vh',
          background: 'transparent',
          marginLeft,
          transition: 'margin-left 0.28s cubic-bezier(0.2, 0.8, 0.2, 1)',
        }}
      >
        <Topbar />
        <Content style={{ minWidth: 0 }}>{children}</Content>
      </Layout>
    </Layout>
  );
};

export default LayoutComponent;
