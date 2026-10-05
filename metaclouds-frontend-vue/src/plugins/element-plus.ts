/**
 * Element Plus 按需注册（唯一注册入口）。
 *
 * 背景：此前 main.ts 全量 `app.use(ElementPlus)` + `element-plus/dist/index.css`，
 * 构建产物 element-plus chunk 949 kB（gzip 304 kB）、CSS 370 kB（gzip 51 kB），
 * 其中大部分组件从未使用。
 *
 * 【必须深路径导入的原因】包级桶文件 `element-plus/es/index.mjs` 会通过
 * `defaults.mjs → component.mjs`（makeInstaller 全组件数组）把**全部**组件
 * 拉进模块图；此时 vite.config 的 manualChunks 一旦把这些模块钉进 vendor
 * chunk，被钉模块的导出会被强制保留，tree-shaking 完全失效（产物退回全量）。
 * 因此本文件与所有业务代码一律使用 `element-plus/es/components/<x>/index`
 * 深路径（unplugin-element-plus 同款策略），桶文件永不进图。
 * 类型导入（import type）编译期擦除，可继续从包名引入。
 *
 * 维护约定：新增 el-* 组件 / ElXxx API / v-loading 等指令时，必须同步本文件，
 * 否则运行时组件解析不到（测试 setup 也走本入口，可提前暴露遗漏）。
 */
import type { App, Component } from 'vue'
import { ElAlert } from 'element-plus/es/components/alert/index'
import { ElAvatar } from 'element-plus/es/components/avatar/index'
import { ElBreadcrumb, ElBreadcrumbItem } from 'element-plus/es/components/breadcrumb/index'
import { ElButton } from 'element-plus/es/components/button/index'
import { ElCard } from 'element-plus/es/components/card/index'
import { ElCol } from 'element-plus/es/components/col/index'
import { ElConfigProvider } from 'element-plus/es/components/config-provider/index'
import { ElDescriptions, ElDescriptionsItem } from 'element-plus/es/components/descriptions/index'
import { ElDialog } from 'element-plus/es/components/dialog/index'
import { ElDivider } from 'element-plus/es/components/divider/index'
import { ElDrawer } from 'element-plus/es/components/drawer/index'
import { ElDropdown, ElDropdownItem, ElDropdownMenu } from 'element-plus/es/components/dropdown/index'
import { ElEmpty } from 'element-plus/es/components/empty/index'
import { ElForm, ElFormItem } from 'element-plus/es/components/form/index'
import { ElIcon } from 'element-plus/es/components/icon/index'
import { ElInput } from 'element-plus/es/components/input/index'
import { ElInputNumber } from 'element-plus/es/components/input-number/index'
import { ElLink } from 'element-plus/es/components/link/index'
import { ElLoading } from 'element-plus/es/components/loading/index'
import { ElPagination } from 'element-plus/es/components/pagination/index'
import { ElPopconfirm } from 'element-plus/es/components/popconfirm/index'
import { ElProgress } from 'element-plus/es/components/progress/index'
import { ElRadioButton, ElRadioGroup } from 'element-plus/es/components/radio/index'
import { ElRow } from 'element-plus/es/components/row/index'
import { ElOption, ElSelect } from 'element-plus/es/components/select/index'
import { ElSlider } from 'element-plus/es/components/slider/index'
import { ElSwitch } from 'element-plus/es/components/switch/index'
import { ElTabPane, ElTabs } from 'element-plus/es/components/tabs/index'
import { ElTable, ElTableColumn } from 'element-plus/es/components/table/index'
import { ElTag } from 'element-plus/es/components/tag/index'
import { ElTooltip } from 'element-plus/es/components/tooltip/index'

// ---- 按需样式（顺序无要求；base 样式由各组件样式自动引入）----
import 'element-plus/es/components/alert/style/css'
import 'element-plus/es/components/avatar/style/css'
import 'element-plus/es/components/breadcrumb/style/css'
import 'element-plus/es/components/button/style/css'
import 'element-plus/es/components/card/style/css'
import 'element-plus/es/components/col/style/css'
import 'element-plus/es/components/row/style/css'
import 'element-plus/es/components/descriptions/style/css'
import 'element-plus/es/components/dialog/style/css'
import 'element-plus/es/components/divider/style/css'
import 'element-plus/es/components/drawer/style/css'
import 'element-plus/es/components/dropdown/style/css'
import 'element-plus/es/components/empty/style/css'
import 'element-plus/es/components/form/style/css'
import 'element-plus/es/components/icon/style/css'
import 'element-plus/es/components/input/style/css'
import 'element-plus/es/components/input-number/style/css'
import 'element-plus/es/components/link/style/css'
import 'element-plus/es/components/select/style/css'
import 'element-plus/es/components/option/style/css'
import 'element-plus/es/components/pagination/style/css'
import 'element-plus/es/components/popconfirm/style/css'
import 'element-plus/es/components/progress/style/css'
import 'element-plus/es/components/radio/style/css'
import 'element-plus/es/components/slider/style/css'
import 'element-plus/es/components/switch/style/css'
import 'element-plus/es/components/tabs/style/css'
import 'element-plus/es/components/table/style/css'
import 'element-plus/es/components/tag/style/css'
import 'element-plus/es/components/tooltip/style/css'
// ElMessage / ElMessageBox 为 API 调用组件，样式单独引入
import 'element-plus/es/components/message/style/css'
import 'element-plus/es/components/message-box/style/css'
// v-loading 指令与 dialog/drawer 遮罩依赖
import 'element-plus/es/components/loading/style/css'
import 'element-plus/es/components/overlay/style/css'

/** 全局注册的组件（子组件如 el-form-item 随父组件样式，但注册不可少） */
const components: Component[] = [
  ElAlert,
  ElAvatar,
  ElBreadcrumb,
  ElBreadcrumbItem,
  ElButton,
  ElCard,
  ElCol,
  ElConfigProvider,
  ElDescriptions,
  ElDescriptionsItem,
  ElDialog,
  ElDivider,
  ElDrawer,
  ElDropdown,
  ElDropdownItem,
  ElDropdownMenu,
  ElEmpty,
  ElForm,
  ElFormItem,
  ElIcon,
  ElInput,
  ElInputNumber,
  ElLink,
  ElOption,
  ElPagination,
  ElPopconfirm,
  ElProgress,
  ElRadioButton,
  ElRadioGroup,
  ElRow,
  ElSelect,
  ElSlider,
  ElSwitch,
  ElTabPane,
  ElTable,
  ElTableColumn,
  ElTabs,
  ElTag,
  ElTooltip,
]

/**
 * 安装 Element Plus 按需集。同时作为 Vue 插件入口：
 * main.ts 与 vitest 的 config.global.plugins 共用，保证测试与生产注册一致。
 */
export function installElementPlus(app: App): void {
  // v-loading 指令（ElLoading.install 注册 directive 与 $loading 服务）
  app.use(ElLoading)
  for (const component of components) {
    const name = (component as { name?: string }).name
    if (name) app.component(name, component)
  }
}

export default { install: installElementPlus }
