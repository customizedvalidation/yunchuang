# Dashboard 组件单元测试

## 测试文件位置
`src/pages/Dashboard.test.tsx`

## 运行测试

### 1. 安装依赖
由于网络问题，请手动运行以下命令：

```bash
npm install --save-dev jest-environment-jsdom
```

### 2. 运行所有测试
```bash
npm test
```

### 3. 运行特定测试文件
```bash
npm test -- Dashboard.test.tsx
```

### 4. 运行测试并显示详细输出
```bash
npm test -- --verbose
```

## 测试覆盖范围

### 1. 加载状态测试
- ✅ 测试数据加载中显示骨架屏
- ✅ 测试任一查询加载时显示加载状态

### 2. 错误状态测试
- ✅ 测试集群查询失败时显示错误提示
- ✅ 测试作业查询失败时显示错误提示
- ✅ 错误状态包含重试按钮

### 3. 成功加载数据测试
- ✅ 测试显示仪表盘标题
- ✅ 测试正确显示集群数量统计
- ✅ 测试正确计算运行中作业数量
- ✅ 测试正确显示总作业数量
- ✅ 测试显示资源利用率

### 4. 告警列表测试
- ✅ 测试显示告警列表
- ✅ 测试无告警时显示默认告警
- ✅ 测试 critical 级别告警样式
- ✅ 测试 warning 级别告警样式
- ✅ 测试 error 级别告警样式
- ✅ 测试不同的字段名称映射

### 5. 图表测试
- ✅ 测试渲染资源使用情况图表
- ✅ 测试渲染作业状态分布图表
- ✅ 测试渲染 ECharts 组件

### 6. 统计卡片测试
- ✅ 测试显示所有4个统计卡片
- ✅ 测试显示正确的统计标题
- ✅ 测试无数据时使用默认值

### 7. 刷新按钮测试
- ✅ 测试存在刷新按钮
- ✅ 测试刷新按钮点击事件

### 8. 边界情况测试
- ✅ 测试处理 null 数据
- ✅ 测试处理 undefined 数据
- ✅ 测试处理空作业数组
- ✅ 测试处理各种作业状态
- ✅ 测试处理缺少 created_at 字段的告警

### 9. 数据结构测试
- ✅ 测试正确访问 API 响应的嵌套数据结构
- ✅ 测试作业数据的嵌套结构访问

## 测试策略

### Mock 策略
- 使用 Jest mock 函数模拟 RTK Query hooks
- Mock Ant Design 组件以便进行单元测试
- Mock ECharts 组件避免渲染问题

### 测试数据
测试使用标准 API 响应格式：
```typescript
{
  success: true,
  data: [...],
  timestamp: number
}
```

## 修复的 Bug 验证

测试文件专门验证了以下已修复的 Bug：

1. **数据解析 Bug**：测试验证代码正确访问 `data.data` 而不是直接访问 `data`
2. **告警列表数据源**：测试验证使用 `alerts?.data` 而不是 `alerts`
3. **刷新按钮功能**：测试验证按钮有正确的点击处理函数
4. **告警级别渲染**：测试验证所有告警级别（critical、error、warning、success）都能正确渲染

## 注意事项

- 测试需要在 Node.js 环境中运行
- 确保已安装所有测试依赖（特别是 jest-environment-jsdom）
- 如果遇到网络问题，请配置 npm 镜像源：
  ```bash
  npm config set registry https://registry.npmmirror.com
  ```
