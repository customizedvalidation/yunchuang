import React from 'react';
import { statusColor, statusText } from '../../theme/tokens';

/** antd mc-status 识别的状态类，配合 index.css 提供色点 + 脉冲动画 */
const KNOWN = ['running', 'pending', 'completed', 'failed', 'idle'];

/**
 * 统一状态单元格：色点 + 文字，禁止仅用颜色表意。
 * 已知状态走 CSS 类（带_running 脉冲），未知状态回退到内联语义色，文案始终展示。
 */
const StatusCell: React.FC<{ status: string }> = ({ status }) => {
  const known = KNOWN.includes(status);
  const color = statusColor[status] ?? '#7C8DA6';
  const text = statusText[status] ?? status;
  return (
    <span className={`mc-status ${known ? status : 'idle'}`} style={known ? undefined : { color }}>
      <i className="mc-status-dot" style={known ? undefined : { background: color }} />
      {text}
    </span>
  );
};

export default StatusCell;
