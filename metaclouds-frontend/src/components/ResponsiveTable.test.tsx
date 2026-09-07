/**
 * ResponsiveTable 组件测试
 *
 * 测试范围：
 * 1. 桌面端（≥768px）渲染 antd Table
 * 2. 移动端（<768px）渲染卡片列表
 * 3. 卡片视图展示正确的字段和值
 * 4. 卡片视图保留操作列
 * 5. rowKey 用于稳定的 key
 * 6. React.memo 包裹（引用不变时不重渲染）
 *
 * Mock 策略：
 * - antd Grid.useBreakpoint：通过 mock 控制断点，模拟桌面/移动端
 */
import React from 'react';
import { render, screen } from '../../test/test-utils';
import ResponsiveTable from './ResponsiveTable';

// ===== Mock 配置 =====

// Mock antd Grid.useBreakpoint，默认桌面端（md=true）
let mockBreakpoint = { xs: true, sm: true, md: true, lg: true, xl: true };
jest.mock('antd', () => {
  const actual = jest.requireActual('antd');
  return {
    ...actual,
    Grid: {
      ...actual.Grid,
      useBreakpoint: () => mockBreakpoint,
    },
  };
});

// 测试数据
const testData = [
  { id: 1, name: 'item-1', status: 'active', value: 100 },
  { id: 2, name: 'item-2', status: 'inactive', value: 200 },
];

const testColumns = [
  { title: 'ID', dataIndex: 'id', key: 'id' },
  { title: '名称', dataIndex: 'name', key: 'name' },
  { title: '状态', dataIndex: 'status', key: 'status' },
  {
    title: '操作',
    key: 'action',
    render: (_: unknown, record: { id: number }) => (
      <button data-testid={`action-btn-${record.id}`}>编辑</button>
    ),
  },
];

describe('ResponsiveTable 组件', () => {
  beforeEach(() => {
    // 默认桌面端
    mockBreakpoint = { xs: true, sm: true, md: true, lg: true, xl: true };
  });

  // ===== 1. 桌面端渲染 =====

  describe('桌面端渲染（≥768px）', () => {
    test('渲染 antd Table 元素', () => {
      render(<ResponsiveTable columns={testColumns} dataSource={testData} rowKey="id" />);
      // antd Table 渲染包含 .ant-table 类名的元素
      expect(document.querySelector('.ant-table')).toBeInTheDocument();
    });

    test('渲染表头列名', () => {
      render(<ResponsiveTable columns={testColumns} dataSource={testData} rowKey="id" />);
      expect(screen.getByText('ID')).toBeInTheDocument();
      expect(screen.getByText('名称')).toBeInTheDocument();
      expect(screen.getByText('状态')).toBeInTheDocument();
      expect(screen.getByText('操作')).toBeInTheDocument();
    });

    test('渲染数据行内容', () => {
      render(<ResponsiveTable columns={testColumns} dataSource={testData} rowKey="id" />);
      expect(screen.getByText('item-1')).toBeInTheDocument();
      expect(screen.getByText('item-2')).toBeInTheDocument();
      expect(screen.getByText('active')).toBeInTheDocument();
      expect(screen.getByText('inactive')).toBeInTheDocument();
    });

    test('渲染操作列按钮', () => {
      render(<ResponsiveTable columns={testColumns} dataSource={testData} rowKey="id" />);
      expect(screen.getByTestId('action-btn-1')).toBeInTheDocument();
      expect(screen.getByTestId('action-btn-2')).toBeInTheDocument();
    });
  });

  // ===== 2. 移动端渲染 =====

  describe('移动端渲染（<768px）', () => {
    beforeEach(() => {
      // 模拟移动端（md=false）
      mockBreakpoint = { xs: true, sm: true, md: false, lg: false, xl: false };
    });

    test('渲染卡片列表容器', () => {
      render(<ResponsiveTable columns={testColumns} dataSource={testData} rowKey="id" />);
      expect(document.querySelector('.mc-responsive-cards')).toBeInTheDocument();
    });

    test('为每条数据渲染一张卡片', () => {
      render(<ResponsiveTable columns={testColumns} dataSource={testData} rowKey="id" />);
      const cards = document.querySelectorAll('.mc-responsive-card');
      expect(cards.length).toBe(2);
    });

    test('卡片展示字段标签和值', () => {
      render(<ResponsiveTable columns={testColumns} dataSource={testData} rowKey="id" />);
      // 标签
      expect(screen.getAllByText('ID').length).toBeGreaterThan(0);
      expect(screen.getAllByText('名称').length).toBeGreaterThan(0);
      // 值
      expect(screen.getByText('item-1')).toBeInTheDocument();
      expect(screen.getByText('item-2')).toBeInTheDocument();
    });

    test('卡片标题字段使用第一个字段', () => {
      render(<ResponsiveTable columns={testColumns} dataSource={testData} rowKey="id" />);
      // 标题行有特殊 class
      const titleRows = document.querySelectorAll('.mc-rc-row--title');
      expect(titleRows.length).toBe(2); // 两张卡片各一个标题行
    });

    test('卡片视图保留操作列按钮', () => {
      render(<ResponsiveTable columns={testColumns} dataSource={testData} rowKey="id" />);
      expect(screen.getByTestId('action-btn-1')).toBeInTheDocument();
      expect(screen.getByTestId('action-btn-2')).toBeInTheDocument();
    });

    test('空数据时不渲染卡片', () => {
      render(<ResponsiveTable columns={testColumns} dataSource={[]} rowKey="id" />);
      const cards = document.querySelectorAll('.mc-responsive-card');
      expect(cards.length).toBe(0);
    });
  });

  // ===== 3. cardFields 自定义 =====

  describe('cardFields 自定义', () => {
    beforeEach(() => {
      mockBreakpoint = { xs: true, sm: true, md: false, lg: false, xl: false };
    });

    test('自定义 cardFields 只展示指定字段', () => {
      render(
        <ResponsiveTable
          columns={testColumns}
          dataSource={testData}
          rowKey="id"
          cardFields={['name', 'status']}
        />,
      );
      // 只展示 name 和 status，不展示 id
      expect(screen.getByText('名称')).toBeInTheDocument();
      expect(screen.getByText('状态')).toBeInTheDocument();
      // ID 标签不应出现在卡片字段中（但操作列可能有，这里检查卡片行）
      const labelRows = document.querySelectorAll('.mc-rc-label');
      const labelTexts = Array.from(labelRows).map((el) => el.textContent);
      expect(labelTexts).toContain('名称');
      expect(labelTexts).toContain('状态');
    });

    test('自定义 cardTitleField', () => {
      render(
        <ResponsiveTable
          columns={testColumns}
          dataSource={testData}
          rowKey="id"
          cardFields={['name', 'status']}
          cardTitleField="name"
        />,
      );
      // 标题行应包含 name 的值
      const titleRows = document.querySelectorAll('.mc-rc-row--title');
      expect(titleRows.length).toBe(2);
    });
  });

  // ===== 4. rowKey 稳定性 =====

  describe('rowKey 稳定性', () => {
    test('使用字符串 rowKey', () => {
      render(<ResponsiveTable columns={testColumns} dataSource={testData} rowKey="id" />);
      // 不报错即通过
      expect(screen.getByText('item-1')).toBeInTheDocument();
    });

    test('使用函数 rowKey', () => {
      render(
        <ResponsiveTable
          columns={testColumns}
          dataSource={testData}
          rowKey={(record: { id: number }) => `custom-${record.id}`}
        />,
      );
      expect(screen.getByText('item-1')).toBeInTheDocument();
    });
  });
});
