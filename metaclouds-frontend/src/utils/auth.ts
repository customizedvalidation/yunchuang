import type { UserRole } from '../types';

/**
 * 平台支持的三类角色，与后端 authz.RoleAdmin / RoleManager / RoleUser 对应。
 */
export const KNOWN_ROLES: readonly UserRole[] = ['admin', 'manager', 'user'];

/**
 * 从 localStorage 的 user 中取出当前角色；取不到或不是已知角色时返回 null。
 *
 * 三个失败分支（无 user、JSON 损坏、role 非法）统一返回 null，由调用方按降级
 * 策略处理 —— 不做"猜一个默认角色"这种事。
 *
 * 这是侧边栏菜单过滤与路由守卫共用的唯一角色来源，两处必须保持同一套语义。
 */
export function readStoredRole(): UserRole | null {
  try {
    const raw = localStorage.getItem('user');
    if (!raw) {
      return null;
    }
    const parsed = JSON.parse(raw) as { role?: unknown };
    const role = parsed?.role;
    if (typeof role !== 'string') {
      return null;
    }
    return KNOWN_ROLES.includes(role as UserRole) ? (role as UserRole) : null;
  } catch {
    return null;
  }
}

/**
 * 判断当前角色是否满足 roles 约束。
 *
 * 降级策略：role 为 null（读不到 / 非法）时**放行**（fail-open）。
 * 理由：前端这一层只是可见性/可达性，真正的访问控制在后端 ——
 * authz.HasPermission 对未知与缺失角色都是 fail-closed 的，越权请求必然 403。
 * 前端若在此 fail-closed，只会把本应有权限的用户挡在门外，换不来任何额外安全性。
 */
export function isRoleAllowed(role: UserRole | null, roles: UserRole[]): boolean {
  if (role === null) {
    return true;
  }
  return roles.includes(role);
}

// ===== 操作级权限（按钮可见性收敛）=====
// 与后端 pkg/authz.rolePermissions 保持同形；后端为权威来源。
// JWT 仅携带 role，不携带权限列表，故前端镜像一份用于按钮级可见性。
export type Permission =
  | 'cluster:read' | 'cluster:write'
  | 'resource:read' | 'resource:write'
  | 'job:read' | 'job:write' | 'job:submit'
  | 'tenant:read' | 'tenant:write'
  | 'monitoring:read' | 'monitoring:write'
  | 'acceleration:read' | 'acceleration:write'
  | 'security:read' | 'security:write'
  | 'admin';

// 镜像后端 rolePermissions（pkg/authz/authz.go）。后端调整权限矩阵时此处必须同步。
const ROLE_PERMISSIONS: Record<UserRole, Permission[]> = {
  admin: ['admin', 'cluster:read', 'cluster:write', 'resource:read', 'resource:write', 'job:read', 'job:write', 'job:submit', 'tenant:read', 'tenant:write', 'monitoring:read', 'monitoring:write', 'acceleration:read', 'acceleration:write', 'security:read', 'security:write'],
  manager: ['cluster:read', 'cluster:write', 'resource:read', 'resource:write', 'job:read', 'job:write', 'job:submit', 'tenant:read', 'tenant:write', 'monitoring:read', 'monitoring:write', 'acceleration:read', 'acceleration:write', 'security:read'],
  user: ['cluster:read', 'resource:read', 'job:read', 'monitoring:read', 'acceleration:read', 'security:read'],
};

/**
 * 判断当前登录用户是否具备某项权限（前端按钮级可见性用）。
 * 降级策略与 isRoleAllowed 一致：role 为 null（读不到 / 非法）时放行（fail-open）。
 * 真正的访问控制在后端 authz.HasPermission —— 它对未知/缺失角色 fail-closed，越权请求必然 403。
 */
export function hasPermission(permission: Permission): boolean {
  const role = readStoredRole();
  if (role === null) return true;
  if (role === 'admin') return true;
  return ROLE_PERMISSIONS[role].includes(permission);
}
