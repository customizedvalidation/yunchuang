import type { UserRole, User } from '@/types'

/** 从 localStorage 读取当前用户角色（非敏感信息） */
export function readStoredRole(): UserRole | null {
  try {
    const raw = localStorage.getItem('user')
    if (!raw) return null
    const parsed = JSON.parse(raw) as { role?: string }
    return (parsed.role as UserRole) || null
  } catch {
    return null
  }
}

/** 从 localStorage 读取当前用户信息 */
export function readStoredUser(): User | null {
  try {
    const raw = localStorage.getItem('user')
    if (!raw) return null
    return JSON.parse(raw) as User
  } catch {
    return null
  }
}

/**
 * 角色权限判定（**fail-closed**：role 为 null 时返回 false）。
 *
 * 后端 `authz` 对未知/缺失角色一律拒绝，前端必须同口径；旧实现在 role 为
 * null 时放行，篡改 localStorage 即可看到不该看到的入口。
 */
export function isRoleAllowed(role: UserRole | null, allowed: UserRole[]): boolean {
  if (role === null) return false
  return allowed.includes(role)
}

/** 令牌剩余有效期低于该阈值（1h）时主动续期 */
export const TOKEN_REFRESH_THRESHOLD_MS = 60 * 60 * 1000
/** 续期检查间隔（5min） */
export const TOKEN_REFRESH_CHECK_INTERVAL_MS = 5 * 60 * 1000
