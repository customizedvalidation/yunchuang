/**
 * src/nav.ts 单元测试
 *
 * 锁住「统一导航源（Single Source of Truth）」的核心不变量，防止后续改动
 * 无意中让 Sidebar 菜单 / Topbar 面包屑 / router 角色守卫三者再次分叉：
 *  - 14 个一级模块 + 嵌套子路由完整覆盖；
 *  - tenant 角色约束保持 ['admin','manager']；
 *  - 图标名集合与旧 Sidebar iconMap 一致；
 *  - getNavTitle 对嵌套路径返回正确叶级标题，未知路径回退 'Metaclouds'；
 *  - findNavItem 精确优先、前缀取最长父级，并继承父级角色约束。
 */
import { describe, it, expect } from 'vitest'
import {
  navConfig,
  findNavItem,
  getNavTitle,
  getNavRoles,
  type NavItem,
} from '@/nav'

/** 深度优先收集所有导航项（父 + 子），供内部一致性校验 */
function allItems(): NavItem[] {
  const acc: NavItem[] = []
  const walk = (items: NavItem[]) => {
    for (const it of items) {
      acc.push(it)
      if (it.children?.length) walk(it.children)
    }
  }
  navConfig.forEach((g) => walk(g.children))
  return acc
}

describe('nav.ts 统一导航源', () => {
  it('包含 14 个一级模块（dashboard/cluster/resource/job/gpus/partitions/schedulers/topology/acceleration/datasets/monitoring/tenant/security/k8s）', () => {
    const top = navConfig.flatMap((g) => g.children)
    expect(top).toHaveLength(14)
    expect(top.map((i) => i.key).sort()).toEqual(
      [
        '/acceleration', '/cluster', '/datasets', '/dashboard', '/gpus',
        '/job', '/k8s', '/monitoring', '/partitions', '/resource',
        '/schedulers', '/security', '/tenant', '/topology',
      ].sort(),
    )
  })

  it('覆盖嵌套子路由 /job/{list,queue,history} 与 /k8s/{nodes,pods,services}', () => {
    expect(findNavItem('/job')?.children?.map((c) => c.key)).toEqual([
      '/job/list', '/job/queue', '/job/history',
    ])
    expect(findNavItem('/k8s')?.children?.map((c) => c.key)).toEqual([
      '/k8s/nodes', '/k8s/pods', '/k8s/services',
    ])
  })

  it('tenant 角色约束保留为 admin/manager（与 router 守卫 / Sidebar 过滤同源）', () => {
    expect(getNavRoles('/tenant')).toEqual(['admin', 'manager'])
  })

  it('无角色约束的路由 getNavRoles 为 undefined（守卫不误拦普通模块）', () => {
    expect(getNavRoles('/dashboard')).toBeUndefined()
    expect(getNavRoles('/job')).toBeUndefined()
    expect(getNavRoles('/job/list')).toBeUndefined()
    expect(getNavRoles('/k8s/nodes')).toBeUndefined()
  })

  it('图标名集合与旧 Sidebar iconMap 一致', () => {
    const expectedIcons = [
      'Odometer', 'Tickets', 'Cloudy', 'Lightning', 'Grid',
      'Connection', 'Box', 'Share', 'House', 'Promotion',
      'FolderOpened', 'Bell', 'UserFilled', 'Lock',
    ]
    const used = allItems().map((i) => i.icon).filter(Boolean) as string[]
    for (const icon of expectedIcons) {
      expect(used).toContain(icon)
    }
  })

  it('getNavTitle 对嵌套路径返回正确叶级标题，未知路径回退 Metaclouds', () => {
    expect(getNavTitle('/dashboard')).toBe('仪表盘')
    expect(getNavTitle('/job')).toBe('作业管理')
    expect(getNavTitle('/job/queue')).toBe('任务队列')
    expect(getNavTitle('/job/history')).toBe('历史记录')
    expect(getNavTitle('/k8s/nodes')).toBe('节点管理')
    expect(getNavTitle('/k8s/services')).toBe('服务管理')
    expect(getNavTitle('/tenant')).toBe('多租户管理')
    expect(getNavTitle('/no-such-page')).toBe('Metaclouds')
  })

  it('findNavItem 精确匹配优先，前缀匹配取最长父级', () => {
    // 精确匹配
    expect(findNavItem('/job/list')?.key).toBe('/job/list')
    // 前缀：更长父级优先（避免 /job 误匹配 /job/list 子树）
    expect(findNavItem('/job/list/extra')?.key).toBe('/job/list')
  })

  it('findNavItem 前缀匹配继承父级角色约束（tenant 子路径仍受限）', () => {
    expect(getNavRoles('/tenant/settings')).toEqual(['admin', 'manager'])
  })
})
