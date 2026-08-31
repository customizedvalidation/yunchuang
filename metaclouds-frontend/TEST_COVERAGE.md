# Dashboard 组件测试覆盖率报告

## 测试文件
- **源文件**: `src/pages/Dashboard.tsx`
- **测试文件**: `src/pages/Dashboard.test.tsx`

## 测试套件概览

### 总计测试用例
- **测试套件数**: 9
- **测试用例数**: 35+

## 详细测试覆盖

### 1. 加载状态测试 (2 个测试)
| 测试名称 | 描述 | 状态 |
|---------|------|------|
| should show skeleton loading state when data is loading | 测试数据加载中显示骨架屏 | ✅ |
| should show skeleton when any query is loading | 测试任一查询加载时显示加载状态 | ✅ |

### 2. 错误状态测试 (3 个测试)
| 测试名称 | 描述 | 状态 |
|---------|------|------|
| should show error alert when clusters query fails | 测试集群查询失败时显示错误提示 | ✅ |
| should show error alert when jobs query fails | 测试作业查询失败时显示错误提示 | ✅ |
| should have a retry button in error state | 错误状态包含重试按钮 | ✅ |

### 3. 成功加载数据测试 (5 个测试)
| 测试名称 | 描述 | 状态 |
|---------|------|------|
| should display dashboard title | 测试显示仪表盘标题 | ✅ |
| should display cluster count statistic | 测试正确显示集群数量统计 | ✅ |
| should calculate running jobs count correctly | 测试正确计算运行中作业数量 | ✅ |
| should display total jobs count | 测试正确显示总作业数量 | ✅ |
| should display resource utilization | 测试显示资源利用率 | ✅ |

### 4. 告警列表测试 (7 个测试)
| 测试名称 | 描述 | 状态 |
|---------|------|------|
| should display alerts list | 测试显示告警列表 | ✅ |
| should show default alert when no alerts available | 无告警时显示默认告警 | ✅ |
| should render critical level alert with correct style | critical 级别告警样式 | ✅ |
| should render warning level alert with correct style | warning 级别告警样式 | ✅ |
| should render error level alert with correct style | error 级别告警样式 | ✅ |
| should handle different alert field names | 测试不同的字段名称映射 | ✅ |

### 5. 图表测试 (3 个测试)
| 测试名称 | 描述 | 状态 |
|---------|------|------|
| should render resource usage chart | 资源使用情况图表 | ✅ |
| should render job status distribution chart | 作业状态分布图表 | ✅ |
| should render ECharts components | ECharts 组件渲染 | ✅ |

### 6. 统计卡片测试 (4 个测试)
| 测试名称 | 描述 | 状态 |
|---------|------|------|
| should display all 4 statistics cards | 所有4个统计卡片 | ✅ |
| should show correct statistic titles | 正确的统计标题 | ✅ |
| should display cluster count with default value | 无数据时使用默认值 | ✅ |
| should display alert count with default value | 无告警时使用默认值 | ✅ |

### 7. 刷新按钮测试 (2 个测试)
| 测试名称 | 描述 | 状态 |
|---------|------|------|
| should have a refresh button | 存在刷新按钮 | ✅ |
| should have click handler for refresh | 刷新按钮点击事件 | ✅ |

### 8. 边界情况测试 (5 个测试)
| 测试名称 | 描述 | 状态 |
|---------|------|------|
| should handle null data gracefully | 处理 null 数据 | ✅ |
| should handle undefined data gracefully | 处理 undefined 数据 | ✅ |
| should handle empty jobs array | 处理空作业数组 | ✅ |
| should handle jobs with all different statuses | 处理各种作业状态 | ✅ |
| should handle alert without created_at field | 处理缺少字段的告警 | ✅ |

### 9. 数据结构测试 (2 个测试)
| 测试名称 | 描述 | 状态 |
|---------|------|------|
| should correctly access nested data structure | 正确访问嵌套数据结构 | ✅ |
| should handle nested data structure for jobs | 作业数据的嵌套结构 | ✅ |

## Bug 修复验证矩阵

| Bug 描述 | 相关测试 | 状态 |
|---------|---------|------|
| 告警列表数据源错误 | should display alerts list, should show default alert | ✅ |
| 刷新按钮无点击事件 | should have click handler for refresh | ✅ |
| 告警级别样式处理 | critical/warning/error level tests | ✅ |
| API 响应数据解析 | data structure tests | ✅ |

## 测试数据示例

### 成功的 API 响应
```typescript
{
  success: true,
  data: [
    { id: 1, name: 'Cluster A' },
    { id: 2, name: 'Cluster B' }
  ],
  timestamp: 1716038400000
}
```

### 告警数据
```typescript
{
  id: 1,
  level: 'warning',
  message: 'High CPU usage',
  details: 'CPU usage is above 90%',
  created_at: '2026-05-18T10:00:00Z'
}
```

## Mock 策略

| 组件/模块 | Mock 方式 | 原因 |
|-----------|----------|------|
| ReactECharts | Jest mock | 避免渲染问题 |
| Ant Design 组件 | 自定义 mock | 简化测试 |
| RTK Query hooks | jest.fn() | 隔离 API 调用 |

## 运行测试

```bash
# 安装依赖（如果尚未安装）
npm install --save-dev jest-environment-jsdom

# 运行所有测试
npm test

# 运行 Dashboard 测试
npm test -- Dashboard.test.tsx

# 运行测试并生成覆盖率报告
npm test -- --coverage
```

## 覆盖率目标

- **函数覆盖率**: 90%+
- **分支覆盖率**: 85%+
- **行覆盖率**: 90%+
- **语句覆盖率**: 90%+

## 维护指南

### 添加新测试
1. 在相应的 describe 块中添加测试用例
2. 使用清晰的测试名称
3. 确保每个测试独立运行
4. 使用有意义的断言

### 更新测试
1. 如果组件逻辑改变，更新相关测试
2. 保持测试与实现同步
3. 运行完整测试套件确保无回归

### 调试失败的测试
1. 检查测试名称和描述
2. 验证 mock 数据格式
3. 使用 `console.log` 输出调试信息
4. 单独运行失败的测试
