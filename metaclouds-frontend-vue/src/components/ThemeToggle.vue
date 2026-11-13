<template>
  <!--
    外观切换：三态（浅色 / 深色 / 跟随系统）。
    图标按「已解析主题」显示，菜单项标记的是「用户选择的模式」，
    两者分离，避免 system 模式下无法表达当前实际是亮还是暗。
  -->
  <el-dropdown trigger="click" @command="handleCommand">
    <button
      type="button"
      class="mc-icon-btn mc-theme-trigger"
      :aria-label="`外观设置，当前${modeLabel}（${theme === 'dark' ? '深色' : '浅色'}）`"
      aria-haspopup="menu"
    >
      <svg v-if="theme === 'dark'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M20.5 14.3A8.5 8.5 0 0 1 9.7 3.5 8.5 8.5 0 1 0 20.5 14.3Z" />
      </svg>
      <svg v-else viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" aria-hidden="true">
        <circle cx="12" cy="12" r="4" />
        <path d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41" />
      </svg>
    </button>
    <template #dropdown>
      <el-dropdown-menu>
        <el-dropdown-item v-for="opt in options" :key="opt.value" :command="opt.value">
          <span class="mc-theme-option" :class="{ 'is-active': mode === opt.value }">
            <span>{{ opt.label }}</span>
            <span v-if="mode === opt.value" class="mc-theme-check" aria-hidden="true">✓</span>
          </span>
        </el-dropdown-item>
      </el-dropdown-menu>
    </template>
  </el-dropdown>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useTheme, type ThemeMode } from '@/theme/useTheme'

const { mode, theme, setMode } = useTheme()

const options: { label: string; value: ThemeMode }[] = [
  { label: '浅色', value: 'light' },
  { label: '深色', value: 'dark' },
  { label: '跟随系统', value: 'system' },
]

const modeLabel = computed(
  () => options.find((o) => o.value === mode.value)?.label ?? '跟随系统',
)

function handleCommand(command: ThemeMode) {
  if (command === 'light' || command === 'dark' || command === 'system') setMode(command)
}
</script>
