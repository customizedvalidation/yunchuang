import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useNavigate } from 'react-router-dom';
import { useGetJobsQuery } from '../../store/api';
import { extractArrayData } from '../../utils/api';
import { useThemeMode } from '../../theme/ThemeModeContext';
import { statusText } from '../../theme/tokens';
import './CommandPalette.css';

interface CommandItem {
  key: string;
  group: string;
  title: string;
  hint?: string;
  run: () => void;
}

interface CommandPaletteContextValue {
  isOpen: boolean;
  open: () => void;
  close: () => void;
  toggle: () => void;
}

const CommandPaletteContext = createContext<CommandPaletteContextValue | null>(null);

const PAGES: { path: string; title: string; group: string; hint: string }[] = [
  { path: '/dashboard', title: '仪表盘', group: '跳转到', hint: '总览' },
  { path: '/job/list', title: '作业列表', group: '跳转到', hint: '调度' },
  { path: '/job/queue', title: '任务队列', group: '跳转到', hint: '调度' },
  { path: '/job/history', title: '历史记录', group: '跳转到', hint: '调度' },
  { path: '/cluster', title: '集群管理', group: '跳转到', hint: '基础设施' },
  { path: '/resource', title: '资源管理', group: '跳转到', hint: '基础设施' },
  { path: '/k8s/nodes', title: '节点管理', group: '跳转到', hint: '基础设施' },
  { path: '/k8s/pods', title: 'Pod 管理', group: '跳转到', hint: '基础设施' },
  { path: '/k8s/services', title: '服务管理', group: '跳转到', hint: '基础设施' },
  { path: '/tenant', title: '多租户管理', group: '跳转到', hint: '平台' },
  { path: '/acceleration', title: '加速套件', group: '跳转到', hint: '平台' },
  { path: '/monitoring', title: '监控告警', group: '跳转到', hint: '可观测与治理' },
  { path: '/security', title: '安全管理', group: '跳转到', hint: '可观测与治理' },
];

export const CommandPaletteProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [isOpen, setIsOpen] = useState(false);
  const open = useCallback(() => setIsOpen(true), []);
  const close = useCallback(() => setIsOpen(false), []);
  const toggle = useCallback(() => setIsOpen((v) => !v), []);

  const value = useMemo<CommandPaletteContextValue>(
    () => ({ isOpen, open, close, toggle }),
    [isOpen, open, close, toggle],
  );

  // 全局快捷键：Ctrl / ⌘ + K 唤起，Esc 关闭
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        toggle();
      } else if (e.key === 'Escape' && isOpen) {
        close();
      }
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [toggle, close, isOpen]);

  return (
    <CommandPaletteContext.Provider value={value}>
      {children}
      <CommandPalette />
    </CommandPaletteContext.Provider>
  );
};

export const useCommandPalette = (): CommandPaletteContextValue => {
  const ctx = useContext(CommandPaletteContext);
  if (!ctx) throw new Error('useCommandPalette 必须在 CommandPaletteProvider 内使用');
  return ctx;
};

const CommandPalette: React.FC = () => {
  const { isOpen, close } = useCommandPalette();
  const navigate = useNavigate();
  const { isDark, toggle: toggleTheme } = useThemeMode();
  const { data: jobs } = useGetJobsQuery(undefined);
  const jobsData = extractArrayData(jobs);

  const [query, setQuery] = useState('');
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const go = useCallback(
    (path: string) => {
      navigate(path);
      close();
    },
    [navigate, close],
  );

  const items = useMemo<CommandItem[]>(() => {
    const actions: CommandItem[] = [
      {
        key: 'action-new-job',
        group: '快捷操作',
        title: '新建作业',
        hint: 'N',
        run: () => go('/job/list'),
      },
      {
        key: 'action-queue',
        group: '快捷操作',
        title: '查看排队中的作业',
        hint: `${jobsData.filter((j: { status?: string }) => j.status === 'pending').length} 个`,
        run: () => go('/job/queue'),
      },
      {
        key: 'action-alerts',
        group: '快捷操作',
        title: '查看监控告警',
        hint: '可观测',
        run: () => go('/monitoring'),
      },
      {
        key: 'action-theme',
        group: '快捷操作',
        title: isDark ? '切换到浅色模式' : '切换到深色模式',
        hint: 'T',
        run: () => {
          toggleTheme();
          close();
        },
      },
    ];

    const pageItems: CommandItem[] = PAGES.map((p) => ({
      key: `page-${p.path}`,
      group: p.group,
      title: p.title,
      hint: p.hint,
      run: () => go(p.path),
    }));

    const jobItems: CommandItem[] = jobsData.slice(0, 8).map((j: Record<string, unknown>) => ({
      key: `job-${String(j.id ?? j.job_id ?? Math.random())}`,
      group: '作业',
      title: `${String(j.id ?? j.job_id ?? '')} · ${String(j.name ?? '未命名作业')}`,
      hint: statusText[String(j.status ?? '')] ?? String(j.status ?? ''),
      run: () => go('/job/list'),
    }));

    return [...actions, ...pageItems, ...jobItems];
  }, [go, isDark, toggleTheme, close, jobsData]);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return items;
    return items.filter(
      (it) => it.title.toLowerCase().includes(q) || (it.hint ?? '').toLowerCase().includes(q),
    );
  }, [items, query]);

  // 分组展示（保持原有顺序分组）
  const grouped = useMemo(() => {
    const map = new Map<string, CommandItem[]>();
    filtered.forEach((it) => {
      const arr = map.get(it.group) ?? [];
      arr.push(it);
      map.set(it.group, arr);
    });
    return Array.from(map.entries());
  }, [filtered]);

  // 扁平顺序（用于键盘上下移动）
  const flat = useMemo(() => grouped.flatMap(([, list]) => list), [grouped]);

  useEffect(() => {
    setActiveIndex(0);
  }, [query, isOpen]);

  useEffect(() => {
    if (!isOpen) {
      setQuery('');
      return;
    }
    const t = window.setTimeout(() => inputRef.current?.focus(), 40);
    return () => window.clearTimeout(t);
  }, [isOpen]);

  const onKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setActiveIndex((i) => (flat.length ? (i + 1) % flat.length : 0));
      } else if (e.key === 'ArrowUp') {
        e.preventDefault();
        setActiveIndex((i) => (flat.length ? (i - 1 + flat.length) % flat.length : 0));
      } else if (e.key === 'Enter') {
        e.preventDefault();
        const item = flat[activeIndex];
        if (item) item.run();
      } else if (e.key === 'Escape') {
        e.preventDefault();
        close();
      }
    },
    [flat, activeIndex, close],
  );

  // 让选中项始终可见
  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>('.cmdk-item.active');
    el?.scrollIntoView({ block: 'nearest' });
  }, [activeIndex]);

  if (!isOpen) return null;

  let cursor = -1;

  return (
    <div className="cmdk-mask" onMouseDown={close} role="presentation">
      <div
        className="cmdk-box"
        onMouseDown={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label="命令面板"
      >
        <div className="cmdk-input-row">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" aria-hidden="true">
            <circle cx="11" cy="11" r="8" />
            <path d="m21 21-4.3-4.3" strokeLinecap="round" />
          </svg>
          <input
            ref={inputRef}
            className="cmdk-input"
            placeholder="搜索页面、作业 ID，或输入命令…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onKeyDown}
            aria-label="命令面板搜索"
          />
          <kbd className="cmdk-kbd">Esc</kbd>
        </div>

        <div className="cmdk-list" ref={listRef}>
          {flat.length === 0 && <div className="cmdk-empty">没有匹配的结果</div>}
          {grouped.map(([group, list]) => (
            <div key={group} className="cmdk-group">
              <div className="cmdk-group-title">{group}</div>
              {list.map((item) => {
                cursor += 1;
                const index = cursor;
                return (
                  <div
                    key={item.key}
                    className={`cmdk-item${index === activeIndex ? ' active' : ''}`}
                    onMouseEnter={() => setActiveIndex(index)}
                    onClick={item.run}
                    role="option"
                    aria-selected={index === activeIndex}
                  >
                    <span className="cmdk-item-title">{item.title}</span>
                    {item.hint && <span className="cmdk-item-hint">{item.hint}</span>}
                  </div>
                );
              })}
            </div>
          ))}
        </div>

        <div className="cmdk-foot">
          <span>
            <kbd className="cmdk-kbd">↑</kbd>
            <kbd className="cmdk-kbd">↓</kbd> 选择
          </span>
          <span>
            <kbd className="cmdk-kbd">↵</kbd> 执行
          </span>
          <span>
            <kbd className="cmdk-kbd">Esc</kbd> 关闭
          </span>
        </div>
      </div>
    </div>
  );
};
