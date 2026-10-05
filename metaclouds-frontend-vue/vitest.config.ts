/// <reference types="vitest" />
import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'
import { mkdirSync } from 'node:fs'

// 本机 WorkBuddy 沙箱禁止写系统 %TEMP%（brokered-fs EPERM），而 vitest 的
// vite-node 会把转换后的模块缓存到 `os.tmpdir()/web/<sha1>`：写失败时测试文件
// 会被静默漏收（5 个 spec 只跑到 3 个）或随机失败。这里在加载期把 TMP/TEMP
// 重定向到项目内可写目录，保证本仓库测试在该环境下结果稳定。
// 在普通开发机上写到项目内 node_modules 同样无害（已被 .gitignore 排除）。
const vitestTmpDir = fileURLToPath(new URL('./node_modules/.vitest-tmp', import.meta.url))
mkdirSync(vitestTmpDir, { recursive: true })
process.env.TMP = vitestTmpDir
process.env.TEMP = vitestTmpDir

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
    // vmThreads 在主进程内运行测试：不经过 worker IPC 转换管线，
    // 也就不会触发「vite-node 把转换模块写盘缓存」这一步。本机沙箱对每个
    // 文件写都要过 broker 审批，forks/threads 池在模块图突发抓取时会有
    // 个别写被拒（EPERM），表现为测试文件被静默漏收、结果随机。
    pool: 'vmThreads',
    coverage: {
      enabled: false,
      provider: 'v8',
      reporter: ['text', 'html'],
      include: ['src/**/*.{ts,vue}'],
      exclude: [
        'src/**/*.{test,spec}.{ts,tsx}',
        'src/test/**',
        'src/main.ts',
        'src/**/*.d.ts',
      ],
    },
  },
})
