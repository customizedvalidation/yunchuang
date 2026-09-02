import React, { createContext, useCallback, useContext, useState, useMemo } from 'react';

/** 密度模式：comfortable=默认舒适；compact=紧凑（叠加 antd compactAlgorithm） */
export type DensityMode = 'comfortable' | 'compact';

const STORAGE_KEY = 'mc-density';

interface DensityContextValue {
  density: DensityMode;
  isCompact: boolean;
  setDensity: (d: DensityMode) => void;
  toggle: () => void;
}

const DensityContext = createContext<DensityContextValue | null>(null);

const readInitial = (): DensityMode => {
  try {
    const saved = localStorage.getItem(STORAGE_KEY);
    if (saved === 'compact' || saved === 'comfortable') return saved;
  } catch {
    /* localStorage 不可用时忽略，回退默认 */
  }
  return 'comfortable';
};

export const DensityProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [density, setDensityState] = useState<DensityMode>(readInitial);

  const setDensity = useCallback((next: DensityMode) => {
    setDensityState(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      /* 忽略写入失败 */
    }
  }, []);

  const toggle = useCallback(() => {
    setDensityState((prev) => {
      const next: DensityMode = prev === 'compact' ? 'comfortable' : 'compact';
      try {
        localStorage.setItem(STORAGE_KEY, next);
      } catch {
        /* 忽略写入失败 */
      }
      return next;
    });
  }, []);

  const value = useMemo<DensityContextValue>(
    () => ({ density, isCompact: density === 'compact', setDensity, toggle }),
    [density, setDensity, toggle],
  );

  return <DensityContext.Provider value={value}>{children}</DensityContext.Provider>;
};

export const useDensity = (): DensityContextValue => {
  const ctx = useContext(DensityContext);
  if (!ctx) throw new Error('useDensity 必须在 DensityProvider 内使用');
  return ctx;
};
