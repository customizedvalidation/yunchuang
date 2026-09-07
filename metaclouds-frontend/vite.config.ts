import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react(),
    // ===== 构建时包体积分析（可选启用）=====
    // 离线环境未安装 vite-plugin-bundle-analyzer，有网环境执行：
    //   npm install -D vite-plugin-bundle-analyzer
    // 然后取消下方注释，并运行：npx vite build --mode analyze
    // 构建完成后会在 dist/.analyze/ 下生成交互式 treemap HTML，
    // 可直观查看每个 chunk 的模块构成与体积占比。
    //
    // import { visualizer } from 'vite-plugin-bundle-analyzer'
    // visualizer({
    //   filename: 'dist/.analyze/stats.html',
    //   gzipSize: true,
    //   brotliSize: true,
    //   open: false,
    // }),
  ],
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
    // ECharts 按需引入后体积已降至 ~300KB（原 ~1MB），
    // 告警阈值维持 1200KB 以覆盖 antd 路由级拆分产生的中等 chunk。
    chunkSizeWarningLimit: 1200,
    rollupOptions: {
      output: {
        // 按依赖类别拆分 vendor chunk，提升浏览器长效缓存命中率：
        // 应用代码变更时 vendor chunk（react / antd / echarts）哈希不变，可复用缓存。
        manualChunks(id: string) {
          if (!id.includes('node_modules')) return undefined
          // 统一为 POSIX 分隔符，规避 Windows 反斜杠导致的匹配失效
          const p = id.replace(/\\/g, '/')
          // ECharts 全家桶（echarts core / charts / components / zrender / echarts-for-react）
          // 统一打入 echarts chunk：随 Dashboard / MonitoringAlert 路由懒加载，不进首屏。
          if (
            p.includes('/node_modules/echarts/') ||
            p.includes('/node_modules/zrender/') ||
            p.includes('/node_modules/echarts-for-react/')
          ) {
            return 'echarts'
          }
          // antd 不建独立 manual chunk：
          //   App.tsx 静态 import { ConfigProvider, Spin, App } from 'antd' 会命中 barrel，
          //   一旦把 antd / rc-* / @ant-design / @emotion 强制合并成 'antd' 块，
          //   barrel 的静态 re-export 会把首屏未用到的 Table / DatePicker / Tree / Upload /
          //   Cascader 等 rc-* 全部绑进首屏关键路径（实测 ~1200 kB）。
          //   返回 undefined 让 Rollup 按可达性按路由自动切分：
          //   入口实际用到的 ConfigProvider/Spin/App 进首屏 chunk，其余 rc-* 跟所属路由走。
          //   代价是 antd 代码的缓存粒度变粗（不能跨路由复用整块），但首屏体积正确。
          if (
            id.includes('antd') ||
            id.includes('@ant-design') ||
            id.includes('rc-') ||
            id.includes('@rc-component') ||
            id.includes('@emotion')
          ) {
            return undefined
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
