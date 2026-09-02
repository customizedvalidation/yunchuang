import React from 'react';
import { Table, Card, Grid } from 'antd';
import type { TableProps } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import './ResponsiveTable.css';

type Row = Record<string, any>;

export interface ResponsiveTableProps<T extends Row> extends TableProps<T> {
  /** 卡片视图展示的字段（columns 的 dataIndex）；缺省取前 3 个含 dataIndex 的列 */
  cardFields?: (keyof T | string)[];
  /** 卡片标题字段；缺省取 cardFields[0] */
  cardTitleField?: keyof T | string;
}

/**
 * 响应式表格（PRD R6）：≥768px 用 antd Table（scroll.x 改为 max-content，去掉硬编码宽度）；
 * <768px 自动切换为卡片列表，避免横向滚动条（G1 最大失分点）。
 *
 * 断点判定用 antd Grid.useBreakpoint()，不引入任何 window.innerWidth / resize 监听（G3）。
 * 卡片视图同构于同一份 columns：含 dataIndex 的列作为「标签-值」行，无 dataIndex 的渲染列（操作列）
 * 作为卡片底部操作区，因此移动端也能保留关键操作（如作业取消）。
 */
const ResponsiveTable = <T extends Row>(props: ResponsiveTableProps<T>) => {
  const { cardFields, cardTitleField, columns, dataSource, scroll, ...rest } = props;
  const screens = Grid.useBreakpoint();
  const isCard = !screens.md;

  if (isCard) {
    const cols = (columns as ColumnsType<T>) ?? [];
    const dataCols = cols.filter((c) => (c as any).dataIndex != null) as any[];
    const actionCols = cols.filter(
      (c) => (c as any).dataIndex == null && typeof (c as any).render === 'function',
    ) as any[];
    const fields: string[] =
      (cardFields as string[] | undefined) ?? dataCols.slice(0, 3).map((c) => String(c.dataIndex));
    const titleField = (cardTitleField as string | undefined) ?? fields[0];
    const colByKey = new Map(dataCols.map((c) => [String(c.dataIndex), c]));
    const rows = (dataSource as T[]) ?? [];

    return (
      <div className="mc-responsive-cards">
        {rows.map((row, i) => {
          const rk = rest.rowKey;
          const key =
            typeof rk === 'function'
              ? (rk as any)(row)
              : rk != null
                ? (row as any)[rk as string]
                : i;
          return (
            <Card key={key} size="small" className="mc-responsive-card">
              {fields.map((f) => {
                const col = colByKey.get(f);
                const label = typeof col?.title === 'string' ? col.title : f;
                const value = col?.render ? col.render((row as any)[f], row, i) : (row as any)[f];
                const isTitle = f === titleField;
                return (
                  <div className={`mc-rc-row${isTitle ? ' mc-rc-row--title' : ''}`} key={f}>
                    <span className="mc-rc-label">{label}</span>
                    <span className="mc-rc-value">{value ?? '-'}</span>
                  </div>
                );
              })}
              {actionCols.length > 0 && (
                <div className="mc-rc-actions">
                  {actionCols.map((col, ci) => (
                    <React.Fragment key={ci}>{col.render?.(undefined, row, i)}</React.Fragment>
                  ))}
                </div>
              )}
            </Card>
          );
        })}
      </div>
    );
  }

  return (
    <Table<T>
      columns={columns}
      dataSource={dataSource}
      scroll={{ ...(scroll as object), x: 'max-content' }}
      {...rest}
    />
  );
};

export default ResponsiveTable;
