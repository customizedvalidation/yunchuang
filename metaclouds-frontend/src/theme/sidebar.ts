export const SIDEBAR_WIDTH = {
  DEFAULT: 240,
  COLLAPSED: 64,
  TABLET: 200,
  MOBILE: 260,
  PHONE: 240,
};

export const SIDEBAR_BREAKPOINTS = {
  TABLET: 1024,
  MOBILE: 768,
  PHONE: 480,
};

export const getSidebarWidth = (collapsed: boolean, screenWidth?: number): number => {
  const width = screenWidth ?? (typeof window !== 'undefined' ? window.innerWidth : Infinity);
  
  if (width <= SIDEBAR_BREAKPOINTS.PHONE) {
    return collapsed ? SIDEBAR_WIDTH.COLLAPSED : SIDEBAR_WIDTH.PHONE;
  }
  
  if (width <= SIDEBAR_BREAKPOINTS.MOBILE) {
    return collapsed ? SIDEBAR_WIDTH.COLLAPSED : SIDEBAR_WIDTH.MOBILE;
  }
  
  if (width <= SIDEBAR_BREAKPOINTS.TABLET) {
    return collapsed ? SIDEBAR_WIDTH.COLLAPSED : SIDEBAR_WIDTH.TABLET;
  }
  
  return collapsed ? SIDEBAR_WIDTH.COLLAPSED : SIDEBAR_WIDTH.DEFAULT;
};

export const SIDEBAR_COLORS = {
  PRIMARY: '#3b82f6',
  PRIMARY_LIGHT: '#60a5fa',
  PRIMARY_DARK: '#2563eb',
  SECONDARY: '#8b5cf6',
  SECONDARY_LIGHT: '#a78bfa',
  ACCENT: '#ec4899',
};
