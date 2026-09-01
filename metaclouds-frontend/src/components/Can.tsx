import type { ReactNode } from 'react';
import { hasPermission, type Permission } from '../utils/auth';

/**
 * 按权限收敛子元素可见性；权限不足时不渲染任何内容。
 * 真实授权仍在后端 authz.HasPermission 强制，此处仅为 UX 收敛（与菜单/路由守卫同类）。
 */
export function Can({ perm, children }: { perm: Permission; children: ReactNode }) {
  return hasPermission(perm) ? <>{children}</> : null;
}
