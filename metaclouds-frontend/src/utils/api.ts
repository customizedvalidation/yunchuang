export interface ApiResponse<T = any> {
  success: boolean;
  data: T;
  message?: string;
  code?: string;
  timestamp?: number;
}

export const extractData = <T = any>(response: ApiResponse<T> | T | undefined): T | undefined => {
  if (!response) {
    return undefined;
  }
  // 兼容两种格式：
  // 1. 后端统一信封 { success, data, message, code, timestamp }
  // 2. baseQueryWithReauth 统一解包后的直接数据（数组/对象）
  if (typeof response === 'object' && response !== null && 'success' in response) {
    const envelope = response as ApiResponse<T>;
    if (!envelope.success) {
      console.warn('API response failed:', envelope.message);
      return undefined;
    }
    return envelope.data;
  }
  return response as T;
};

export const extractArrayData = <T = any>(response: ApiResponse<T[]> | T[] | undefined): T[] => {
  const data = extractData(response);
  if (!data) {
    return [];
  }
  if (!Array.isArray(data)) {
    console.warn('Expected array data but got:', typeof data);
    return [];
  }
  return data;
};

export const extractObjectData = <T = any>(response: ApiResponse<T> | T | undefined): T | null => {
  const data = extractData(response);
  if (!data || typeof data !== 'object') {
    return null;
  }
  return data as T;
};
