/**
 * 外观切换组件回归测试（MDS v3.1）
 * 验证三态命令确实驱动主题状态机，而不只是改了个图标。
 */
import { describe, it, expect, beforeEach, afterEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { ElDropdown } from 'element-plus'
import ThemeToggle from '../ThemeToggle.vue'
import { THEME_STORAGE_KEY, activeTheme, destroyTheme, setThemeMode, themeMode } from '@/theme/useTheme'

describe('ThemeToggle', () => {
  beforeEach(() => {
    localStorage.clear()
    destroyTheme()
    setThemeMode('light')
  })

  afterEach(() => {
    document.documentElement.removeAttribute('data-theme')
    document.documentElement.classList.remove('dark')
    document.documentElement.style.colorScheme = ''
  })

  it('按钮带可访问名称与 menu 语义', () => {
    const wrapper = mount(ThemeToggle)
    const btn = wrapper.find('button')
    expect(btn.attributes('aria-haspopup')).toBe('menu')
    expect(btn.attributes('aria-label')).toContain('外观设置')
    expect(btn.attributes('aria-label')).toContain('浅色')
    wrapper.unmount()
  })

  it('三个外观选项齐备，命令切换 dark 后状态机与 DOM 同步', async () => {
    const wrapper = mount(ThemeToggle)
    const dropdown = wrapper.findComponent(ElDropdown)
    expect(dropdown.exists()).toBe(true)

    await dropdown.vm.$emit('command', 'dark')
    expect(themeMode.value).toBe('dark')
    expect(activeTheme.value).toBe('dark')
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('dark')
    expect(document.documentElement.dataset.theme).toBe('dark')

    await dropdown.vm.$emit('command', 'system')
    expect(themeMode.value).toBe('system')
    expect(localStorage.getItem(THEME_STORAGE_KEY)).toBe('system')
    wrapper.unmount()
  })

  it('忽略非法命令，不破坏当前主题', async () => {
    const wrapper = mount(ThemeToggle)
    const dropdown = wrapper.findComponent(ElDropdown)
    await dropdown.vm.$emit('command', 'neon')
    expect(themeMode.value).toBe('light')
    wrapper.unmount()
  })
})
