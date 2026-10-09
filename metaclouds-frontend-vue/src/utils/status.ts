/**
 * 状态色 / 文案映射（纯函数，与具体页面解耦）。
 *
 * 抽取自 JobManagement / K8SManagement 中重复的 statusText / statusClass，
 * 保持原行为不变：
 *  - job*：作业状态（含 cancelled），CSS 用 .mc-status.cancelled 回退 base；
 *  - resource*：资源/节点状态（含 active/online/ready/available），
 *    online/ready/available/active 归并到 running，cancelled/未知回退 idle。
 *
 * 两套映射分别对齐两个页面既有的 CSS 修饰类（src/styles/index.css），
 * 不做合并以免改变既有渲染语义。
 */

export function jobStatusText(s?: string): string {
  const map: Record<string, string> = {
    pending: '排队中',
    running: '运行中',
    completed: '已完成',
    failed: '失败',
    cancelled: '已取消',
  }
  return map[s ?? ''] ?? s ?? '-'
}

export function jobStatusClass(s?: string): string {
  return ['running', 'pending', 'completed', 'failed', 'cancelled'].includes(s ?? '')
    ? (s as string)
    : 'idle'
}

export function resourceStatusText(s?: string): string {
  const map: Record<string, string> = {
    running: '运行中',
    pending: '排队中',
    completed: '已完成',
    failed: '失败',
    cancelled: '已取消',
    active: '在线',
    online: '在线',
    ready: '就绪',
    available: '可用',
  }
  return map[s ?? ''] ?? s ?? '-'
}

export function resourceStatusClass(s?: string): string {
  const runningSet = ['running', 'active', 'online', 'ready', 'available']
  if (runningSet.includes(s ?? '')) return 'running'
  if (s === 'pending') return 'pending'
  if (s === 'completed') return 'completed'
  if (s === 'failed' || s === 'error') return 'failed'
  return 'idle'
}
