import React from 'react';
import { Button } from 'antd';
import './States.css';

export interface EmptyStateProps {
  title?: string;
  description?: string;
  action?: React.ReactNode;
}

/** 空状态：必须给出下一步动作，避免用户卡住 */
export const EmptyState: React.FC<EmptyStateProps> = ({
  title = '暂无数据',
  description = '当前筛选条件下没有匹配的结果，试着放宽条件或清空搜索关键词。',
  action,
}) => (
  <div className="mc-empty" role="status">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden="true">
      <path d="M3 7h18v13H3z" strokeLinejoin="round" />
      <path d="M3 7l2-4h14l2 4" strokeLinejoin="round" />
      <path d="M9 12h6" strokeLinecap="round" />
    </svg>
    <div className="mc-empty-title">{title}</div>
    {description && <div className="mc-empty-desc">{description}</div>}
    {action && <div className="mc-empty-action">{action}</div>}
  </div>
);

export interface ErrorStateProps {
  title?: string;
  description?: string;
  onRetry?: () => void;
}

/** 错误状态：说明发生了什么 + 可操作的恢复入口 */
export const ErrorState: React.FC<ErrorStateProps> = ({
  title = '数据加载失败',
  description = '请求未能完成，请检查网络后重试。若持续失败请联系平台管理员。',
  onRetry,
}) => (
  <div className="mc-state-error" role="alert">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true">
      <circle cx="12" cy="12" r="9" />
      <path d="M12 8v5" strokeLinecap="round" />
      <path d="M12 16h.01" strokeLinecap="round" />
    </svg>
    <div className="mc-state-error-body">
      <div className="mc-state-error-title">{title}</div>
      <div className="mc-state-error-desc">{description}</div>
    </div>
    {onRetry && (
      <Button type="primary" size="small" onClick={onRetry}>
        重试
      </Button>
    )}
  </div>
);

export interface TableSkeletonProps {
  /** 骨架行数，默认 5 */
  rows?: number;
  /** 骨架列数，默认 6 */
  columns?: number;
}

/** 表格骨架屏：首屏用骨架屏（≥3 行），不用整页 spinner */
export const TableSkeleton: React.FC<TableSkeletonProps> = ({ rows = 5, columns = 6 }) => (
  <div className="mc-skeleton" aria-busy="true" aria-live="polite">
    <div className="mc-skeleton-head">
      {Array.from({ length: columns }).map((_, i) => (
        <span key={i} className="mc-skeleton-bar" style={{ width: i === 0 ? '18%' : '13%' }} />
      ))}
    </div>
    {Array.from({ length: rows }).map((_, r) => (
      <div className="mc-skeleton-row" key={r}>
        {Array.from({ length: columns }).map((_, c) => (
          <span
            key={c}
            className="mc-skeleton-bar"
            style={{ width: c === 0 ? '18%' : `${60 + ((r * 7 + c * 11) % 30)}%` }}
          />
        ))}
      </div>
    ))}
    <span className="mc-sr-only">内容加载中</span>
  </div>
);

export interface StateGuardProps {
  isLoading?: boolean;
  error?: unknown;
  /** 数据为空时显示空态；不传则不判断空态 */
  isEmpty?: boolean;
  onRetry?: () => void;
  empty?: React.ReactNode;
  skeletonRows?: number;
  skeletonColumns?: number;
}

/**
 * 表格三态守卫：按 错误 → 加载 → 空 的优先级返回对应占位内容。
 * 返回 null 表示数据正常，调用方应渲染真实表格。
 *
 * 用法：
 *   const state = renderState({ ... });
 *   if (state) return state;   // 在 Table 外层直接返回
 */
export const renderState = ({
  isLoading,
  error,
  isEmpty,
  onRetry,
  empty,
  skeletonRows,
  skeletonColumns,
}: StateGuardProps): React.ReactNode | null => {
  if (error) return <ErrorState onRetry={onRetry} />;
  if (isLoading) return <TableSkeleton rows={skeletonRows} columns={skeletonColumns} />;
  if (isEmpty) return <>{empty ?? <EmptyState />}</>;
  return null;
};
