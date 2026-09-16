# Metaclouds 前端 React → Vue 迁移说明

## 一、迁移概述

| 项目 | 说明 |
|------|------|
| 原前端 | metaclouds-frontend（React 18 + TypeScript + antd 5 + Redux Toolkit Query + ECharts 5） |
| 新前端 | metaclouds-frontend-vue（Vue 3 + TypeScript + Vite + Pinia + Vue Router + Element Plus + ECharts 5） |
| 迁移页面数 | 15 个（全部一次性迁移） |
| 前端定位 | Vue 成为唯一前端，React 源码保留为 git 历史参考，运行入口唯一指向 Vue |
| 后端 | 零改动，API 完全兼容 |

## 二、技术栈对照

| 维度 | React 版 | Vue 版 |
|------|----------|--------|
| 框架 | React 18 | Vue 3（Composition API + `<script setup>`） |
| 构建 | CRA / Vite | Vite 5 |
| 语言 | TypeScript | TypeScript（strict） |
| UI 组件库 | antd 5 | Element Plus 2.8 |
| 状态管理 | Redux Toolkit | Pinia 2 |
| 路由 | React Router 6 | Vue Router 4 |
| 数据请求 | RTK Query | axios + 自定义 useFetch/useMutation composable |
| 图表 | ECharts 5 | ECharts 5（按需引入） |
| 样式 | Tailwind + CSS 变量 | 纯 CSS 变量 + scoped style |
| HTTP 客户端 | fetch（RTK Query 封装） | axios（withCredentials） |

## 三、页面清单

| 序号 | 页面 | 路由 | 文件 | 功能要点 |
|------|------|------|------|----------|
| 1 | 登录 | /login | Login.vue | 科技蓝双栏布局、表单校验、5 次失败锁定 1 分钟 |
| 2 | 仪表盘 | /dashboard | Dashboard.vue | 7 KPI 卡 + 3 ECharts 饼图 + 最近告警列表 |
| 3 | 集群管理 | /cluster | ClusterManagement.vue | CRUD、动态扩容、重建、详情（含分区子表） |
| 4 | 资源管理 | /resource | ResourceManagement.vue | 资源列表、类型筛选、利用率进度条、编辑 |
| 5 | 作业管理 | /job, /job/list, /job/queue, /job/history | JobManagement.vue | 三 tab 切换、CRUD、取消、提交 K8S、详情（含 Checkpoint） |
| 6 | 监控告警 | /monitoring | MonitoringAlert.vue | 指标卡 + ECharts 折线/柱状图 + 告警列表 + 详情抽屉 |
| 7 | 多租户 | /tenant | MultiTenantManagement.vue | 租户 CRUD、配额进度条（仅 admin/manager） |
| 8 | 加速套件 | /acceleration | AccelerationSuiteManagement.vue | 套件列表、编辑、启用/禁用、配置详情 |
| 9 | 安全管理 | /security | SecurityManagement.vue | 策略列表、编辑、启用/禁用、规则详情 |
| 10 | K8S 管理 | /k8s, /k8s/nodes, /k8s/pods, /k8s/services | K8SManagement.vue | 三 tab：节点/Pod/服务管理 |
| 11 | GPU 设备 | /gpus | GPUManagement.vue | 设备 CRUD、分配记录、细粒度分配（1/2、1/4） |
| 12 | 分区管理 | /partitions | PartitionManagement.vue | 分区 CRUD、优先级/最大运行时调整、权限管理 |
| 13 | 调度器 | /schedulers | SchedulerManagement.vue | 调度器集成 CRUD、同步作业、队列/节点查看、健康检查 |
| 14 | 拓扑感知 | /topology | TopologyManagement.vue | 节点拓扑 CRUD、拓扑评分计算 |
| 15 | 数据集 | /datasets | DatasetManagement.vue | 数据集 CRUD、Fluid 缓存管理（启用/禁用/预取） |

## 四、API 对齐情况

### 4.1 端点总数
- React 版 RTK Query 定义端点：60+ 个
- Vue 版 axios API 层：全部对齐，按领域分为 14 个 API 对象

### 4.2 关键机制对齐

| 机制 | React 版 | Vue 版 |
|------|----------|--------|
| 响应信封解包 | RTK Query baseQuery 中解包 `{success, data}` | axios 响应拦截器统一解包 |
| 401 会话失效 | baseQuery 检测 → 清理 localStorage → 跳登录 | axios 响应拦截器检测 → 清理 + 调用 logout → 跳登录 |
| 认证端点 401 豁免 | `AUTH_ENDPOINT_PATTERN` 判定 | 同逻辑，`/auth/*` 的 401 不触发跳转 |
| CSRF 双提交令牌 | `csrfHeaders()` + prepareHeaders | `csrfHeaders()` + axios 请求拦截器 |
| httpOnly Cookie | `credentials: 'include'` | `withCredentials: true` |
| 静默续期 | `useSilentTokenRefresh` hook（5min 间隔，1h 阈值） | App.vue onMounted 启动 setInterval（同参数） |
| RBAC 权限 | `Can` 组件（perm 属性） | `Can` 组件（roles 属性）+ 路由 meta.roles 守卫 |

### 4.3 API 对象清单
`authApi`, `clusterApi`, `resourceApi`, `jobApi`, `monitoringApi`, `tenantApi`, `accelerationApi`, `securityApi`, `gpuApi`, `partitionApi`, `quotaApi`, `schedulerApi`, `topologyApi`, `datasetApi`, `checkpointApi`

## 五、项目结构

```
metaclouds-frontend-vue/
├── index.html
├── package.json
├── vite.config.ts          # Vite 配置 + /api 代理到 :8000
├── tsconfig.json
├── tsconfig.node.json
└── src/
    ├── main.ts             # 入口：Pinia + Router + Element Plus
    ├── App.vue             # 根组件 + 静默续期
    ├── env.d.ts
    ├── api/
    │   ├── http.ts         # axios 实例 + 拦截器（信封解包/401/CSRF）
    │   └── index.ts        # 14 个领域 API 对象
    ├── components/
    │   ├── Layout.vue      # 应用外壳（Sidebar + Topbar + 内容区）
    │   ├── Sidebar.vue     # 侧边栏（6 组菜单 + 徽标 + 科技蓝深色主题）
    │   ├── Topbar.vue      # 顶栏（面包屑 + 用户菜单 + 刷新）
    │   ├── Can.vue         # RBAC 权限组件
    │   └── PageState.vue   # 三态守卫（loading/error/empty）
    ├── router/
    │   └── index.ts        # 路由定义 + 登录守卫 + 角色守卫
    ├── stores/
    │   └── auth.ts         # Pinia auth store（login/logout/silentRefresh）
    ├── styles/
    │   ├── index.css       # 全局主题令牌 + Element Plus 皮肤覆盖
    │   └── sidebar.css     # 侧边栏科技蓝深色主题
    ├── types/
    │   └── index.ts        # 全部领域模型类型
    ├── utils/
    │   ├── auth.ts         # 角色判定 + localStorage 读取
    │   └── useFetch.ts     # useFetch/useMutation composable
    └── pages/              # 15 个页面组件
        ├── Login.vue
        ├── Dashboard.vue
        ├── ClusterManagement.vue
        ├── ResourceManagement.vue
        ├── JobManagement.vue
        ├── MonitoringAlert.vue
        ├── MultiTenantManagement.vue
        ├── AccelerationSuiteManagement.vue
        ├── SecurityManagement.vue
        ├── K8SManagement.vue
        ├── GPUManagement.vue
        ├── PartitionManagement.vue
        ├── SchedulerManagement.vue
        ├── TopologyManagement.vue
        └── DatasetManagement.vue
```

## 六、设计系统

### 6.1 科技蓝主题令牌
- 主色：`#1677ff`（--sidebar-primary-color）
- 深色端：`#0958d9`（--sidebar-primary-dark）
- 浅色端：`#4096ff`（--sidebar-primary-light）
- 强调色：`#69b1ff`（--sidebar-accent-color）
- 侧边栏背景：`#0a1628 → #0e2140` 渐变
- 品牌渐变：`linear-gradient(135deg, #2f6bff 0%, #5b3fd9 100%)`

### 6.2 侧边栏
- 6 组菜单：总览 / 作业调度 / 基础资源 / 集群管理 / 加速套件 / 系统治理
- 子菜单默认全部展开
- 实时数量徽标（基于 API 数据）
- 折叠态：64px 图标轨
- 移动端：浮层 Drawer + 遮罩

### 6.3 页面通用模式
- 页头：`.mc-page-head`（标题 + 描述 + 操作区）
- 三态守卫：`PageState` 组件（loading / error / empty）
- 搜索筛选工具栏：`el-input` + `el-select`
- CRUD：`el-dialog` + `el-form`（layout="vertical"）
- 表格：`el-table` + `el-pagination`
- 状态：`el-tag` 或 `.mc-status` 色点

## 七、验证结果

### 7.1 类型检查
```
vue-tsc --noEmit → 0 errors
```

### 7.2 生产构建
```
vite build → ✓ built in 11.70s
```
- 构建产物：dist/
- 代码分割：按页面懒加载，element-plus / echarts / vue 独立 chunk
- 最大 chunk：echarts 1,035 KB（gzip 343 KB）

### 7.3 后端零回归
- 后端 metaclouds-backend（Go + Gin + GORM，端口 8000）未做任何改动
- API 契约完全兼容

### 7.4 开发服务器
- Vue dev server 运行于端口 3000
- `/api` 代理到 `http://localhost:8000`
- 登录接口验证通过（admin/Admin@123456 → 200 + JWT）

## 八、运行方式

### 开发
```bash
cd metaclouds-frontend-vue
npm install
npm run dev    # http://localhost:3000
```

### 生产构建
```bash
npm run build   # vue-tsc + vite build
npm run preview # 预览构建产物
```

### 环境要求
- Node.js ≥ 18（推荐 22.x）
- 后端服务运行于 `http://localhost:8000`

## 九、skill.md 能力保留对照

| skill.md 能力层 | Vue 版页面对应 | 状态 |
|-----------------|-----------------|------|
| 基础资源层 | ResourceManagement, GPUManagement, PartitionManagement | ✓ 保留 |
| 容器层 | JobManagement, K8SManagement | ✓ 保留 |
| 集群管理层 | ClusterManagement, SchedulerManagement, TopologyManagement | ✓ 保留 |
| 加速套件层 | AccelerationSuiteManagement, DatasetManagement（Fluid） | ✓ 保留 |
| 监控管理安全层 | MonitoringAlert, SecurityManagement, MultiTenantManagement | ✓ 保留 |
| 多 GPU 厂商 | GPUManagement（nvidia/enflame/moore_threads/domestic_x） | ✓ 保留 |
| GPU 细粒度分配 | GPUManagement（fraction 1.0/0.5/0.25 + MIG） | ✓ 保留 |
| 分区管理 | PartitionManagement（优先级/最大运行时/权限） | ✓ 保留 |
| 多维度配额 | PartitionManagement + quotaApi | ✓ 保留 |
| 拓扑感知 | TopologyManagement（机架/交换机/NUMA/NVLink） | ✓ 保留 |
| 弹性/容错训练 | JobManagement（elastic_enabled, checkpoint, max_retries） | ✓ 保留 |
| Fluid 缓存 | DatasetManagement（Alluxio/JindoFS + 预取） | ✓ 保留 |
| 分布式训练 | JobManagement（并行策略/DeepSpeed/Megatron 字段） | ✓ 保留 |
| 推理加速 | AccelerationSuiteManagement（TensorRT/ONNX/量化） | ✓ 保留 |

---

*文档生成时间：2026-09-15*
*迁移版本：metaclouds-frontend-vue v2.0.0*
