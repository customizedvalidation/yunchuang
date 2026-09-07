/**
 * States 组件测试
 *
 * 测试范围：
 * 1. EmptyState — 空状态渲染（标题、描述、操作按钮）
 * 2. ErrorState — 错误状态渲染（标题、描述、重试按钮）
 * 3. TableSkeleton — 表格骨架屏（行数、列数、aria 属性）
 * 4. renderState — 三态守卫（错误 > 加载 > 空 > null）
 *
 * 这些是纯展示组件，无外部依赖，测试用户可见行为。
 */
import React from 'react';
import { render, screen, fireEvent } from '../../test/test-utils';
import { EmptyState, ErrorState, TableSkeleton, renderState } from './States';

describe('States 组件', () => {
  // ===== 1. EmptyState =====

  describe('EmptyState 空状态', () => {
    test('渲染默认标题"暂无数据"', () => {
      render(<EmptyState />);
      expect(screen.getByText('暂无数据')).toBeInTheDocument();
    });

    test('渲染默认描述文本', () => {
      render(<EmptyState />);
      expect(screen.getByText(/当前筛选条件下没有匹配的结果/)).toBeInTheDocument();
    });

    test('渲染自定义标题', () => {
      render(<EmptyState title="还没有作业" />);
      expect(screen.getByText('还没有作业')).toBeInTheDocument();
    });

    test('渲染自定义描述', () => {
      render(<EmptyState description="提交第一个作业后就可以在这里跟踪状态" />);
      expect(screen.getByText('提交第一个作业后就可以在这里跟踪状态')).toBeInTheDocument();
    });

    test('渲染操作按钮', () => {
      const handleClick = jest.fn();
      render(
        <EmptyState
          title="还没有作业"
          action={
            <button onClick={handleClick}>新建作业</button>
          }
        />,
      );
      const btn = screen.getByRole('button', { name: '新建作业' });
      expect(btn).toBeInTheDocument();
      fireEvent.click(btn);
      expect(handleClick).toHaveBeenCalledTimes(1);
    });

    test('不渲染操作按钮区域（未传 action）', () => {
      render(<EmptyState />);
      expect(screen.queryByRole('button')).not.toBeInTheDocument();
    });

    test('设置 role="status" 用于无障碍', () => {
      render(<EmptyState />);
      expect(screen.getByRole('status')).toBeInTheDocument();
    });
  });

  // ===== 2. ErrorState =====

  describe('ErrorState 错误状态', () => {
    test('渲染默认标题"数据加载失败"', () => {
      render(<ErrorState />);
      expect(screen.getByText('数据加载失败')).toBeInTheDocument();
    });

    test('渲染默认描述文本', () => {
      render(<ErrorState />);
      expect(screen.getByText(/请求未能完成/)).toBeInTheDocument();
    });

    test('渲染自定义标题', () => {
      render(<ErrorState title="仪表盘数据加载失败" />);
      expect(screen.getByText('仪表盘数据加载失败')).toBeInTheDocument();
    });

    test('渲染自定义描述', () => {
      render(<ErrorState description="部分接口请求失败，可尝试重新加载" />);
      expect(screen.getByText('部分接口请求失败，可尝试重新加载')).toBeInTheDocument();
    });

    test('渲染重试按钮', () => {
      const handleRetry = jest.fn();
      render(<ErrorState onRetry={handleRetry} />);
      const btn = screen.getByRole('button', { name: '重试' });
      expect(btn).toBeInTheDocument();
      fireEvent.click(btn);
      expect(handleRetry).toHaveBeenCalledTimes(1);
    });

    test('未传 onRetry 时不显示重试按钮', () => {
      render(<ErrorState />);
      expect(screen.queryByRole('button', { name: '重试' })).not.toBeInTheDocument();
    });

    test('设置 role="alert" 用于无障碍', () => {
      render(<ErrorState />);
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
  });

  // ===== 3. TableSkeleton =====

  describe('TableSkeleton 表格骨架屏', () => {
    test('渲染默认 5 行 6 列', () => {
      render(<TableSkeleton />);
      // 表头 1 行 + 5 数据行 = 6 行容器
      const rows = document.querySelectorAll('.mc-skeleton-row');
      expect(rows.length).toBe(5);
      // 每行 6 个骨架条
      const barsInFirstRow = rows[0]?.querySelectorAll('.mc-skeleton-bar');
      expect(barsInFirstRow?.length).toBe(6);
    });

    test('渲染自定义行数', () => {
      render(<TableSkeleton rows={10} />);
      const rows = document.querySelectorAll('.mc-skeleton-row');
      expect(rows.length).toBe(10);
    });

    test('渲染自定义列数', () => {
      render(<TableSkeleton columns={4} />);
      const rows = document.querySelectorAll('.mc-skeleton-row');
      const barsInFirstRow = rows[0]?.querySelectorAll('.mc-skeleton-bar');
      expect(barsInFirstRow?.length).toBe(4);
    });

    test('设置 aria-busy="true"', () => {
      render(<TableSkeleton />);
      const skeleton = document.querySelector('.mc-skeleton');
      expect(skeleton).toHaveAttribute('aria-busy', 'true');
    });

    test('设置 aria-live="polite"', () => {
      render(<TableSkeleton />);
      const skeleton = document.querySelector('.mc-skeleton');
      expect(skeleton).toHaveAttribute('aria-live', 'polite');
    });

    test('包含屏幕阅读器文本"内容加载中"', () => {
      render(<TableSkeleton />);
      expect(screen.getByText('内容加载中')).toBeInTheDocument();
    });
  });

  // ===== 4. renderState 三态守卫 =====

  describe('renderState 三态守卫', () => {
    test('error 优先返回 ErrorState', () => {
      const result = renderState({
        isLoading: true,
        error: new Error('fail'),
        isEmpty: true,
      });
      render(<>{result}</>);
      // 错误状态优先于加载和空态
      expect(screen.getByRole('alert')).toBeInTheDocument();
      expect(screen.queryByText('内容加载中')).not.toBeInTheDocument();
    });

    test('isLoading 返回 TableSkeleton', () => {
      const result = renderState({
        isLoading: true,
        error: undefined,
        isEmpty: true,
      });
      render(<>{result}</>);
      expect(screen.getByText('内容加载中')).toBeInTheDocument();
    });

    test('isEmpty 返回空态（默认 EmptyState）', () => {
      const result = renderState({
        isLoading: false,
        error: undefined,
        isEmpty: true,
      });
      render(<>{result}</>);
      expect(screen.getByText('暂无数据')).toBeInTheDocument();
    });

    test('isEmpty 时使用自定义 empty 节点', () => {
      const result = renderState({
        isLoading: false,
        error: undefined,
        isEmpty: true,
        empty: <div data-testid="custom-empty">自定义空态</div>,
      });
      render(<>{result}</>);
      expect(screen.getByTestId('custom-empty')).toBeInTheDocument();
    });

    test('正常状态返回 null', () => {
      const result = renderState({
        isLoading: false,
        error: undefined,
        isEmpty: false,
      });
      expect(result).toBeNull();
    });

    test('ErrorState 携带 onRetry 回调', () => {
      const handleRetry = jest.fn();
      const result = renderState({
        error: new Error('fail'),
        onRetry: handleRetry,
      });
      render(<>{result}</>);
      fireEvent.click(screen.getByRole('button', { name: '重试' }));
      expect(handleRetry).toHaveBeenCalledTimes(1);
    });

    test('TableSkeleton 使用自定义 skeletonRows 和 skeletonColumns', () => {
      const result = renderState({
        isLoading: true,
        skeletonRows: 3,
        skeletonColumns: 4,
      });
      render(<>{result}</>);
      const rows = document.querySelectorAll('.mc-skeleton-row');
      expect(rows.length).toBe(3);
    });
  });
});
