/**
 * ============================================================
 * MDS v3.1 — 主题状态机（light / dark / system）
 * ============================================================
 *
 * 设计约束：
 *  1. 单一真源：DOM 上的 `data-theme` + `html.dark` class 由本模块唯一写入；
 *     `html.dark` 是 Element Plus 官方 dark/css-vars.css 的挂载点，
 *     `data-theme` 供 MDS 的 `html.dark[data-theme='dark']` 高特异性块使用。
 *  2. 防闪白（FOUC）：`index.html` 的同步内联脚本在 CSS 之前先写一版；
 *     `initTheme()` 在 main.ts 中于 mount 前再同步一次 ref、注册系统偏好监听。
 *  3. 三态：`system` 跟随 `prefers-color-scheme`，用户显式选择后写入
 *     localStorage（隐私模式下写入失败静默降级，不影响本次会话）。
 *  4. `activeTheme` 是「已解析」的 light/dark，ECharts 等无法读 CSS 的
 *     场景订阅它，不要自己再解析一次系统偏好。
 * ============================================================
 */
import { computed, ref } from 'vue'

export type ThemeMode = 'light' | 'dark' | 'system'
export type ResolvedTheme = 'light' | 'dark'

export const THEME_STORAGE_KEY = 'mc-theme'

/** 用户选择的模式（三态），默认跟随系统 */
export const themeMode = ref<ThemeMode>('system')
/** 已解析生效的主题（二态），ECharts / 逻辑层订阅此值 */
export const activeTheme = ref<ResolvedTheme>('light')

let initialized = false
let systemQuery: MediaQueryList | null = null
let onSystemChange: ((e: MediaQueryListEvent) => void) | null = null

function isBrowser(): boolean {
  return typeof window !== 'undefined' && typeof document !== 'undefined'
}

/** 系统是否偏好深色；测试环境 matchMedia 被 mock 成 matches:false，故默认 light */
export function prefersDark(): boolean {
  if (!isBrowser() || typeof window.matchMedia !== 'function') return false
  try {
    return window.matchMedia('(prefers-color-scheme: dark)').matches === true
  } catch {
    return false
  }
}

/** 读取持久化选择，非法值 / 存储不可用时一律回落 system */
export function readStoredMode(): ThemeMode {
  if (!isBrowser()) return 'system'
  try {
    const raw = window.localStorage.getItem(THEME_STORAGE_KEY)
    return raw === 'light' || raw === 'dark' || raw === 'system' ? raw : 'system'
  } catch {
    return 'system'
  }
}

/** 三态 → 二态 */
export function resolveTheme(mode: ThemeMode): ResolvedTheme {
  if (mode === 'dark') return 'dark'
  if (mode === 'light') return 'light'
  return prefersDark() ? 'dark' : 'light'
}

/** 把已解析主题写到 DOM，并同步浏览器 UI（滚动条 / 表单控件 / 移动端状态栏） */
export function applyTheme(theme: ResolvedTheme): void {
  activeTheme.value = theme
  if (!isBrowser()) return

  const root = document.documentElement
  root.dataset.theme = theme
  root.classList.toggle('dark', theme === 'dark')
  root.style.colorScheme = theme

  // <meta name="theme-color"> 与 <meta name="color-scheme"> 同步，
  // 否则移动端浏览器地址栏仍是浅色，形成「半亮半暗」的破面。
  const metaColor = document.querySelector('meta[name="theme-color"]')
  if (metaColor) metaColor.setAttribute('content', theme === 'dark' ? '#0b111c' : '#0f1826')
  const metaScheme = document.querySelector('meta[name="color-scheme"]')
  if (metaScheme) metaScheme.setAttribute('content', theme)
}

/** 切换模式：持久化 + 立即生效 */
export function setThemeMode(mode: ThemeMode): void {
  themeMode.value = mode
  if (isBrowser()) {
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, mode)
    } catch {
      // 隐私模式 / 配额耗尽：本次会话仍生效，不抛错打断 UI
    }
  }
  applyTheme(resolveTheme(mode))
}

/**
 * 应用启动时调用一次（幂等）。
 * 内联脚本已先应用过主题，这里主要负责同步 Vue 侧 ref 与注册系统偏好监听。
 */
export function initTheme(): void {
  if (!isBrowser() || initialized) return
  initialized = true

  const mode = readStoredMode()
  themeMode.value = mode
  applyTheme(resolveTheme(mode))

  if (typeof window.matchMedia !== 'function') return
  try {
    systemQuery = window.matchMedia('(prefers-color-scheme: dark)')
    onSystemChange = () => {
      if (themeMode.value === 'system') applyTheme(resolveTheme('system'))
    }
    if (typeof systemQuery.addEventListener === 'function') {
      systemQuery.addEventListener('change', onSystemChange)
    } else if (typeof systemQuery.addListener === 'function') {
      // Safari < 14 兼容
      systemQuery.addListener(onSystemChange)
    }
  } catch {
    systemQuery = null
    onSystemChange = null
  }
}

/** 卸载监听（仅测试 / HMR 场景需要） */
export function destroyTheme(): void {
  if (systemQuery && onSystemChange) {
    try {
      if (typeof systemQuery.removeEventListener === 'function') {
        systemQuery.removeEventListener('change', onSystemChange)
      } else if (typeof systemQuery.removeListener === 'function') {
        systemQuery.removeListener(onSystemChange)
      }
    } catch {
      /* noop */
    }
  }
  systemQuery = null
  onSystemChange = null
  initialized = false
}

/** 组件侧统一入口 */
export function useTheme() {
  return {
    mode: themeMode,
    theme: activeTheme,
    isDark: computed(() => activeTheme.value === 'dark'),
    setMode: setThemeMode,
  }
}

export default useTheme
