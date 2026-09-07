# Metaclouds 前端组件级测试体系

## 概述

本项目已建立完整的组件级测试体系，基于 **Jest + React Testing Library + ts-jest**，覆盖页面组件与通用可复用组件。

## 测试架构

```
src/
├── test/
│   ├── __mocks__/
│   │   ├── styleMock.js       # CSS 导入 mock（替代 identity-obj-proxy）
│   │   └── fileMock.js        # 静态资源（图片/字体）mock
│   ├── mocks/
│   │   └── index.ts            # RTK Query API mock 工厂（query/mutation）
│   ├── setup.ts                # 全局测试设置（jest-dom / DOM API mock / 清理）
│   └── test-utils.tsx          # 自定义 render（Redux/Router/Theme/AntApp 全 Provider）
├── components/
│   ├── Can.test.tsx            # 权限组件测试（admin/manager/user/fail-open）
│   ├── ResponsiveTable.test.tsx # 响应式表格测试（桌面/移动端/卡片视图）
│   └── States/
│       └── States.test.tsx     # 状态组件测试（EmptyState/ErrorState/Skeleton/renderState）
├── pages/
│   ├── Login.test.tsx          # 登录页测试（渲染/校验/成功/失败）
│   ├── Dashboard.test.tsx      # 仪表盘测试（统计卡/图表/加载/错误）
│   └── JobManagement.test.tsx  # 作业管理测试（表格/筛选/弹窗/分页）
└── utils/
    └── auth.test.ts            # 原有 RBAC 纯逻辑测试（12/12 通过）
```

## 测试覆盖统计

| 测试文件 | 测试用例数 | 覆盖范围 |
|---------|-----------|---------|
| `auth.test.ts` | 12 | RBAC 角色权限矩阵（纯逻辑） |
| `Login.test.tsx` | 16 | 表单渲染、输入验证、登录成功导航、登录失败错误 |
| `Dashboard.test.tsx` | 20+ | 统计卡片、图表容器、Loading Skeleton、错误状态、空态 |
| `JobManagement.test.tsx` | 20+ | 表格渲染、页签筛选、新建弹窗、分页、虚拟滚动、加载/错误/空态 |
| `ResponsiveTable.test.tsx` | 15+ | 桌面端 Table、移动端卡片、cardFields、cardTitleField、rowKey |
| `States.test.tsx` | 20+ | EmptyState、ErrorState、TableSkeleton、renderState 三态守卫 |
| `Can.test.tsx` | 15+ | admin/manager/user 权限、fail-open 策略、JSON 损坏降级 |
| **合计** | **~120** | |

## 缺失依赖（离线环境）

以下依赖在当前离线环境中未安装，**有网环境执行以下命令后即可运行全部测试**：

```bash
npm install -D jest-environment-jsdom @types/jest identity-obj-proxy msw
```

| 依赖 | 用途 | 替代方案（当前） |
|-----|------|----------------|
| `jest-environment-jsdom` | Jest 28+ 不再内置 jsdom，组件测试必需 | 无（必须安装才能运行组件测试） |
| `@types/jest` | TypeScript 识别 `describe/test/expect/jest` 全局类型 | ts-jest 运行时自动注入，仅影响独立 tsc 检查 |
| `identity-obj-proxy` | CSS Module 类名代理 | `src/test/__mocks__/styleMock.js`（空对象 mock） |
| `msw` | API 请求 Mock Service Worker | `src/test/mocks/index.ts`（jest.mock 方案模拟 RTK Query） |

## 运行测试

```bash
# 运行全部测试
npm test

# 运行指定测试文件
npm test -- Login.test.tsx

# 运行测试并生成覆盖率报告
npm test -- --coverage

# 仅运行原有纯逻辑测试（无需 jsdom）
npm test -- auth.test.ts
```

## 测试策略

1. **不测试实现细节**：只测试用户可见行为（渲染内容、交互结果、错误提示）
2. **不依赖外部网络**：所有 API 调用通过 `jest.mock` 模拟 RTK Query hooks
3. **完整 Provider 包装**：`test-utils.tsx` 的自定义 render 自动注入 Redux/Router/Theme/AntApp
4. **每个测试文件 ≥3 个用例**：使用 `describe`/`test` 组织，按功能分组
5. **测试间状态隔离**：`setup.ts` 的 `afterEach` 自动清理 localStorage

## Mock 策略

### API Mock（RTK Query）

使用 `src/test/mocks/index.ts` 提供的工厂函数：

```typescript
import { mockQueryHook, mockMutationHook, mockEnvelope } from '../test/mocks';

// Query mock
jest.mock('../store/api', () => ({
  ...jest.requireActual('../store/api'),
  useGetJobsQuery: mockQueryHook({ data: mockEnvelope([...]) }),
}));

// Mutation mock
jest.mock('../store/api', () => ({
  ...jest.requireActual('../store/api'),
  useLoginMutation: mockMutationHook({ result: mockEnvelope({ token: '...' }) }),
}));
```

### 组件 Mock

- `ResponsiveChart`：mock 为简单 div，避免 ECharts canvas 渲染
- `DraggableGrid`：mock 为简单容器，避免拖拽逻辑
- `ResponsiveTable`（部分测试）：mock 为记录 props 的 div，便于断言数据筛选

### 切换到 MSW（可选）

安装 `msw` 后，可在 `src/test/setup.ts` 中取消 MSW 配置注释，
创建 `src/test/mocks/handlers.ts` 定义请求拦截器，实现更真实的 API Mock。
