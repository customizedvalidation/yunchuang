/**
 * RTK Query API Mock 工厂
 *
 * 离线环境未安装 msw，使用 jest.mock 方案模拟 RTK Query hooks。
 * 本模块提供可复用的 mock 工厂函数，测试文件中通过 jest.mock('../store/api')
 * 替换为这些 mock 实现。
 *
 * 用法示例：
 *   import { mockUseLoginMutation, mockUseGetJobsQuery } from './mocks';
 *   jest.mock('../store/api', () => ({
 *     ...jest.requireActual('../store/api'),
 *     useLoginMutation: mockUseLoginMutation({ success: true }),
 *     useGetJobsQuery: mockUseGetJobsQuery({ data: [...] }),
 *   }));
 *
 * 安装 msw 后可切换为真实请求拦截方案，见 src/test/setup.ts 中的注释。
 */

import type { ApiEnvelope } from '../../store/api';

/** 构造 RTK Query 成功响应信封 */
export function mockEnvelope<T>(data: T): ApiEnvelope<T> {
  return {
    success: true,
    data,
    message: 'ok',
    code: 'SUCCESS',
    timestamp: Date.now(),
  };
}

/** 构造 RTK Query 失败响应（fetchBaseQuery 错误结构） */
export function mockErrorEnvelope(message: string, status = 400) {
  return {
    status,
    data: { message },
  };
}

// ===== Query Hook Mock 工厂 =====

/**
 * mockQueryHook — 通用 useXxxQuery mock 工厂
 *
 * 返回一个 jest mock 函数，调用时返回 { data, isLoading, error, refetch }。
 * refetch 是一个 jest.fn()，可在测试中断言是否被调用。
 */
export function mockQueryHook<T>(options: {
  data?: T;
  isLoading?: boolean;
  error?: unknown;
} = {}) {
  const { data = undefined, isLoading = false, error = undefined } = options;
  const refetch = jest.fn().mockResolvedValue({ data });
  return jest.fn(() => ({
    data,
    isLoading,
    error,
    refetch,
    // 补充 RTK Query 的其他常用字段
    isFetching: isLoading,
    isSuccess: !isLoading && !error,
    isError: Boolean(error),
    originalArgs: undefined,
    requestId: 'mock-request-id',
    endpointName: 'mock',
    startedTimeStamp: Date.now(),
    fulfilledTimeStamp: Date.now(),
  }));
}

// ===== Mutation Hook Mock 工厂 =====

export interface MockMutationResult {
  /** 调用 mutation 时返回的结果（unwrap 后的值） */
  result?: unknown;
  /** 调用 mutation 时抛出的错误（unwrap 会 reject） */
  error?: unknown;
  /** mutation 是否处于 loading 状态 */
  isLoading?: boolean;
}

/**
 * mockMutationHook — 通用 useXxxMutation mock 工厂
 *
 * 返回一个 jest mock 函数，调用时返回 [trigger, { isLoading, error, data }]。
 * trigger 是一个 jest.fn()，调用后返回 { unwrap: () => Promise.resolve(result) }。
 * 若设置了 error，unwrap 会 reject 该错误。
 */
export function mockMutationHook(options: MockMutationResult = {}) {
  const { result, error, isLoading = false } = options;
  const trigger = jest.fn().mockReturnValue({
    unwrap: error
      ? jest.fn().mockRejectedValue(error)
      : jest.fn().mockResolvedValue(result),
  });
  return jest.fn(() => [
    trigger,
    {
      isLoading,
      isSuccess: !error && !isLoading,
      isError: Boolean(error),
      error,
      data: result,
      reset: jest.fn(),
    },
  ]);
}

// ===== 常用预设 Mock =====

/** 空列表查询（data: []） */
export function mockEmptyQuery() {
  return mockQueryHook({ data: mockEnvelope([]) });
}

/** 加载中查询 */
export function mockLoadingQuery() {
  return mockQueryHook({ isLoading: true });
}

/** 错误查询 */
export function mockErrorQuery(message = 'Network error') {
  return mockQueryHook({ error: { status: 500, data: { message } } });
}
