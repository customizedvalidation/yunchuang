import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { ElMessage } from 'element-plus'
import App from './App.vue'
import router from './router'
// Element Plus 按需注册（含各组件样式），替代原全量 install + 全量 CSS
import { installElementPlus } from '@/plugins/element-plus'
// Element Plus 官方暗色组件级变量（el-button / el-card / el-empty 等在 html.dark 下的细节）。
// 我们的 MDS 暗色块用 html.dark[data-theme='dark'] 更高特异性覆盖 --el-* 为品牌令牌。
import 'element-plus/theme-chalk/dark/css-vars.css'
import './styles/index.css'
import { initTheme } from '@/theme/useTheme'

// 在 mount 之前解析并应用主题，避免 Vue 首帧使用未同步的 ref。
// DOM 上已由 index.html 的同步内联脚本写过一遍，此处为幂等同步 + 注册系统偏好监听。
initTheme()

const app = createApp(App)
const pinia = createPinia()

// 全局未捕获错误兜底：记录日志并提示用户刷新，不吞掉错误。
app.config.errorHandler = (err, _vm, info) => {
  console.error('[global errorHandler]', info, err)
  ElMessage.error('页面发生未预期错误，请刷新')
}

// 未处理的 Promise rejection 兜底：记录日志，不吞掉错误。
window.addEventListener('unhandledrejection', (ev) => {
  console.error('[unhandledrejection]', ev.reason)
})

app.use(pinia)
app.use(router)
installElementPlus(app)

app.mount('#app')
