export interface ApiResponse<T = any> {
  success: boolean;
  data: T;
  message?: string;
  code?: string;
  timestamp?: number;
}

export const extractData = <T = any>(response: ApiResponse<T> | undefined): T | undefined => {
  if (!response) {
    return undefined;
  }
  if (!response.success) {
    console.warn('API response failed:', response.message);
    return undefined;
  }
  return response.data;
};

export const extractArrayData = <T = any>(response: ApiResponse<T[]> | undefined): T[] => {
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

export const extractObjectData = <T = any>(response: ApiResponse<T> | undefined): T | null => {
  const data = extractData(response);
  if (!data || typeof data !== 'object') {
    return null;
  }
  return data as T;
};
