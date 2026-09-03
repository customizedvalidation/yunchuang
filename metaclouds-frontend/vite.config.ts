import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:8000',
        changeOrigin: true
      }
    }
  },
  build: {
    // ECharts 是体积最大的 vendor，且已随路由懒加载（Dashboard / MonitoringAlert），
    // 上调告警阈值避免误报；真正的收益来自下方 manualChunks 的 vendor 拆包。
    chunkSizeWarningLimit: 1200,
    rollupOptions: {
      output: {
        // 按依赖类别拆分 vendor chunk，提升浏览器长效缓存命中率：
        // 应用代码变更时 vendor chunk（react / antd / echarts）哈希不变，可复用缓存。
        manualChunks(id: string) {
          if (!id.includes('node_modules')) return undefined
          // 统一为 POSIX 分隔符，规避 Windows 反斜杠导致的匹配失效
          const p = id.replace(/\\/g, '/')
          // 图表库：echarts 及其渲染层 zrender
          if (id.includes('echarts') || id.includes('zrender')) return 'echarts'
          // antd 及其子包（rc-*、@ant-design、@rc-component），含 cssinjs 的 @emotion
          if (
            id.includes('antd') ||
            id.includes('@ant-design') ||
            id.includes('rc-') ||
            id.includes('@rc-component') ||
            id.includes('@emotion')
          ) {
            return 'antd'
          }
          // React 核心三件套（react / react-dom / scheduler）自包含、无第三方依赖，
          // 单独成块可打破 vendor <-> react-vendor 的循环依赖（路由/状态库留在 vendor）。
          if (
            p.includes('/node_modules/react/') ||
            p.includes('/node_modules/react-dom/') ||
            p.includes('/node_modules/scheduler/')
          ) {
            return 'react-vendor'
          }
          // 其余第三方依赖（react-router / redux / dayjs / axios ...）归并到 vendor
          return 'vendor'
        }
      }
    }
  }
})