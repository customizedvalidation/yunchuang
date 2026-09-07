/**
 * Dashboard 页面组件测试
 *
 * 测试范围：
 * 1. 渲染统计卡片（集群数量、GPU 利用率、运行中作业、活跃告警）
 * 2. 图表容器渲染（mock ResponsiveChart，验证图表区域存在）
 * 3. Loading 状态显示 Skeleton
 * 4. 错误状态显示 ErrorState
 * 5. 空数据状态显示 EmptyState
 *
 * Mock 策略：
 * - useGetClustersQuery / useGetResourcesQuery / useGetJobsQuery / useGetAlertsQuery：
 *   用 jest.mock 替换为 mockQueryHook
 * - ResponsiveChart：mock 为简单 div，避免真实 ECharts 渲染（jsdom 不支持 canvas）
 * - DraggableGrid：保留真实实现（纯布局组件，无外部依赖）
 */
import React from 'react';
import { render, screen, waitFor } from '../test/test-utils';
import { mockQueryHook, mockEnvelope, mockLoadingQuery, mockErrorQuery } from '../test/mocks';
import Dashboard from './Dashboard';

// ===== Mock 配置 =====

// Mock ResponsiveChart：避免真实 ECharts 渲染，返回带 testid 的 div
jest.mock('../components/ResponsiveChart', () => ({
  __esModule: true,
  default: ({ option, size, className }: { option: unknown; size?: string; className?: string }) => (
    <div data-testid="mock-responsive-chart" data-size={size} className={className}>
      {/* 渲染 option 的 JSON 长度，便于断言图表有数据 */}
      <span data-testid="chart-option-hash">{JSON.stringify(option).length}</span>
    </div>
  ),
}));

// Mock DraggableGrid：避免复杂的拖拽逻辑，直接渲染子项
jest.mock('../components/DraggableGrid', () => ({
  __esModule: true,
  default: ({ items, editable, hiddenIds, onReorder, onMove, onToggleHidden }: {
    items: Array<{ id: string; node: React.ReactNode }>;
    editable?: boolean;
    hiddenIds?: string[];
    onReorder?: (ids: string[]) => void;
    onMove?: (id: string, dir: -1 | 1) => void;
    onToggleHidden?: (id: string) => void;
  }) => (
    <div data-testid="mock-draggable-grid">
      {items.map((item) => (
        <div key={item.id} data-testid={`grid-item-${item.id}`}>
          {item.node}
        </div>
      ))}
    </div>
  ),
}));

// 默认 mock：全部 API 返回空数据
let mockClusters = mockQueryHook({ data: mockEnvelope([]) });
let mockResources = mockQueryHook({ data: mockEnvelope([]) });
let mockJobs = mockQueryHook({ data: mockEnvelope([]) });
let mockAlerts = mockQueryHook({ data: mockEnvelope([]) });

jest.mock('../store/api', () => ({
  ...jest.requireActual('../store/api'),
  useGetClustersQuery: () => mockClusters(),
  useGetResourcesQuery: () => mockResources(),
  useGetJobsQuery: () => mockJobs(),
  useGetAlertsQuery: () => mockAlerts(),
}));

describe('Dashboard 页面', () => {
  beforeEach(() => {
    // 重置为默认空数据
    mockClusters = mockQueryHook({ data: mockEnvelope([]) });
    mockResources = mockQueryHook({ data: mockEnvelope([]) });
    mockJobs = mockQueryHook({ data: mockEnvelope([]) });
    mockAlerts = mockQueryHook({ data: mockEnvelope([]) });
  });

  // ===== 1. 统计卡片渲染 =====

  describe('统计卡片渲染', () => {
    test('渲染"集群数量"卡片标题', () => {
      render(<Dashboard />);
      expect(screen.getByText('集群数量')).toBeInTheDocument();
    });

    test('渲染"GPU 利用率"卡片标题', () => {
      render(<Dashboard />);
      expect(screen.getByText('GPU 利用率')).toBeInTheDocument();
    });

    test('渲染"运行中作业"卡片标题', () => {
      render(<Dashboard />);
      expect(screen.getByText('运行中作业')).toBeInTheDocument();
    });

    test('渲染"活跃告警"卡片标题', () => {
      render(<Dashboard />);
      expect(screen.getByText('活跃告警')).toBeInTheDocument();
    });

    test('有数据时显示正确的集群数量', () => {
      mockClusters = mockQueryHook({
        data: mockEnvelope([
          { id: 1, name: 'cluster-a', status: 'active' },
          { id: 2, name: 'cluster-b', status: 'active' },
          { id: 3, name: 'cluster-c', status: 'maintenance' },
        ]),
      });
      render(<Dashboard />);
      // antd Statistic 渲染数字，用文本匹配
      expect(screen.getByText('3')).toBeInTheDocument();
    });

    test('有运行中作业时显示正确数量', () => {
      mockJobs = mockQueryHook({
        data: mockEnvelope([
          { id: 1, name: 'job-1', status: 'running' },
          { id: 2, name: 'job-2', status: 'running' },
          { id: 3, name: 'job-3', status: 'pending' },
          { id: 4, name: 'job-4', status: 'completed' },
        ]),
      });
      render(<Dashboard />);
      // 运行中作业数 = 2
      expect(screen.getByText('2')).toBeInTheDocument();
    });

    test('有活跃告警时显示正确数量', () => {
      mockAlerts = mockQueryHook({
        data: mockEnvelope([
          { id: 1, level: 'critical', message: 'GPU 过热' },
          { id: 2, level: 'warning', message: '磁盘空间不足' },
        ]),
      });
      render(<Dashboard />);
      expect(screen.getByText('2')).toBeInTheDocument();
    });

    test('GPU 利用率根据资源数据计算', () => {
      mockResources = mockQueryHook({
        data: mockEnvelope([
          { id: 1, type: 'gpu', name: 'gpu-1', total: 8, used: 4, status: 'active' },
          { id: 2, type: 'gpu', name: 'gpu-2', total: 8, used: 2, status: 'active' },
        ]),
      });
      render(<Dashboard />);
      // 利用率 = (4+2)/(8+8) * 100 = 37.5 ≈ 38
      expect(screen.getByText('38')).toBeInTheDocument();
    });
  });

  // ===== 2. 图表容器渲染 =====

  describe('图表容器渲染', () => {
    test('渲染"资源分布"图表卡片', () => {
      // 提供资源数据，使图表正常渲染（而非空态）
      mockResources = mockQueryHook({
        data: mockEnvelope([{ id: 1, type: 'gpu', name: 'gpu-1', total: 8, used: 4, status: 'active' }]),
      });
      render(<Dashboard />);
      expect(screen.getByText('资源分布')).toBeInTheDocument();
    });

    test('渲染"作业状态分布"图表卡片', () => {
      mockJobs = mockQueryHook({
        data: mockEnvelope([{ id: 1, name: 'job-1', status: 'running' }]),
      });
      render(<Dashboard />);
      expect(screen.getByText('作业状态分布')).toBeInTheDocument();
    });

    test('图表组件被渲染（mock ResponsiveChart）', () => {
      mockResources = mockQueryHook({
        data: mockEnvelope([{ id: 1, type: 'gpu', total: 8, used: 4 }]),
      });
      mockJobs = mockQueryHook({
        data: mockEnvelope([{ id: 1, status: 'running' }]),
      });
      render(<Dashboard />);
      // 两个图表都应被渲染
      const charts = screen.getAllByTestId('mock-responsive-chart');
      expect(charts.length).toBeGreaterThanOrEqual(2);
    });

    test('无资源数据时显示空态而非图表', () => {
      mockResources = mockQueryHook({ data: mockEnvelope([]) });
      render(<Dashboard />);
      expect(screen.getByText('暂无资源数据')).toBeInTheDocument();
    });

    test('无作业数据时显示空态而非图表', () => {
      mockJobs = mockQueryHook({ data: mockEnvelope([]) });
      render(<Dashboard />);
      expect(screen.getByText('还没有作业')).toBeInTheDocument();
    });
  });

  // ===== 3. Loading 状态 =====

  describe('Loading 状态', () => {
    test('数据加载中显示 Skeleton', () => {
      mockClusters = mockLoadingQuery();
      mockResources = mockLoadingQuery();
      mockJobs = mockLoadingQuery();
      mockAlerts = mockLoadingQuery();
      render(<Dashboard />);
      // antd Skeleton 渲染 aria-busy="true" 的元素
      expect(screen.getByText('内容加载中')).toBeInTheDocument();
    });

    test('加载中不显示统计数字', () => {
      mockClusters = mockLoadingQuery();
      mockResources = mockLoadingQuery();
      mockJobs = mockLoadingQuery();
      mockAlerts = mockLoadingQuery();
      render(<Dashboard />);
      // 加载中不应显示具体数字（Skeleton 占位）
      expect(screen.queryByText('0')).not.toBeInTheDocument();
    });
  });

  // ===== 4. 错误状态 =====

  describe('错误状态', () => {
    test('API 报错时显示错误状态', () => {
      mockClusters = mockErrorQuery('集群接口异常');
      render(<Dashboard />);
      expect(screen.getByText('仪表盘数据加载失败')).toBeInTheDocument();
    });

    test('错误状态显示重试按钮', () => {
      mockClusters = mockErrorQuery('集群接口异常');
      render(<Dashboard />);
      expect(screen.getByRole('button', { name: '重试' })).toBeInTheDocument();
    });
  });

  // ===== 5. 页面标题与操作 =====

  describe('页面标题与操作', () => {
    test('渲染页面标题"算力总览"', () => {
      render(<Dashboard />);
      expect(screen.getByText('算力总览')).toBeInTheDocument();
    });

    test('渲染"刷新"按钮', () => {
      render(<Dashboard />);
      expect(screen.getByRole('button', { name: '刷新' })).toBeInTheDocument();
    });

    test('渲染"自定义布局"按钮', () => {
      render(<Dashboard />);
      expect(screen.getByRole('button', { name: '自定义布局' })).toBeInTheDocument();
    });

    test('渲染时间范围选择器', () => {
      render(<Dashboard />);
      expect(screen.getByText('近 7 天')).toBeInTheDocument();
    });
  });
});
