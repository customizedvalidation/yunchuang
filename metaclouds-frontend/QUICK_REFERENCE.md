# 🚀 Dashboard 测试快速参考

## 📁 文件位置

| 文件 | 路径 | 描述 |
|------|------|------|
| 测试文件 | `src/pages/Dashboard.test.tsx` | 32 个测试用例 |
| 源代码 | `src/pages/Dashboard.tsx` | 被测试的组件 |
| Jest 配置 | `jest.config.js` | Jest 测试配置 |

## 🔧 快速开始

### 1. 安装依赖（如果尚未安装）
```bash
npm install --save-dev jest-environment-jsdom
```

### 2. 运行所有测试
```bash
npm test
```

### 3. 运行特定测试
```bash
npm test -- Dashboard.test.tsx
```

### 4. 生成覆盖率报告
```bash
npm test -- --coverage
```

## 📊 测试统计

- **总测试数**: 32 个
- **测试套件**: 9 个
- **代码行数**: ~650 行

## 🎯 关键测试场景

### ✅ 已验证的 Bug 修复
1. 告警列表数据源解析 ✅
2. 刷新按钮功能 ✅
3. API 响应嵌套结构访问 ✅
4. 告警级别样式处理 ✅

### 🔍 覆盖的功能
- 加载状态显示
- 错误处理和重试
- 统计卡片渲染
- 图表组件渲染
- 告警列表和级别
- 用户交互（刷新按钮）
- 边界情况处理
- 数据结构验证

## 📖 关键测试示例

### 测试 API 响应数据解析
```typescript
it('should correctly access nested data structure', () => {
  mockUseGetClustersQuery.mockReturnValue({
    data: {
      success: true,
      data: [{ id: 1, name: 'Cluster A' }],
      timestamp: Date.now(),
    },
    isLoading: false,
    error: null,
  });

  const { getByTestId } = renderDashboard();
  expect(getByTestId('statistic-value')).toHaveTextContent('1');
});
```

### 测试告警级别渲染
```typescript
it('should render critical level alert with correct style', () => {
  mockUseGetAlertsQuery.mockReturnValue({
    data: {
      success: true,
      data: [{
        id: 1,
        level: 'critical',
        message: 'System failure',
      }],
      timestamp: Date.now(),
    },
    isLoading: false,
    error: null,
  });

  const { container } = renderDashboard();
  expect(container.textContent).toContain('🔴');
});
```

## 🔧 故障排除

### 问题：document is not defined
**解决方案**: 安装 jest-environment-jsdom
```bash
npm install --save-dev jest-environment-jsdom
```

### 问题：网络超时
**解决方案**: 设置镜像源
```bash
npm config set registry https://registry.npmmirror.com
npm install --save-dev jest-environment-jsdom
```

### 问题：测试全部失败
**解决方案**: 检查依赖安装
```bash
# 删除 node_modules 和 lock 文件
rm -rf node_modules package-lock.json

# 重新安装所有依赖
npm install

# 再次安装测试依赖
npm install --save-dev jest-environment-jsdom
```

## 📚 更多资源

- **详细文档**: [TESTING_SUMMARY.md](file:///d:/YCYD/metaclouds-frontend/TESTING_SUMMARY.md)
- **覆盖率报告**: [TEST_COVERAGE.md](file:///d:/YCYD/metaclouds-frontend/TEST_COVERAGE.md)
- **测试说明**: [TEST_README.md](file:///d:/YCYD/metaclouds-frontend/TEST_README.md)

## ✅ 验证清单

- [ ] jest-environment-jsdom 已安装
- [ ] 可以运行 `npm test`
- [ ] 所有测试通过
- [ ] 覆盖率报告已生成
- [ ] 了解如何添加新测试

## 🎓 学习目标

通过这个测试文件，您可以学习到：

1. ✅ 如何为 React 组件编写单元测试
2. ✅ 如何使用 React Testing Library
3. ✅ 如何 Mock RTK Query hooks
4. ✅ 如何测试组件的各个状态
5. ✅ 如何处理边界情况
6. ✅ 如何验证 Bug 修复

---

**提示**: 测试是保证代码质量的重要手段，建议在修改组件前先运行测试，修改后再次运行确保无回归！
