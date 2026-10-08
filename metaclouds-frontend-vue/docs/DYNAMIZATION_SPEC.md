# Metaclouds 前端「全站动态化」改造规范（DYNAMIZATION_SPEC）

> 本文档是把已完成并验证的 Dashboard.vue 动态化模式（数值滚动 / 可点击卡片 / 自动轮询）
> 复用至其余 13 个业务菜单页面的统一执行规范。所有改造必须严格遵守本文档与
> Dashboard.vue 参考实现，任何偏离都必须在上交时逐条说明理由。

## 0. 落地记录（2026-10-08）

- 13 个业务菜单页全部完成动态化复用：GPUManagement / MonitoringAlert / AccelerationSuiteManagement / ResourceManagement / SecurityManagement（统计卡片类：CountUp + 可点击卡片 + 自动轮询）；JobManagement / ClusterManagement / PartitionManagement / SchedulerManagement / TopologyManagement / DatasetManagement / K8SManagement / MultiTenantManagement（表格类：30s 自动轮询 + 有统计则 CountUp）。
- 新增 5 个最小行为测试（AccelerationSuiteManagement / GPUManagement / MonitoringAlert / ResourceManagement / SecurityManagement 各 1 个 spec）。
- 门禁：`npm run type-check` 零报错；`npm run test` 10 文件 / 48 用例全绿（基线 33 + 新增 15）；`npm run build` 成功。
- 浏览器实测：/gpus 与 /monitoring 的 CountUp 数值滚动、30s 倒计时走秒、<10s 呼吸灯、卡片点击跳转（/monitoring → /cluster）均生效。
- 既有测试 JobManagement.spec.ts / Login.spec.ts / Dashboard.spec.ts 全部保持通过。

## 1. 任务范围

| 类别 | 页面 | 主改造 |
|---|---|---|
| 统计卡片类 | GPUManagement.vue、MonitoringAlert.vue、AccelerationSuiteManagement.vue、ResourceManagement.vue、SecurityManagement.vue | CountUp 数字滚动 + 可点击卡片 RouterLink（+ 若含异步表格/统计则加自动轮询） |
| 表格/多 Tab 类 | JobManagement.vue、ClusterManagement.vue、PartitionManagement.vue、SchedulerManagement.vue、TopologyManagement.vue、DatasetManagement.vue、K8SManagement.vue、MultiTenantManagement.vue | 自动轮询（30s 倒计时 + visibilitychange 暂停/恢复）+ 有统计则加 CountUp |

不修改：Dashboard.vue、Login.vue、src/router/index.ts、src/styles/index.css、src/components/CountUp.vue。
不改依赖、不改主题色（科技蓝不变）、不做 git 提交（由组织者统一验证后一次性提交）。

## 2. 必读参考文件（绝对路径）

- 模式基准（最重要）：`D:\YCYD\metaclouds-frontend-vue\src\pages\Dashboard.vue`
- 数字滚动组件：`D:\YCYD\metaclouds-frontend-vue\src\components\CountUp.vue`
- 路由表（跳转目标唯一来源）：`D:\YCYD\metaclouds-frontend-vue\src\router\index.ts`
- CSS 变量表：`D:\YCYD\metaclouds-frontend-vue\src\styles\index.css`
- 测试写法参考：`D:\YCYD\metaclouds-frontend-vue\src\pages\__tests__\Dashboard.spec.ts`
- 数据获取 composable：`D:\YCYD\metaclouds-frontend-vue\src\utils\useFetch.ts`（返回 `{ data, loading, error, execute, refetch }`）

## 3. 模式 A：CountUp 数字滚动

```ts
import CountUp from '@/components/CountUp.vue'
```

模板中替换统计数字：

```html
<!-- 整数统计 -->
<CountUp :value="totalCount" />
<!-- 小数统计（如百分比保留 1 位） -->
<CountUp :value="utilization" :decimals="1" />
```

组件 props：`value: number`、`duration?: number`（默认 1200ms）、`decimals?: number`（默认 0）。
首次挂载从 0 滚动到初始值；后续 value 变化从旧值平滑过渡。

**测试环境注意（重要）**：vitest 的 jsdom 环境 `pretendToBeVisual=true`，`requestAnimationFrame`
**存在**，因此 CountUp 在测试中会真实执行 1200ms 动画、不直接落终值。
→ **测试绝不断言 CountUp 渲染出的数字文本**（会处于动画中间态），只断言标题、RouterLink 目标、aria-label 等结构性内容（见第 8 节）。
→ **现有测试断言的统计文本不得包进 CountUp**。典型：JobManagement.spec.ts 断言 `'共 5 个作业'`，
JobManagement.vue 中该统计行必须保持普通文本、禁止 CountUp 化。

## 4. 模式 B：可点击统计卡片（RouterLink）

仅对「统计卡片」使用；卡片点击后应能跳转到路由表中与之主题匹配的目标。
若匹配目标即当前页面自身（自链），则不包裹 RouterLink，保持静态卡片（仍加 CountUp），并在上交说明中列出。

```html
<RouterLink
  :to="'/job'"
  class="stat-card stat-card--link"
  :aria-label="`运行中作业，点击查看详情`"
>
  <!-- 保持卡片原有内部结构（label / value / suffix / hint） -->
  <div class="stat-card-label">{{ title }}</div>
  <div class="stat-card-value">
    <CountUp :value="value" />
    <span class="stat-card-suffix">{{ suffix }}</span>
  </div>
  <!-- 悬停箭头（绝对定位，hover 显现） -->
  <el-icon class="stat-go" :size="14"><ArrowRight /></el-icon>
</RouterLink>
```

需要 `import { ArrowRight } from '@element-plus/icons-vue'`。

配套 CSS（按页面既有类名微调；保证可点击态一致）：

```css
.stat-card--link {
  position: relative;
  border: 1px solid transparent;
  text-decoration: none;
  transition: border-color 0.2s ease, box-shadow 0.2s ease, transform 0.2s ease;
  outline-offset: 2px;
}
.stat-card--link:hover,
.stat-card--link:focus-visible {
  border-color: var(--mc-brand);
  box-shadow: 0 8px 24px rgba(24, 110, 255, 0.14);
  transform: translateY(-2px);
}
.stat-card--link:active { transform: translateY(0); }
.stat-go {
  position: absolute; right: 10px; top: 50%; transform: translateY(-50%);
  color: var(--mc-text-3); opacity: 0;
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.stat-card--link:hover .stat-go,
.stat-card--link:focus-visible .stat-go {
  opacity: 1; transform: translateY(-50%) translateX(2px); color: var(--mc-brand);
}
```

注意：若页面卡片原来就是 `<div>`/`<el-card>`，换成 `RouterLink` 后需保证原有布局（flex 对齐等）
不破坏；`el-card` 内部结构不适合直接换标签时，可将整卡包一层 `RouterLink`（`display:block`）或
只对可跳转的统计卡改用 RouterLink 元素并复制其原有样式类。

### 路由跳转目标表（唯一来源，来自 src/router/index.ts）

| 主题 | 目标 | 主题 | 目标 |
|---|---|---|---|
| 作业 | `/job` | 调度器 | `/schedulers` |
| GPU | `/gpus` | 资源 | `/resource` |
| 分区 | `/partitions` | 租户 | `/tenant` |
| 集群 | `/cluster` | 加速 | `/acceleration` |
| 监控 | `/monitoring` | 数据集 | `/datasets` |
| 安全 | `/security` | 拓扑 | `/topology` |

（`/k8s` 不在映射表内，统计卡片不要链到 `/k8s`。）

## 5. 模式 C：自动轮询（30s + 倒计时 + visibilitychange）

适用判定：页面主数据通过异步请求加载（表格或统计）且存在可复用的 refetch 机制 → 加。
纯静态、无异步数据源的页面不加，并在上交说明中说明。

脚本块（复制 Dashboard 语义，按页面 refetch 命名适配；必须 onBeforeUnmount 清理）：

```ts
// ---------- 自动轮询（动态呈现：数据按周期自动刷新） ----------
const AUTO_REFRESH_SECONDS = 30
const autoRefreshCountdown = ref(AUTO_REFRESH_SECONDS)
let countdownTimer: number | null = null

function stopAutoRefresh() {
  if (countdownTimer !== null) {
    window.clearInterval(countdownTimer)
    countdownTimer = null
  }
}

function startAutoRefresh() {
  stopAutoRefresh()
  autoRefreshCountdown.value = AUTO_REFRESH_SECONDS
  countdownTimer = window.setInterval(() => {
    autoRefreshCountdown.value -= 1
    if (autoRefreshCountdown.value <= 0) {
      autoRefreshCountdown.value = AUTO_REFRESH_SECONDS
      // 页面不可见时暂停拉取，回到前台后立即补一次刷新
      if (document.visibilityState === 'visible') refetch()
    }
  }, 1000)
}

function onVisibilityChange() {
  if (document.visibilityState === 'visible') {
    // 回到前台：立即刷新一次并重启周期
    refetch()
    startAutoRefresh()
  } else {
    stopAutoRefresh()
  }
}

onMounted(() => {
  document.addEventListener('visibilitychange', onVisibilityChange)
  startAutoRefresh()
})

onBeforeUnmount(() => {
  stopAutoRefresh()
  document.removeEventListener('visibilitychange', onVisibilityChange)
})
```

要求：
- **复用页面已有的 refetch**（useFetch 的 `refetch` 或页面既有刷新函数）；若页面已有手动刷新按钮，
  保留按钮并让自动轮询调用同一函数。
- refetch 触发处（含手动刷新按钮、PageState @retry）重置倒计时：`autoRefreshCountdown.value = AUTO_REFRESH_SECONDS`（参考 Dashboard 的 refetchAll）。
- 若页面已有 `onMounted`/`onBeforeUnmount` 生命周期，把新逻辑并入现有生命周期，不要重复注册。
- 多 Tab 页面：轮询刷新「当前 Tab 的主数据」（参考页面现有 Tab 切换逻辑；若各 Tab 数据来自同一 refetch 则刷新主数据即可）。

页头提示（放在页面标题右侧的工具区；无工具区则新建 `.mc-page-head-extra`）：

```html
<span class="auto-refresh-hint" :class="{ 'is-counting': autoRefreshCountdown < 10 }">
  <i class="auto-refresh-dot" />
  自动刷新 · 下次 {{ autoRefreshCountdown }}s
</span>
```

配套 CSS（scoped 内自包含，重复定义无害）：

```css
.auto-refresh-hint {
  display: inline-flex; align-items: center; gap: 6px;
  font-size: 12px; color: var(--mc-text-3); margin-right: 8px;
  transition: color 0.3s ease;
}
.auto-refresh-hint.is-counting .auto-refresh-dot { animation: mc-pulse 1s ease-in-out infinite; }
.auto-refresh-dot { width: 7px; height: 7px; border-radius: 50%; background: var(--mc-brand); opacity: 0.85; }
@keyframes mc-pulse {
  0%, 100% { opacity: 0.35; transform: scale(0.85); }
  50% { opacity: 1; transform: scale(1.15); }
}
```

## 6. CSS 变量白名单（已核实存在于 src/styles/index.css，只可用这些）

`--mc-brand` `--mc-brand-600` `--mc-brand-100` `--mc-brand-grad` `--mc-brand-fg`
`--mc-text-1` `--mc-text-2` `--mc-text-3` `--mc-line` `--mc-line-strong`
`--mc-surface` `--mc-surface-2` `--mc-surface-3` `--mc-bg`
`--mc-success` `--mc-success-fg` `--mc-success-soft` `--mc-warning` `--mc-warning-fg` `--mc-warning-soft`
`--mc-danger` `--mc-danger-fg` `--mc-danger-soft` `--mc-info` `--mc-info-fg` `--mc-info-soft` `--mc-teal`
`--mc-radius-xs/sm/md/lg/xl/pill` `--mc-shadow-raised-sm` `--mc-shadow-raised`
`--mc-gap` `--mc-gutter` `--mc-fs-h1/h2/body/sm` `--mc-ease` `--mc-t-fast/base/slow` `--mc-focus-ring`

**不存在**：`--mc-brand-hover`、`--mc-text-4`。禁止使用未列出的变量。

## 7. 测试约定（新增最小行为测试，仅统计卡片类页面）

为每个被改造的统计卡片类页面新增一个 spec（表格类页面不加新测试，仅保证既有测试通过）。
参考 `Dashboard.spec.ts`：

1. `vi.mock('@/api', () => ({ ... }))` —— **mock 必须覆盖该页面从 `@/api` 导入的全部导出**（逐项对照页面 import）。
2. mount 参数：`global: { plugins: [createPinia()], stubs: { RouterLink: RouterLinkStub } }`（`@vue/test-utils` 的 RouterLinkStub）。
   若页面依赖 store，检查是否需要设置 localStorage（参考 JobManagement.spec.ts 的 admin 登录）。
3. mock 返回数据后 `await flushPromises()`（必要时两次）。
4. 断言：页面标题、统计卡片标题渲染、RouterLinkStub 的 `to` 目标、`aria-label` 含「点击查看详情」、
   自动刷新提示 `'自动刷新'` 文本。**不断言 CountUp 数字**（见第 3 节）。
5. 测试文件路径：`src/pages/__tests__/<PageName>.spec.ts`。

## 8. 硬约束（每个改造者必须遵守）

1. 只修改分配给自己的页面文件；只读 Dashboard.vue / CountUp.vue / router / styles / 测试。
2. 不 git add / commit / push（避免并行冲突）；不做任何删除或移动文件。
3. 不引入新依赖；不修改 package.json / vite / vitest / tsconfig / 全局样式。
4. 不动 Dashboard.vue、Login.vue。
5. 自动轮询必须 onBeforeUnmount 清理定时器与 visibilitychange 监听。
6. 表格页已有 refetch/刷新按钮 → 保留并复用同一函数。
7. 改造必须真实生效（CountUp 真替换数字、RouterLink 真包裹卡片、倒计时真走秒），禁止占位。
8. **不要运行全项目门禁**（`npm run type-check` / `npm run test` / `npm run build`）——并行改造期间
   全项目检查会因他人文件中间态而误报；门禁由组织者统一在全部完成后运行。
   允许运行：单文件 vitest（`npx vitest run src/pages/__tests__/<自己新增或受托的 spec>.ts`）。
9. 完成后自检：逐文件重新通读改动，确认模板结构闭合、import 无遗漏（ArrowRight、CountUp、onBeforeUnmount/onMounted/ref）、CSS 只引用白名单变量。

## 9. 上交格式（每个改造者返回给组织者）

逐页面给出：
- 页面绝对路径
- 改动点清单（CountUp 替换处 / RouterLink 包裹的卡片与目标 / 自动轮询接入的 refetch）
- 未加 RouterLink 的卡片及理由（自链或无匹配目标）
- 未加自动轮询的页面及理由（无异步数据源）
- 新增/运行的测试文件路径与结果
- 自检结论（模板闭合、无新依赖、变量白名单合规）
