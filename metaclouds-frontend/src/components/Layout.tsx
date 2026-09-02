import React, { useEffect, useState } from 'react';
import Sidebar from './Sidebar';
import Topbar from './Topbar/Topbar';

interface LayoutProps {
  children: React.ReactNode;
}

/**
 * 应用外壳（PRD R2 / R3-a）。
 *
 * 布局完全由 CSS Grid 驱动（grid-template-columns: var(--mc-sider-w) minmax(0,1fr)），
 * 断点只改 --mc-sider-w 变量，组件内**不做任何 window.innerWidth / resize 计算**，
 * 因此 resize 期间无抖动、无 React 重渲染（对应 G3）。
 *
 * 仅保留两类交互态：
 *  - collapsed：桌面侧边栏折叠为图标轨（切 .mc-app-shell.is-collapsed，改 --mc-sider-w）
 *  - mobileOpen：<1024 时侧边栏浮层（Drawer）开关，配套遮罩 / Esc / 锁定 body 滚动（G2）
 */
const LayoutComponent: React.FC<LayoutProps> = ({ children }) => {
  const [collapsed, setCollapsed] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);

  // 仅处理交互副作用（Esc 关闭 + 锁定 body 滚动），不做布局计算（G3）
  useEffect(() => {
    if (!mobileOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setMobileOpen(false);
    };
    document.body.style.overflow = 'hidden';
    window.addEventListener('keydown', onKey);
    return () => {
      document.body.style.overflow = '';
      window.removeEventListener('keydown', onKey);
    };
  }, [mobileOpen]);

  return (
    <div
      className={`mc-app-shell ${collapsed ? 'is-collapsed' : ''} ${
        mobileOpen ? 'is-mobile-open' : ''
      }`}
    >
      <Sidebar collapsed={collapsed} onCollapse={setCollapsed} mobileOpen={mobileOpen} />
      <div className="mc-app-main">
        <Topbar onMenuClick={() => setMobileOpen(true)} />
        <main className="mc-app-content">{children}</main>
      </div>
      <div
        className="mc-app-overlay"
        onClick={() => setMobileOpen(false)}
        aria-hidden="true"
      />
    </div>
  );
};

export default LayoutComponent;
