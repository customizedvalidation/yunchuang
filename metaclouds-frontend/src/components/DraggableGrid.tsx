import React, { useState } from 'react';
import './DraggableGrid.css';

export interface GridItem {
  id: string;
  /** 12 栅格制下的列宽 */
  span: 3 | 4 | 6 | 8 | 12;
  title: string;
  node: React.ReactNode;
}

export interface DraggableGridProps {
  /** 已按当前顺序排列的卡片 */
  items: GridItem[];
  /** 是否进入自定义布局模式（可拖拽） */
  editable: boolean;
  hiddenIds?: string[];
  onReorder: (ids: string[]) => void;
  /** 键盘可达的顺序调整（兼顾无障碍） */
  onMove?: (id: string, dir: -1 | 1) => void;
  onToggleHidden?: (id: string) => void;
}

/**
 * 可拖拽栅格（原生 HTML5 Drag & Drop，零第三方依赖）
 *
 * 设计要点：
 * - 仅在 drop 时重排序，避免拖拽过程中移动 DOM 导致 Chrome 中断拖拽。
 * - 除鼠标拖拽外，提供「前移 / 后移」按钮，保证键盘与读屏用户同样可调整顺序。
 */
const DraggableGrid: React.FC<DraggableGridProps> = ({
  items,
  editable,
  hiddenIds = [],
  onReorder,
  onMove,
  onToggleHidden,
}) => {
  const [dragId, setDragId] = useState<string | null>(null);
  const [overId, setOverId] = useState<string | null>(null);

  const handleDragStart = (e: React.DragEvent<HTMLDivElement>, id: string) => {
    if (!editable) return;
    setDragId(id);
    e.dataTransfer.effectAllowed = 'move';
    // Firefox 需要设置数据才会触发拖拽
    e.dataTransfer.setData('text/plain', id);
  };

  const handleDragOver = (e: React.DragEvent<HTMLDivElement>, id: string) => {
    if (!editable || !dragId) return;
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    if (overId !== id) setOverId(id);
  };

  const handleDrop = (e: React.DragEvent<HTMLDivElement>, targetId: string) => {
    e.preventDefault();
    const sourceId = dragId ?? e.dataTransfer.getData('text/plain');
    setDragId(null);
    setOverId(null);
    if (!sourceId || sourceId === targetId) return;

    const from = items.findIndex((i) => i.id === sourceId);
    const to = items.findIndex((i) => i.id === targetId);
    if (from < 0 || to < 0) return;

    const next = [...items];
    const [moved] = next.splice(from, 1);
    next.splice(to, 0, moved);
    onReorder(next.map((i) => i.id));
  };

  const handleDragEnd = () => {
    setDragId(null);
    setOverId(null);
  };

  return (
    <div className={`dg${editable ? ' dg-editable' : ''}`}>
      {items.map((item, index) => {
        const isHidden = hiddenIds.includes(item.id);
        const classes = [
          'dg-item',
          `dg-span-${item.span}`,
          dragId === item.id ? 'dg-dragging' : '',
          overId === item.id && dragId !== item.id ? 'dg-over' : '',
          isHidden ? 'dg-hidden' : '',
        ]
          .filter(Boolean)
          .join(' ');

        return (
          <div
            key={item.id}
            className={classes}
            draggable={editable}
            onDragStart={(e) => handleDragStart(e, item.id)}
            onDragOver={(e) => handleDragOver(e, item.id)}
            onDrop={(e) => handleDrop(e, item.id)}
            onDragEnd={handleDragEnd}
            aria-label={item.title}
          >
            {editable && (
              <div className="dg-toolbar">
                <span className="dg-handle" title="拖拽以调整顺序" aria-hidden="true">
                  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <circle cx="9" cy="6" r="1.4" />
                    <circle cx="15" cy="6" r="1.4" />
                    <circle cx="9" cy="12" r="1.4" />
                    <circle cx="15" cy="12" r="1.4" />
                    <circle cx="9" cy="18" r="1.4" />
                    <circle cx="15" cy="18" r="1.4" />
                  </svg>
                </span>
                <span className="dg-toolbar-title">{item.title}</span>
                <span className="dg-toolbar-actions">
                  <button
                    type="button"
                    className="dg-btn"
                    aria-label={`将「${item.title}」前移`}
                    disabled={index === 0}
                    onClick={() => onMove?.(item.id, -1)}
                  >
                    ←
                  </button>
                  <button
                    type="button"
                    className="dg-btn"
                    aria-label={`将「${item.title}」后移`}
                    disabled={index === items.length - 1}
                    onClick={() => onMove?.(item.id, 1)}
                  >
                    →
                  </button>
                  <button
                    type="button"
                    className="dg-btn"
                    aria-label={isHidden ? `显示「${item.title}」` : `隐藏「${item.title}」`}
                    onClick={() => onToggleHidden?.(item.id)}
                  >
                    {isHidden ? '显示' : '隐藏'}
                  </button>
                </span>
              </div>
            )}
            <div className="dg-body">{item.node}</div>
          </div>
        );
      })}
    </div>
  );
};

export default DraggableGrid;
