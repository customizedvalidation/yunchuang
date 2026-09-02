import React, { useMemo, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { Badge, Dropdown, Tooltip } from 'antd';
import { useGetAlertsQuery } from '../../store/api';
import { extractArrayData } from '../../utils/api';
import { useThemeMode } from '../../theme/ThemeModeContext';
import { useCommandPalette } from '../CommandPalette';
import './Topbar.css';

/** 路径 → 面包屑（分组 / 页面名） */
const CRUMB_MAP: Record<string, [string, string]> = {
  '/dashboard': ['总览', '仪表盘'],
  '/job': ['调度', '作业管理'],
  '/job/list': ['调度', '作业列表'],
  '/job/queue': ['调度', '任务队列'],
  '/job/history': ['调度', '历史记录'],
  '/cluster': ['基础设施', '集群管理'],
  '/resource': ['基础设施', '资源管理'],
  '/k8s': ['基础设施', 'K8S 管理'],
  '/k8s/nodes': ['基础设施', '节点管理'],
  '/k8s/pods': ['基础设施', 'Pod 管理'],
  '/k8s/services': ['基础设施', '服务管理'],
  '/tenant': ['平台', '多租户管理'],
  '/acceleration': ['平台', '加速套件'],
  '/monitoring': ['可观测与治理', '监控告警'],
  '/security': ['可观测与治理', '安全管理'],
};

const IS_MAC = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);

const Topbar: React.FC<{ onMenuClick: () => void }> = ({ onMenuClick }) => {
  const location = useLocation();
  const navigate = useNavigate();
  const { isDark, toggle } = useThemeMode();
  const { isCompact, toggle: toggleDensity } = useDensity();
  const { open: openCommandPalette } = useCommandPalette();
  const { data: alerts } = useGetAlertsQuery(undefined);
  const alertsData = extractArrayData(alerts);

  const [group, page] = useMemo<[string, string]>(
    () => CRUMB_MAP[location.pathname] ?? ['', '控制台'],
    [location.pathname],
  );

  const username = useMemo(() => {
    try {
      const raw = localStorage.getItem('user');
      if (!raw) return 'admin';
      const u = JSON.parse(raw) as { username?: string; name?: string };
      return u.username || u.name || 'admin';
    } catch {
      return 'admin';
    }
  }, []);

  const [envLabel] = useState(() => {
    const host = typeof window !== 'undefined' ? window.location.hostname : '';
    const isLocal = host === 'localhost' || host === '127.0.0.1';
    return isLocal ? '本地开发环境' : '生产环境';
  });

  const handleLogout = () => {
    localStorage.removeItem('token');
    localStorage.removeItem('user');
    navigate('/login', { replace: true });
  };

  const userMenu = {
    items: [
      { key: 'who', label: `已登录：${username}`, disabled: true },
      { type: 'divider' as const },
      { key: 'logout', label: '退出登录', danger: true },
    ],
    onClick: ({ key }: { key: string }) => {
      if (key === 'logout') handleLogout();
    },
  };

  return (
    <header className="mc-topbar">
      <button
        type="button"
        className="mc-menu-trigger"
        onClick={onMenuClick}
        aria-label="打开菜单"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true">
          <path d="M4 6h16M4 12h16M4 18h16" strokeLinecap="round" />
        </svg>
      </button>
      <button
        type="button"
        className="mc-cmd-trigger"
        onClick={openCommandPalette}
        aria-label="搜索"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true">
          <circle cx="11" cy="11" r="8" />
          <path d="m21 21-4.3-4.3" strokeLinecap="round" />
        </svg>
      </button>
      <nav className="mc-crumb" aria-label="面包屑">
        {group && (
          <>
            <span className="mc-crumb-group">{group}</span>
            <span className="mc-crumb-sep" aria-hidden="true">
              /
            </span>
          </>
        )}
        <b className="mc-crumb-page">{page}</b>
      </nav>

      <button
        type="button"
        className="mc-search"
        onClick={openCommandPalette}
        aria-label="打开命令面板"
      >
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true">
          <circle cx="11" cy="11" r="8" />
          <path d="m21 21-4.3-4.3" strokeLinecap="round" />
        </svg>
        <span className="mc-search-ph">搜索作业、集群、页面…</span>
        <kbd className="mc-kbd">{IS_MAC ? '⌘' : 'Ctrl'} K</kbd>
      </button>

      <span className="mc-env">
        <i className="mc-env-dot" aria-hidden="true" />
        {envLabel}
      </span>

      <Tooltip title="监控告警">
        <button
          type="button"
          className="mc-icon-btn"
          onClick={() => navigate('/monitoring')}
          aria-label={`监控告警，${alertsData.length} 条`}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true">
            <path d="M18 8a6 6 0 1 0-12 0c0 7-3 9-3 9h18s-3-2-3-9" strokeLinecap="round" strokeLinejoin="round" />
            <path d="M13.7 21a2 2 0 0 1-3.4 0" strokeLinecap="round" />
          </svg>
          <Badge count={alertsData.length} size="small" offset={[-2, 2]} />
        </button>
      </Tooltip>

      <Tooltip title={isCompact ? '切换到舒适密度' : '切换到紧凑密度'}>
        <button
          type="button"
          className="mc-icon-btn"
          onClick={toggleDensity}
          aria-label={isCompact ? '切换到舒适密度' : '切换到紧凑密度'}
        >
          {isCompact ? (
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true">
              <path d="M4 7h16M4 12h16M4 17h16" strokeLinecap="round" />
            </svg>
          ) : (
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true">
              <path d="M4 9.5h16M4 12h16M4 14.5h16" strokeLinecap="round" />
            </svg>
          )}
        </button>
      </Tooltip>

      <Tooltip title={isDark ? '切换到浅色模式' : '切换到深色模式'}>
        <button
          type="button"
          className="mc-icon-btn"
          onClick={toggle}
          aria-label={isDark ? '切换到浅色模式' : '切换到深色模式'}
        >
          {isDark ? (
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true">
              <path
                d="M12 4V2M12 22v-2M4 12H2M22 12h-2M6.3 6.3 4.9 4.9M19.1 19.1l-1.4-1.4M6.3 17.7l-1.4 1.4M19.1 4.9l-1.4 1.4"
                strokeLinecap="round"
              />
              <circle cx="12" cy="12" r="4" />
            </svg>
          ) : (
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true">
              <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          )}
        </button>
      </Tooltip>

      <Dropdown menu={userMenu} placement="bottomRight" trigger={['click']}>
        <button type="button" className="mc-user" aria-label="用户菜单">
          <span className="mc-avatar">{username.slice(0, 2).toUpperCase()}</span>
          <span className="mc-user-name">{username}</span>
          <svg
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            className="mc-user-caret"
            aria-hidden="true"
          >
            <path d="m6 9 6 6 6-6" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </button>
      </Dropdown>
    </header>
  );
};

export default Topbar;
