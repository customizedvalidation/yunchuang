import React, { useCallback, useMemo, useState, useEffect } from 'react';
import { Layout, Menu, Tooltip, Badge } from 'antd';
import type { MenuProps } from 'antd';

const { Sider } = Layout;
import {
  DashboardOutlined,
  CloudOutlined,
  TagsOutlined,
  BellOutlined,
  ContainerOutlined,
  LeftOutlined,
  LogoutOutlined,
  ThunderboltOutlined,
  AppstoreOutlined,
  DeploymentUnitOutlined,
  ApiOutlined,
  ApartmentOutlined,
  RocketOutlined,
  DatabaseOutlined,
  TeamOutlined,
  SafetyCertificateOutlined,
} from '@ant-design/icons';
import { useNavigate, useLocation } from 'react-router-dom';
import { semantic, brand } from '../../theme/tokens';
import { useGetJobsQuery, useGetClustersQuery, useGetResourcesQuery, useGetTenantsQuery, useGetAlertsQuery, useGetGPUDevicesQuery, useGetPartitionsQuery, useGetSchedulerIntegrationsQuery, useGetDatasetsQuery, csrfHeaders } from '../../store/api';
import { extractArrayData } from '../../utils/api';
import { isRoleAllowed, readStoredRole } from '../../utils/auth';
import type { Job, MenuItem, UserRole } from '../../types';
import { SIDER_WIDTH } from '../../theme/breakpoints';
import './Sidebar.css';

/** antd Menu 接受的单项类型（判别联合） */
type AntdMenuItem = NonNullable<Required<MenuProps>['items']>[number];

/**
 * 按角色过滤菜单树；role 为 null 时原样返回（fail-open，理由见 isRoleAllowed）。
 *
 * 递归规则：叶子自身不满足 roles 则剔除；group 在其子节点被全部剔除后整体移除，
 * 避免留下没有内容的分组标题。
 */
function filterMenuByRole(items: MenuItem[], role: UserRole | null): MenuItem[] {
  if (role === null) {
    return items;
  }

  const result: MenuItem[] = [];
  for (const item of items) {
    if (item.roles && !isRoleAllowed(role, item.roles)) {
      continue;
    }
    if (item.children?.length) {
      const children = filterMenuByRole(item.children, role);
      if (children.length === 0) {
        continue;
      }
      result.push({ ...item, children });
      continue;
    }
    result.push(item);
  }
  return result;
}

// 菜单按 skill.md 5 层架构分组并补全全部 15 个页面：
// 总览 → 作业调度(容器层) → 基础资源(基础资源层) → 集群管理(集群管理层)
// → 加速套件(加速套件层) → 平台治理 → 可观测与安全(监控管理安全层)
export const menuItems: MenuItem[] = [
  {
    type: 'group',
    label: '总览',
    children: [
      { key: '/dashboard', icon: <DashboardOutlined />, label: '仪表盘', description: '算力总览' },
    ],
  },
  {
    type: 'group',
    label: '作业调度',
    children: [
      {
        key: '/job', icon: <TagsOutlined />, label: '作业管理', description: '作业调度',
        children: [
          { key: '/job/list', label: '作业列表' },
          { key: '/job/queue', label: '任务队列' },
          { key: '/job/history', label: '历史记录' },
        ],
      },
    ],
  },
  {
    type: 'group',
    label: '基础资源',
    children: [
      { key: '/resource', icon: <CloudOutlined />, label: '资源管理', description: '资源分配' },
      { key: '/gpus', icon: <ThunderboltOutlined />, label: 'GPU 设备', description: '细粒度分配' },
      { key: '/partitions', icon: <AppstoreOutlined />, label: '分区管理', description: '分区配额' },
    ],
  },
  {
    type: 'group',
    label: '集群管理',
    children: [
      { key: '/cluster', icon: <DeploymentUnitOutlined />, label: '集群管理', description: '高可用集群' },
      {
        key: '/k8s', icon: <ContainerOutlined />, label: 'K8S 管理', description: '容器编排',
        children: [
          { key: '/k8s/nodes', label: '节点管理' },
          { key: '/k8s/pods', label: 'Pod 管理' },
          { key: '/k8s/services', label: '服务管理' },
        ],
      },
      { key: '/schedulers', icon: <ApiOutlined />, label: '调度器', description: 'Slurm/LSF' },
      { key: '/topology', icon: <ApartmentOutlined />, label: '拓扑感知', description: '网络拓扑' },
    ],
  },
  {
    type: 'group',
    label: '加速套件',
    children: [
      { key: '/acceleration', icon: <RocketOutlined />, label: '加速套件', description: '推理加速' },
      { key: '/datasets', icon: <DatabaseOutlined />, label: '数据集', description: 'Fluid 加速' },
    ],
  },
  {
    type: 'group',
    label: '平台治理',
    children: [
      // 对齐后端 authz：tenant:read 仅授予 admin 与 manager，user 点击会被 403
      { key: '/tenant', icon: <TeamOutlined />, label: '多租户', description: '租户配额', roles: ['admin', 'manager'] },
    ],
  },
  {
    type: 'group',
    label: '可观测与安全',
    children: [
      { key: '/monitoring', icon: <BellOutlined />, label: '监控告警', description: '实时监控' },
      { key: '/security', icon: <SafetyCertificateOutlined />, label: '安全管理', description: '安全策略' },
    ],
  },
];

export interface SidebarProps {
  collapsed: boolean;
  onCollapse: (collapsed: boolean) => void;
  /** <1024 时浮层(Drawer)是否打开；打开时强制展开菜单（即使桌面态折叠） */
  mobileOpen?: boolean;
}

const Sidebar: React.FC<SidebarProps> = ({ collapsed, onCollapse, mobileOpen }) => {
  const navigate = useNavigate();
  const location = useLocation();

  const handleMenuClick = useCallback((e: { key: string }) => {
    navigate(e.key);
  }, [navigate]);

  const logout = useCallback(() => {
    // 清除后端 httpOnly access_token Cookie（幂等），再清理本地非敏感缓存。
    fetch('/api/v1/auth/logout', {
      method: 'POST',
      credentials: 'include',
      headers: { 'Content-Type': 'application/json', ...csrfHeaders() },
    })
      .catch(() => undefined)
      .finally(() => {
        localStorage.removeItem('user');
        localStorage.removeItem('auth_expiry');
        navigate('/login');
      });
  }, [navigate]);

  // 当前角色只读取一次：登录/退出都会整页重挂载，无需做成响应式状态
  const currentRole = useMemo(() => readStoredRole(), []);
  // 过滤后的菜单树，后续父级 key 计算与渲染都基于它（含 badgeMap 的键空间不变）
  const visibleMenuItems = useMemo(() => filterMenuByRole(menuItems, currentRole), [currentRole]);

  // 递归查找当前路由所属的所有父级子菜单 key（需穿透 group 分组层）
  const getParentKeysForPath = useCallback((pathname: string): string[] => {
    const find = (items: MenuItem[], trail: string[]): string[] => {
      let res: string[] = [];
      for (const item of items) {
        if (item.type === 'group') {
          res = res.concat(find(item.children || [], trail));
        } else if (item.children && item.key) {
          // 无 key 的父节点无法成为路由目标，跳过其 trail 计算
          const newTrail = [...trail, item.key];
          if (pathname === item.key || pathname.startsWith(item.key + '/')) {
            res = res.concat(newTrail);
          }
          res = res.concat(find(item.children, newTrail));
        }
      }
      return res;
    };
    return find(visibleMenuItems, []);
  }, [visibleMenuItems]);

  // 受控展开：默认展开当前路由所属父菜单，并允许用户手动展开/收起子菜单
  const [openKeys, setOpenKeys] = useState<string[]>(() => getParentKeysForPath(location.pathname));
  useEffect(() => {
    setOpenKeys((prev) => Array.from(new Set([...prev, ...getParentKeysForPath(location.pathname)])));
  }, [location.pathname, getParentKeysForPath]);

  // 菜单项实时数量徽标：全部基于既有 API，无需新增后端端点
  const { data: badgeJobs } = useGetJobsQuery(undefined);
  const { data: badgeClusters } = useGetClustersQuery(undefined);
  const { data: badgeResources } = useGetResourcesQuery(undefined);
  const { data: badgeTenants } = useGetTenantsQuery(undefined);
  const { data: badgeGPUDevices } = useGetGPUDevicesQuery({});
  const { data: badgePartitions } = useGetPartitionsQuery({});
  const { data: badgeSchedulers } = useGetSchedulerIntegrationsQuery();
  const { data: badgeDatasets } = useGetDatasetsQuery();
  const { data: badgeAlerts } = useGetAlertsQuery(undefined);
  const badgeJobsData = extractArrayData<Job>(badgeJobs);
  const badgeMap: Record<string, number> = {
    '/job': badgeJobsData.length,
    '/job/queue': badgeJobsData.filter((j) => j.status === 'pending').length,
    '/job/history': badgeJobsData.filter((j) => ['completed', 'failed', 'cancelled'].includes(j.status)).length,
    '/k8s/pods': badgeJobsData.filter((j) => j.status === 'running').length,
    '/cluster': extractArrayData(badgeClusters).length,
    '/resource': extractArrayData(badgeResources).length,
    '/tenant': extractArrayData(badgeTenants).length,
    '/monitoring': extractArrayData(badgeAlerts).length,
    '/gpus': extractArrayData(badgeGPUDevices).length,
    '/partitions': extractArrayData(badgePartitions).length,
    '/schedulers': extractArrayData(badgeSchedulers).length,
    '/datasets': extractArrayData(badgeDatasets).length,
  };

  // 徽标颜色分级：按业务语义与数量给出不同警示色，避免一律红色
  // - 监控告警：>0 橙色（warning），>=5 红色升级（danger）
  // - 任务队列(pending)/运行 Pod：进行中蓝 / 绿色（正常态）
  // - 总览类计数（作业/集群/资源/租户/历史）：中性蓝或灰
  const getBadgeColor = (key: string, count: number): string | undefined => {
    if (!count) return undefined;
    switch (key) {
      case '/monitoring':
        return count >= 5 ? semantic.danger : semantic.warning;
      case '/job/queue':
        return brand[500];
      case '/k8s/pods':
        return semantic.success;
      case '/job/history':
        return semantic.muted;
      case '/job':
      case '/cluster':
      case '/resource':
      case '/tenant':
      case '/gpus':
      case '/partitions':
      case '/schedulers':
      case '/datasets':
        return brand[500];
      default:
        return semantic.danger;
    }
  };

  // 递归处理所有菜单项，包括分组、分隔线与子菜单
  const processMenuItem = (item: MenuItem): AntdMenuItem => {
    // 分隔线：直接透传
    if (item.type === 'divider') {
      return { type: 'divider' };
    }
    // 分组标题：保留纯文本标签并递归处理子项（分组本身不加徽标）
    if (item.type === 'group') {
      return {
        type: 'group',
        label: item.label,
        children: (item.children || []).map((child) => processMenuItem(child)),
      };
    }

    // 无 key 的节点（理论上不存在，类型上 key 可选）取空串，badgeMap 查不到即无徽标
    const itemKey = item.key ?? '';
    const count = badgeMap[itemKey];
    const badge = count ? (
      <Badge count={count} size="small" color={getBadgeColor(itemKey, count)} style={{ marginLeft: 'auto' }} />
    ) : null;
    const labelNode = collapsed ? (
      <div className="menu-item-content collapsed">
        <div className="menu-label-row">
          <span className="menu-label">{item.label}</span>
          {badge}
        </div>
      </div>
    ) : (
      <div className="menu-item-content">
        <div className="menu-label-row">
          <span className="menu-label">{item.label}</span>
          {badge}
        </div>
        {item.description && (
          <span className="menu-description">{item.description}</span>
        )}
      </div>
    );
    // antd 的 ItemType 是判别联合，TS 无法从可选字段推导出具体分支，
    // 因此在此边界处做一次收敛断言；上游 menuItems 本身已是强类型。
    if (item.children?.length) {
      return {
        ...item,
        label: labelNode,
        children: item.children.map((child) => processMenuItem(child)),
      } as AntdMenuItem;
    }
    return { ...item, label: labelNode } as AntdMenuItem;
  };

  const processedItems = visibleMenuItems.map((item) => processMenuItem(item));

  // 浮层打开时强制展开完整菜单（移动端抽屉内不需要图标轨）
  const menuCollapsed = mobileOpen ? false : collapsed;

  return (
    <Sider
      className={`metaclouds-sidebar ${collapsed ? 'collapsed' : ''} ${mobileOpen ? 'mobile-open' : ''}`}
      width={mobileOpen ? SIDER_WIDTH.drawer : SIDER_WIDTH.expanded}
      collapsed={menuCollapsed}
      collapsedWidth={SIDER_WIDTH.collapsed}
      trigger={null}
    >
      <div className="logo">
        {!collapsed && (
          <>
            {/* 品牌 logo 不参与文档大纲，用 div 而非 h1（页面标题才是唯一的 h1） */}
            <div className="logo-title">Metaclouds</div>
            <span className="logo-subtitle">算力调度平台</span>
          </>
        )}
      </div>

      <button
        type="button"
        className="collapse-btn"
        onClick={() => onCollapse(!collapsed)}
        aria-label={collapsed ? '展开侧边栏' : '折叠侧边栏'}
      >
        <LeftOutlined />
      </button>

      <div className="menu-container">
        <Menu
          mode="inline"
          theme="dark"
          items={processedItems}
          selectedKeys={[location.pathname]}
          openKeys={openKeys}
          onOpenChange={(keys) => setOpenKeys(keys as string[])}
          onClick={handleMenuClick}
          aria-label="主导航"
        />
      </div>

      <Tooltip
        title={collapsed ? '退出登录' : ''}
        placement="right"
      >
        <button
          type="button"
          className="logout-btn"
          onClick={logout}
          aria-label="退出登录"
        >
          <LogoutOutlined />
          {!collapsed && <span>退出登录</span>}
        </button>
      </Tooltip>
    </Sider>
  );
};

export default Sidebar;
