import { defineConfig, loadEnv } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig(({ mode }) => {
  // 后端代理目标可由 env 覆盖（VITE_API_PROXY_TARGET）。
  // 默认 8000：与 docker-compose.yml 的 backend 端口映射（8000）保持一致；
  // 此前默认 8001，本地 npm run dev 时所有 /api 请求都会 502。
  const env = loadEnv(mode, process.cwd(), '')
  const proxyTarget = env.VITE_API_PROXY_TARGET || 'http://localhost:8000'

  return {
    plugins: [vue()],
    resolve: {
      alias: {
        '@': fileURLToPath(new URL('./src', import.meta.url)),
      },
    },
    server: {
      port: 3000,
      proxy: {
        '/api': {
          target: proxyTarget,
          changeOrigin: true,
        },
      },
    },
    build: {
      outDir: 'dist',
      sourcemap: false,
      chunkSizeWarningLimit: 1500,
      rollupOptions: {
        output: {
          // 函数式分包：只把“实际进入模块图”的 node_modules 文件归组。
          // 注意两点历史教训：
          // 1. 旧的数组写法 manualChunks: { 'element-plus': ['element-plus'] } 会把
          //    包入口整体钉进 chunk，tree-shaking 完全失效（产物 949 kB 全量）。
          // 2. 函数式写法若把桶文件（es/index.mjs 等 re-export 汇总模块）也归入
          //    vendor chunk，该模块成为 chunk 门面后其全部 re-export 被保留，
          //    同样导致整库打包。因此桶文件必须留给 Rollup 自动分包。
          // 函数式分包：只把“实际进入模块图”的 node_modules 文件归组。
          //
          // 【关键约束】element-plus 的聚合模块（barrel/installer）绝不能钉进
          // manualChunks：被显式钉入 chunk 的模块，其顶层语句（makeInstaller /
          // withInstall 调用）会被强制保留，tree-shaking 失效，整库 950 kB 回归。
          // 这些聚合模块留给 Rollup 随引用方自动分包，可按需裁剪。
          // 叶子组件模块（components/<x>/…）不受此影响，可安全归组。
          manualChunks(id) {
            if (!id.includes('node_modules')) return undefined
            if (id.includes('element-plus') || id.includes('@element-plus')) {
              // 聚合/安装器模块排除出 vendor chunk（见上方注释）
              if (
                /element-plus[\\/]es[\\/](index|component|components[\\/]index|defaults|plugin|make-installer)\.mjs$/.test(
                  id,
                )
              ) {
                return undefined
              }
              return 'element-plus'
            }
            if (id.includes('echarts') || id.includes('zrender')) return 'echarts'
            if (
              id.includes('/vue/') ||
              id.includes('/@vue/') ||
              id.includes('vue-router') ||
              id.includes('pinia')
            ) {
              return 'vue'
            }
            return undefined
          },
        },
      },
    },
  }
})
