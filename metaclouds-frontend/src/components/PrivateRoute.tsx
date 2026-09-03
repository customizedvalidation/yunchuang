import React, { useState, useEffect } from 'react';
import { Navigate, Outlet } from 'react-router-dom';
import { Spin } from 'antd';
import { isRoleAllowed, readStoredRole } from '../utils/auth';
import type { UserRole } from '../types';

export interface PrivateRouteProps {
  /**
   * 访问该路由所需的角色，须与后端 authz.rolePermissions 对齐。
   * 不传表示不做角色限制（只校验 token），用于保持其余路由的既有行为。
   */
  roles?: UserRole[];
}

/**
 * 受保护路由的守卫：校验 token 是否有效，并在给定 roles 时校验角色。
 *
 * 本组件只做守卫、渲染 Outlet，不再负责页面框架布局 —— 布局由外层路由的
 * LayoutComponent 承担。这样角色守卫才能作为子路由嵌套使用
 * （例如 /tenant 需要额外收敛），而不会重复渲染出两套侧边栏。
 */
const PrivateRoute: React.FC<PrivateRouteProps> = ({ roles }) => {
  const [loading, setLoading] = useState(true);
  const [isAuthenticated, setIsAuthenticated] = useState(false);

  useEffect(() => {
    // 不再读取 JWT（已移入 httpOnly Cookie）。用本地非敏感的 auth_expiry 时间戳判断登录态：
    // 它存在且未过期即视为已登录；真正的令牌有效性始终由后端在每次请求时校验。
    const expiresAtStr = localStorage.getItem('auth_expiry');
    if (!expiresAtStr) {
      setIsAuthenticated(false);
      setLoading(false);
      return;
    }

    const expiresAtMs = Number(expiresAtStr);
    if (!expiresAtMs || Date.now() > expiresAtMs) {
      localStorage.removeItem('user');
      localStorage.removeItem('auth_expiry');
      setIsAuthenticated(false);
    } else {
      setIsAuthenticated(true);
    }

    setLoading(false);
  }, []);

  if (loading) {
    return (
      <div className="flex items-center justify-center min-h-screen bg-gray-100">
        <Spin size="large" />
      </div>
    );
  }

  if (!isAuthenticated) {
    return <Navigate to="/login" replace />;
  }

  // 角色不满足时退回仪表盘（而非登录页）：用户本身是合法登录态，
  // 只是没有这条路由的权限，踢去重新登录是误导。
  // role 为 null 时 isRoleAllowed 放行，与侧边栏菜单过滤保持同一套降级策略。
  if (roles && !isRoleAllowed(readStoredRole(), roles)) {
    return <Navigate to="/dashboard" replace />;
  }

  return <Outlet />;
};

export default PrivateRoute;
