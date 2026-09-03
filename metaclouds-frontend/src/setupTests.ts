import '@testing-library/jest-dom';

// node 测试环境没有 localStorage；前端鉴权工具（readStoredRole 等）依赖它，这里做最小补全。
// 仅实现测试用到的子集（getItem/setItem/removeItem/clear）。
class LocalStorageMock {
  private store: Record<string, string> = {};

  getItem(key: string): string | null {
    return Object.prototype.hasOwnProperty.call(this.store, key) ? this.store[key] : null;
  }

  setItem(key: string, value: string): void {
    this.store[key] = String(value);
  }

  removeItem(key: string): void {
    delete this.store[key];
  }

  clear(): void {
    this.store = {};
  }

  key(index: number): string | null {
    return Object.keys(this.store)[index] ?? null;
  }

  get length(): number {
    return Object.keys(this.store).length;
  }
}

(globalThis as unknown as { localStorage: Storage }).localStorage =
  new LocalStorageMock() as unknown as Storage;
