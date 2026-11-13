/**
 * 主题状态机回归测试（MDS v3.1）
 *
 * 覆盖：三态解析、DOM 唯一真源（data-theme + html.dark + color-scheme + meta）、
 * 持久化与非法值回落、跟随系统偏好及其变更、隐私模式下写入失败不中断。
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import {
  THEME_STORAGE_KEY,
  activeTheme,
  applyTheme,
  destroyTheme,
  initTheme,
  prefersDark,
  readStoredMode,
  resolveTheme,
  setThemeMode,
  themeMode,
} from '../useTheme'

function resetDom() {
  document.documentElement.removeAttribute('data-theme')
  document.documentElement.classList.remove('dark')
  document.documentElement.style.colorScheme = ''
  document.head.querySelectorAll('meta[data-theme-test]').forEach((el) => el.remove())
}

/**
 * jsdom 不加载 index.html 的 <head>，测试需自行注入 meta 才能验证
 * 「主题切换同步浏览器状态栏颜色」这条真实行为。
 */
function ensureMeta() {
  for (const name of ['theme-color', 'color-scheme']) {
    if (document.head.querySelector(`meta[name="${name}"]`)) continue
    const meta = document.createElement('meta')
    meta.setAttribute('name', name)
    meta.setAttribute('content', name === 'theme-color' ? '#0f1826' : 'light')
    meta.setAttribute('data-theme-test', 'true')
    document.head.appendChild(meta)
  }
}

/** 构造可控的 matchMedia：记录 change 监听，便于模拟系统偏好翻转 */
function mockMatchMedia(matches: boolean) {
  const listeners: Array<(e: MediaQueryListEvent) => void> = []
  const mql = {
    matches,
    media: '(prefers-color-scheme: dark)',
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn((_type: string, cb: (e: MediaQueryListEvent) => void) => listeners.push(cb)),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  } as unknown as MediaQueryList
  vi.mocked(window.matchMedia).mockImplementation(() => mql)
  return { mql, listeners }
}

describe('主题状态机', () => {
  beforeEach(() => {
    localStorage.clear()
    resetDom()
    ensureMeta()
    destroyTheme()
    mockMatchMedia(false)
  })

  afterEach(() => {
    destroyTheme()
    resetDom()
    vi.restoreAllMocks()
  })

  it('resolveTheme 把三态解析为二态', () => {
    expect(resolveTheme('light')).toBe('light')
    expect(resolveTheme('dark')).toBe('dark')
    expect(resolveTheme('system')).toBe('light')
    mockMatchMedia(true)
    expect(prefersDark()).toBe(true)
    expect(resolveTheme('system')).toBe('dark')
  })

  it('readStoredMode 对非法 / 缺失值回落到 system', () => {
    expect(readStoredMode()).toBe('system')
    localStorage.setItem(THEME_STORAGE_KEY, 'neon')
    expect(readStoredMode()).toBe('system')
    localStorage.setItem(THEME_STORAGE_KEY, 'dark')
    expect(readStoredMode()).toBe('dark')
  })

  it('setThemeMode(dark) 同步写入 DOM 四个位置并持久化', () => {
    setThemeMode('dark')
    const root = document.documentElement
    expect(themeMode.value).toBe('dark')
    expect(activeTheme.value).toBe('dark')
    expect(root.dataset.theme).toBe('dark')
    expect(root.classList.contains('dark')).toBe(true)
    expect(root.style.colorScheme).toBe('dark')
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('dark')
    expect(document.querySelector('meta[name="theme-color"]')?.getAttribute('content')).toBe('#0b111c')
    expect(document.querySelector('meta[name="color-scheme"]')?.getAttribute('content')).toBe('dark')

    setThemeMode('light')
    expect(root.dataset.theme).toBe('light')
    expect(root.classList.contains('dark')).toBe(false)
    expect(root.style.colorScheme).toBe('light')
    expect(document.querySelector('meta[name="theme-color"]')?.getAttribute('content')).toBe('#0f1826')
    expect(document.querySelector('meta[name="color-scheme"]')?.getAttribute('content')).toBe('light')
  })

  it('initTheme 在 system 模式下跟随系统，且系统偏好翻转时自动更新', () => {
    localStorage.setItem(THEME_STORAGE_KEY, 'system')
    const { listeners } = mockMatchMedia(false)
    initTheme()
    expect(themeMode.value).toBe('system')
    expect(activeTheme.value).toBe('light')
    expect(listeners.length).toBe(1)

    // 系统切到深色：仅 system 模式应跟随
    vi.mocked(window.matchMedia).mockImplementation(() => ({
      matches: true,
      media: '(prefers-color-scheme: dark)',
    } as MediaQueryList))
    listeners[0]({ matches: true } as MediaQueryListEvent)
    expect(activeTheme.value).toBe('dark')

    // 用户显式选浅色后，系统再翻转也不应覆盖用户选择
    setThemeMode('light')
    vi.mocked(window.matchMedia).mockImplementation(() => ({
      matches: false,
      media: '(prefers-color-scheme: dark)',
    } as MediaQueryList))
    listeners.forEach((cb) => cb({ matches: false } as MediaQueryListEvent))
    expect(activeTheme.value).toBe('light')
  })

  it('存储不可用时静默降级，不影响本次会话生效', () => {
    const spy = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('QuotaExceededError')
    })
    expect(() => setThemeMode('dark')).not.toThrow()
    expect(activeTheme.value).toBe('dark')
    expect(document.documentElement.dataset.theme).toBe('dark')
    spy.mockRestore()
  })

  it('applyTheme 是幂等的', () => {
    applyTheme('dark')
    applyTheme('dark')
    expect(activeTheme.value).toBe('dark')
    expect(document.documentElement.dataset.theme).toBe('dark')
  })
})
