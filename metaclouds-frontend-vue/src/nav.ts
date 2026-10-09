/**
 * 统一导航配置源（Single Source of Truth）。
 *
 * 收口此前散落在三处的导航配置：
 *  - Sidebar.vue 的 menuItems（菜单结构 + 图标名 + 角色约束）
 *  - Topbar.vue 的 titleMap（面包屑标题）
 *  - router/index.ts 的 meta.roles（角色守卫）
 *
 * 任何菜单 / 标题 / 角色变更只需改这里：
 *  - Sidebar  由 navConfig 生成菜单（图标名 → 组件映射在 Sidebar 内保留）；
 *  - Topbar   由 getNavTitle(path) 生成面包屑标题；
 *  - router   守卫由 getNavRoles(path) 取角色约束。
 *
 * 与 router path 严格对齐：14 个一级模块 + 嵌套子路由
 * （/job/{list,queue,history}、/k8s/{nodes,pods,services}）完全保留。
 */
import type { UserRole } from '@/types'

/** 单个导航项（菜单项 / 面包屑项通用） */
export interface NavItem {
  /** 路由路径；同时作为菜单 key 与 router-link 目标（与 router path 完全一致） */
  key: string
  /** 菜单 / 面包屑显示标题 */
  label: string
  /** Element Plus 图标组件名（字符串），Sidebar 内 iconMap 映射为组件 */
  icon?: string
  /** 菜单项描述（副标题） */
  description?: string
  /** 可见角色；缺省 = 全部可见（仅 tenant 含 admin/manager 约束） */
  roles?: UserRole[]
  /** 子菜单项 */
  children?: NavItem[]
}

/** 菜单分组 */
export interface NavGroup {
  label: string
  children: NavItem[]
}

export const navConfig: NavGroup[] = [
  {
    label: '总览',
    children: [
      { key: '/dashboard', icon: 'Odometer', label: '仪表盘', description: '算力总览' },
    ],
  },
  {
    label: '作业调度',
    children: [
      {
        key: '/job', icon: 'Tickets', label: '作业管理', description: '作业调度',
        children: [
          { key: '/job/list', label: '作业列表' },
          { key: '/job/queue', label: '任务队列' },
          { key: '/job/history', label: '历史记录' },
        ],
      },
    ],
  },
  {
    label: '基础资源',
    children: [
      { key: '/resource', icon: 'Cloudy', label: '资源管理', description: '资源分配' },
      { key: '/gpus', icon: 'Lightning', label: 'GPU 设备', description: '细粒度分配' },
      { key: '/partitions', icon: 'Grid', label: '分区管理', description: '分区配额' },
    ],
  },
  {
    label: '集群管理',
    children: [
      { key: '/cluster', icon: 'Connection', label: '集群管理', description: '高可用集群' },
      {
        key: '/k8s', icon: 'Box', label: 'K8S 管理', description: '容器编排',
        children: [
          { key: '/k8s/nodes', label: '节点管理' },
          { key: '/k8s/pods', label: 'Pod 管理' },
          { key: '/k8s/services', label: '服务管理' },
        ],
      },
      { key: '/schedulers', icon: 'Share', label: '调度器', description: 'Slurm/LSF' },
      { key: '/topology', icon: 'House', label: '拓扑感知', description: '网络拓扑' },
    ],
  },
  {
    label: '加速套件',
    children: [
      { key: '/acceleration', icon: 'Promotion', label: '加速套件', description: '推理加速' },
      { key: '/datasets', icon: 'FolderOpened', label: '数据集', description: 'Fluid 加速' },
    ],
  },
  {
    label: '系统治理',
    children: [
      { key: '/monitoring', icon: 'Bell', label: '监控告警', description: '实时监控' },
      { key: '/tenant', icon: 'UserFilled', label: '多租户管理', description: '租户配额', roles: ['admin', 'manager'] },
      { key: '/security', icon: 'Lock', label: '安全管理', description: '安全策略' },
    ],
  },
]

/** 深度优先扁平化所有导航项（父 + 子），供查询使用 */
function flattenNav(items: NavItem[], acc: NavItem[] = []): NavItem[] {
  for (const it of items) {
    acc.push(it)
    if (it.children?.length) flattenNav(it.children, acc)
  }
  return acc
}

const allNavItems: NavItem[] = flattenNav(navConfig.flatMap((g) => g.children))

/**
 * 按路径取最匹配的导航项：
 *  1) 精确匹配 key === path（含子项）；
 *  2) 否则取 path 以 item.key + '/' 开头的“父级”项（取最长前缀，
 *     避免 '/job' 误匹配其它以 /job 开头的路径）。
 */
export function findNavItem(path: string): NavItem | undefined {
  const exact = allNavItems.find((i) => i.key === path)
  if (exact) return exact
  const parent = allNavItems
    .filter((i) => path.startsWith(i.key + '/'))
    .sort((a, b) => b.key.length - a.key.length)[0]
  return parent
}

/** 面包屑标题：取最匹配项的 label；无匹配回退 'Metaclouds' */
export function getNavTitle(path: string): string {
  return findNavItem(path)?.label ?? 'Metaclouds'
}

/** 角色约束：取最匹配项（含父级）声明的 roles；无则 undefined（全部可见） */
export function getNavRoles(path: string): UserRole[] | undefined {
  return findNavItem(path)?.roles
}
