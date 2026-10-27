import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { ElMessage } from 'element-plus'
import App from './App.vue'
import router from './router'
// Element Plus 按需注册（含各组件样式），替代原全量 install + 全量 CSS
import { installElementPlus } from '@/plugins/element-plus'
import './styles/index.css'

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
