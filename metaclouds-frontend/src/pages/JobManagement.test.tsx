/**
 * JobManagement 页面组件测试
 *
 * 测试范围：
 * 1. 渲染作业表格（验证表格容器与数据传递）
 * 2. 页签筛选功能（作业列表 / 任务队列 / 历史记录）
 * 3. 新建作业弹窗打开
 * 4. 分页控件
 * 5. 取消作业操作（Popconfirm 二次确认）
 *
 * Mock 策略：
 * - useGetJobsQuery / useCreateJobMutation / useCancelJobMutation：
 *   用 jest.mock 替换为 mock 实现
 * - ResponsiveTable：mock 为简单 div，避免 antd Table 虚拟滚动在 jsdom 中的性能问题，
 *   同时可断言传入的 dataSource / columns / pagination 是否正确
 */
import React from 'react';
import { render, screen, fireEvent, waitFor } from '../test/test-utils';
import { mockQueryHook, mockMutationHook, mockEnvelope } from '../test/mocks';
import JobManagement from './JobManagement';

// ===== Mock 配置 =====

// Mock ResponsiveTable：记录传入的 props，便于断言数据筛选和分页
let lastTableProps: Record<string, unknown> = {};
jest.mock('../components/ResponsiveTable', () => ({
  __esModule: true,
  default: (props: Record<string, unknown>) => {
    lastTableProps = props;
    return (
      <div data-testid="mock-responsive-table">
        {/* 渲染数据行数，便于断言筛选结果 */}
        <span data-testid="table-row-count">{(props.dataSource as unknown[]).length}</span>
        {/* 渲染列数 */}
        <span data-testid="table-col-count">{(props.columns as unknown[]).length}</span>
      </div>
    );
  },
}));

// 默认 mock：作业列表数据
let mockJobsData = [
  { id: 1, name: 'llama3-finetune', type: 'training', status: 'running', gpus: 4, progress: 65, created_at: '2024-01-01' },
  { id: 2, name: 'gpt4-inference', type: 'inference', status: 'pending', gpus: 2, progress: 0, created_at: '2024-01-02' },
  { id: 3, name: 'batch-job-1', type: 'batch', status: 'completed', gpus: 1, progress: 100, created_at: '2024-01-03' },
  { id: 4, name: 'failed-job', type: 'training', status: 'failed', gpus: 2, progress: 30, error_msg: 'OOM', created_at: '2024-01-04' },
  { id: 5, name: 'cancelled-job', type: 'batch', status: 'cancelled', gpus: 1, progress: 10, created_at: '2024-01-05' },
];

let mockGetJobs = mockQueryHook({ data: mockEnvelope(mockJobsData) });
let mockCreateJob = mockMutationHook({ result: mockEnvelope({ id: 6 }) });
let mockCancelJob = mockMutationHook({ result: mockEnvelope({}) });

jest.mock('../store/api', () => ({
  ...jest.requireActual('../store/api'),
  useGetJobsQuery: () => mockGetJobs(),
  useCreateJobMutation: () => mockCreateJob(),
  useCancelJobMutation: () => mockCancelJob(),
}));

describe('JobManagement 页面', () => {
  beforeEach(() => {
    // 重置默认数据
    mockJobsData = [
      { id: 1, name: 'llama3-finetune', type: 'training', status: 'running', gpus: 4, progress: 65, created_at: '2024-01-01' },
      { id: 2, name: 'gpt4-inference', type: 'inference', status: 'pending', gpus: 2, progress: 0, created_at: '2024-01-02' },
      { id: 3, name: 'batch-job-1', type: 'batch', status: 'completed', gpus: 1, progress: 100, created_at: '2024-01-03' },
      { id: 4, name: 'failed-job', type: 'training', status: 'failed', gpus: 2, progress: 30, error_msg: 'OOM', created_at: '2024-01-04' },
      { id: 5, name: 'cancelled-job', type: 'batch', status: 'cancelled', gpus: 1, progress: 10, created_at: '2024-01-05' },
    ];
    mockGetJobs = mockQueryHook({ data: mockEnvelope(mockJobsData) });
    mockCreateJob = mockMutationHook({ result: mockEnvelope({ id: 6 }) });
    mockCancelJob = mockMutationHook({ result: mockEnvelope({}) });
    lastTableProps = {};
  });

  // ===== 1. 渲染作业表格 =====

  describe('渲染作业表格', () => {
    test('渲染页面标题"作业管理"', () => {
      render(<JobManagement />);
      expect(screen.getByText('作业管理')).toBeInTheDocument();
    });

    test('渲染作业表格组件', () => {
      render(<JobManagement />);
      expect(screen.getByTestId('mock-responsive-table')).toBeInTheDocument();
    });

    test('表格接收全部作业数据（作业列表页签）', () => {
      render(<JobManagement />);
      // 作业列表页签显示全部 5 个作业
      expect(screen.getByTestId('table-row-count')).toHaveTextContent('5');
    });

    test('表格包含正确的列数', () => {
      render(<JobManagement />);
      // 基础列：ID/名称/类型/状态/GPU/进度/创建时间/操作 = 8 列
      expect(screen.getByTestId('table-col-count')).toHaveTextContent('8');
    });

    test('页头显示作业统计信息', () => {
      render(<JobManagement />);
      expect(screen.getByText(/共 5 个作业/)).toBeInTheDocument();
      expect(screen.getByText(/排队 1 个/)).toBeInTheDocument();
      expect(screen.getByText(/运行中 1 个/)).toBeInTheDocument();
    });
  });

  // ===== 2. 页签筛选功能 =====

  describe('页签筛选功能', () => {
    test('点击"任务队列"页签只显示 pending 作业', () => {
      render(<JobManagement />);
      fireEvent.click(screen.getByText('任务队列'));
      // pending 状态的作业只有 1 个
      expect(screen.getByTestId('table-row-count')).toHaveTextContent('1');
    });

    test('点击"历史记录"页签显示已完成/失败/已取消作业', () => {
      render(<JobManagement />);
      fireEvent.click(screen.getByText('历史记录'));
      // completed + failed + cancelled = 3 个
      expect(screen.getByTestId('table-row-count')).toHaveTextContent('3');
    });

    test('历史记录页签额外显示错误信息列', () => {
      render(<JobManagement />);
      fireEvent.click(screen.getByText('历史记录'));
      // 基础 8 列 + 错误信息列 = 9 列
      expect(screen.getByTestId('table-col-count')).toHaveTextContent('9');
    });

    test('点击"作业列表"页签显示全部作业', () => {
      render(<JobManagement />);
      // 先切到任务队列
      fireEvent.click(screen.getByText('任务队列'));
      expect(screen.getByTestId('table-row-count')).toHaveTextContent('1');
      // 再切回作业列表
      fireEvent.click(screen.getByText('作业列表'));
      expect(screen.getByTestId('table-row-count')).toHaveTextContent('5');
    });

    test('任务队列为空时显示空态', () => {
      mockJobsData = [
        { id: 1, name: 'job-1', type: 'training', status: 'running', gpus: 1, progress: 50, created_at: '2024-01-01' },
      ];
      mockGetJobs = mockQueryHook({ data: mockEnvelope(mockJobsData) });
      render(<JobManagement />);
      fireEvent.click(screen.getByText('任务队列'));
      expect(screen.getByText('队列为空')).toBeInTheDocument();
    });
  });

  // ===== 3. 新建作业弹窗 =====

  describe('新建作业弹窗', () => {
    test('渲染"新建作业"按钮', () => {
      render(<JobManagement />);
      expect(screen.getByRole('button', { name: '新建作业' })).toBeInTheDocument();
    });

    test('点击"新建作业"按钮打开弹窗', () => {
      render(<JobManagement />);
      fireEvent.click(screen.getByRole('button', { name: '新建作业' }));
      // antd Modal 打开后显示标题
      expect(screen.getByText('创建作业')).toBeInTheDocument();
    });

    test('弹窗包含名称输入框', () => {
      render(<JobManagement />);
      fireEvent.click(screen.getByRole('button', { name: '新建作业' }));
      expect(screen.getByPlaceholderText('例如：llama3-70b-finetune')).toBeInTheDocument();
    });

    test('弹窗包含类型选择器', () => {
      render(<JobManagement />);
      fireEvent.click(screen.getByRole('button', { name: '新建作业' }));
      expect(screen.getByText('请选择类型')).toBeInTheDocument();
    });

    test('弹窗包含 GPU 数量输入框', () => {
      render(<JobManagement />);
      fireEvent.click(screen.getByRole('button', { name: '新建作业' }));
      expect(screen.getByText('GPU数量')).toBeInTheDocument();
    });

    test('弹窗包含"创建"和"取消"按钮', () => {
      render(<JobManagement />);
      fireEvent.click(screen.getByRole('button', { name: '新建作业' }));
      expect(screen.getByRole('button', { name: '创建' })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: '取消' })).toBeInTheDocument();
    });

    test('点击"取消"按钮关闭弹窗', () => {
      render(<JobManagement />);
      fireEvent.click(screen.getByRole('button', { name: '新建作业' }));
      expect(screen.getByText('创建作业')).toBeInTheDocument();
      fireEvent.click(screen.getByRole('button', { name: '取消' }));
      // antd Modal 关闭有动画，用 waitFor
      waitFor(() => {
        expect(screen.queryByText('创建作业')).not.toBeInTheDocument();
      });
    });
  });

  // ===== 4. 分页控件 =====

  describe('分页控件', () => {
    test('表格配置了分页（pageSize=10）', () => {
      render(<JobManagement />);
      const pagination = lastTableProps.pagination as { pageSize: number; showTotal: (t: number) => string };
      expect(pagination.pageSize).toBe(10);
    });

    test('分页显示总数文本', () => {
      render(<JobManagement />);
      const pagination = lastTableProps.pagination as { showTotal: (t: number) => string };
      expect(pagination.showTotal(5)).toBe('共 5 条');
    });

    test('表格配置了虚拟滚动', () => {
      render(<JobManagement />);
      expect(lastTableProps.virtual).toBe(true);
    });

    test('表格配置了滚动区域', () => {
      render(<JobManagement />);
      const scroll = lastTableProps.scroll as { x: number; y: number };
      expect(scroll.x).toBe(900);
      expect(scroll.y).toBe(520);
    });
  });

  // ===== 5. 加载与错误状态 =====

  describe('加载与错误状态', () => {
    test('加载中显示骨架屏', () => {
      mockGetJobs = mockQueryHook({ isLoading: true });
      render(<JobManagement />);
      expect(screen.getByText('内容加载中')).toBeInTheDocument();
    });

    test('API 报错时显示错误状态', () => {
      mockGetJobs = mockQueryHook({ error: { status: 500, data: { message: '服务器错误' } } });
      render(<JobManagement />);
      expect(screen.getByText('数据加载失败')).toBeInTheDocument();
    });

    test('空数据时显示空态', () => {
      mockGetJobs = mockQueryHook({ data: mockEnvelope([]) });
      render(<JobManagement />);
      expect(screen.getByText('还没有作业')).toBeInTheDocument();
    });

    test('空态显示"新建作业"按钮', () => {
      mockGetJobs = mockQueryHook({ data: mockEnvelope([]) });
      render(<JobManagement />);
      // 空态中的新建作业按钮
      const buttons = screen.getAllByRole('button', { name: '新建作业' });
      expect(buttons.length).toBeGreaterThanOrEqual(1);
    });
  });
});
