import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
// Element Plus 按需注册（含各组件样式），替代原全量 install + 全量 CSS
import { installElementPlus } from '@/plugins/element-plus'
import './styles/index.css'

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.use(router)
installElementPlus(app)

app.mount('#app')
