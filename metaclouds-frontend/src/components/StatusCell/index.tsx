import React from 'react';
import { statusColor, statusColorFg, statusText } from '../../theme/tokens';

/** antd mc-status 识别的状态类，配合 index.css 提供色点 + 脉冲动画 */
const KNOWN = ['running', 'pending', 'completed', 'failed', 'idle'];

/**
 * 统一状态单元格：色点 + 文字，禁止仅用颜色表意。
 * 已知状态走 CSS 类（带_running 脉冲），未知状态回退到内联语义色，文案始终展示。
 *
 * React.memo：纯展示组件，props 不变时跳过重渲染。表格中每行都有 StatusCell，
 * 父组件重渲染时可避免数百个状态单元格无谓重绘。
 */
const StatusCell: React.FC<{ status: string }> = React.memo(({ status }) => {
  const known = KNOWN.includes(status);
  // 文字用 -fg 可读版（原色作文字仅 ~2.9:1）；色点非文字，仍用原色
  const color = statusColorFg[status] ?? 'var(--mc-text-3)';
  const dot = statusColor[status] ?? 'var(--mc-text-3)';
  const text = statusText[status] ?? status;
  return (
    <span className={`mc-status ${known ? status : 'idle'}`} style={known ? undefined : { color }}>
      <i className="mc-status-dot" style={known ? undefined : { background: dot }} />
      {text}
    </span>
  );
});

export default StatusCell;
