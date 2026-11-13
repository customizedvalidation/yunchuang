<template>
  <div class="mc-page">
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">算力总览</h1>
        <p class="mc-page-desc">{{ rangeLabel }} · 实时掌握全平台算力水位、作业吞吐与风险告警</p>
      </div>
      <div class="mc-page-head-extra">
        <span class="auto-refresh-hint" :class="{ 'is-counting': autoRefreshCountdown < 10 }">
          <i class="auto-refresh-dot" />
          自动刷新 · 下次 {{ autoRefreshCountdown }}s
        </span>
        <el-radio-group v-model="range" size="small">
          <el-radio-button value="today">今日</el-radio-button>
          <el-radio-button value="7d">近 7 天</el-radio-button>
          <el-radio-button value="30d">近 30 天</el-radio-button>
        </el-radio-group>
        <el-button :icon="Refresh" @click="refetchAll">刷新</el-button>
      </div>
    </div>

    <PageState
      :loading="anyLoading"
      :error="anyError"
      :data="hasData"
      empty-text="暂无看板数据"
      retry-text="重新加载"
      @retry="refetchAll"
    >
      <!-- KPI 卡片行（可点击跳转 + 数值滚动 + GPU 利用率动态环） -->
      <div class="kpi-grid">
        <RouterLink
          v-for="card in kpiCards"
          :key="card.title"
          :to="card.to"
          class="kpi-card"
          :aria-label="`${card.title}，点击查看详情`"
        >
          <div class="kpi-icon" :style="{ background: card.bg, color: card.iconColor }">
            <el-icon :size="20"><component :is="card.icon" /></el-icon>
          </div>
          <div class="kpi-body">
            <div class="kpi-title">{{ card.title }}</div>
            <div class="kpi-value">
              <CountUp :value="card.value" />
              <span class="kpi-suffix">{{ card.suffix }}</span>
            </div>
            <div class="kpi-footer" :class="{ 'is-danger': card.footerDanger }">{{ card.footer }}</div>
          </div>
          <!-- GPU 利用率动态环形指示器 -->
          <svg
            v-if="card.ring"
            class="kpi-ring"
            viewBox="0 0 44 44"
            aria-hidden="true"
          >
            <circle class="kpi-ring-bg" cx="22" cy="22" :r="RING_RADIUS" />
            <circle
              class="kpi-ring-fg"
              cx="22"
              cy="22"
              :r="RING_RADIUS"
              :stroke-dasharray="RING_CIRC"
              :stroke-dashoffset="ringOffset(card.value)"
            />
            <text x="22" y="26" class="kpi-ring-text">{{ card.value }}%</text>
          </svg>
          <el-icon class="kpi-go" :size="14"><ArrowRight /></el-icon>
        </RouterLink>
      </div>

      <!-- 三个饼图 -->
      <el-row :gutter="16" class="mc-mt">
        <el-col :xs="24" :sm="24" :md="8">
          <el-card shadow="never">
            <template #header>
              <div class="chart-card-head">
                <h3 class="chart-card-title">资源分布</h3>
                <span class="chart-card-head-right">
                  <span class="chart-card-extra">GPU 卡</span>
                  <RouterLink class="chart-card-more" to="/gpus">查看详情 <el-icon :size="12"><ArrowRight /></el-icon></RouterLink>
                </span>
              </div>
            </template>
            <div v-if="gpuTotal > 0" ref="resourceChartRef" class="chart-box" />
            <McEmpty v-else description="暂无资源数据" :size="80" />
          </el-card>
        </el-col>
        <el-col :xs="24" :sm="24" :md="8">
          <el-card shadow="never">
            <template #header>
              <div class="chart-card-head">
                <h3 class="chart-card-title">作业状态分布</h3>
                <span class="chart-card-head-right">
                  <span class="chart-card-extra">共 {{ totalJobs }} 个</span>
                  <RouterLink class="chart-card-more" to="/job">查看详情 <el-icon :size="12"><ArrowRight /></el-icon></RouterLink>
                </span>
              </div>
            </template>
            <div v-if="jobStatusData.length > 0" ref="jobChartRef" class="chart-box" />
            <McEmpty v-else description="还没有作业" :size="80" />
          </el-card>
        </el-col>
        <el-col :xs="24" :sm="24" :md="8">
          <el-card shadow="never">
            <template #header>
              <div class="chart-card-head">
                <h3 class="chart-card-title">GPU 厂商分布</h3>
                <span class="chart-card-head-right">
                  <span class="chart-card-extra">共 {{ gpuDevices.length }} 张</span>
                  <RouterLink class="chart-card-more" to="/gpus">查看详情 <el-icon :size="12"><ArrowRight /></el-icon></RouterLink>
                </span>
              </div>
            </template>
            <div v-if="gpuVendorData.length > 0" ref="vendorChartRef" class="chart-box" />
            <McEmpty v-else description="暂无 GPU 设备" :size="80" />
          </el-card>
        </el-col>
      </el-row>

      <!-- 最近告警 -->
      <el-card shadow="never" class="mc-mt">
        <template #header>
          <div class="chart-card-head">
            <h3 class="chart-card-title">最近告警</h3>
            <span class="chart-card-head-right">
              <span class="chart-card-extra">按严重程度排序</span>
              <RouterLink class="chart-card-more" to="/monitoring">查看全部 <el-icon :size="12"><ArrowRight /></el-icon></RouterLink>
            </span>
          </div>
        </template>
        <McEmpty v-if="alertsData.length === 0" description="暂无告警，系统运行正常" :size="80" />
        <ul v-else class="alert-list">
          <li v-for="item in topAlerts" :key="item.id" class="alert-item" :class="alertLevelClass(item.level)">
            <div class="alert-line">
              <span class="mc-status" :class="alertLevelClass(item.level)">
                <i class="mc-status-dot" />
                {{ alertText(item.level) }}
              </span>
              <span class="alert-message">{{ item.message || item.details || '未知告警' }}</span>
            </div>
            <div class="alert-detail">{{ item.details || item.message || '暂无详情' }}</div>
          </li>
        </ul>
      </el-card>
    </PageState>
  </div>
</template>

<script setup lang="ts">
import McEmpty from '@/components/McEmpty.vue'
import { useAutoRefresh } from '@/composables/useAutoRefresh'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { Refresh, Coin, Cloudy, Timer, Bell, UserFilled, Grid, Operation, ArrowRight } from '@element-plus/icons-vue'
import PageState from '@/components/PageState.vue'
import CountUp from '@/components/CountUp.vue'
// 经 @/theme/echarts 统一入口引入（已按需 use() 注册，勿直接引 'echarts' 全量包）
import {
  registerMcThemes,
  chartThemeName,
  onChartThemeChange,
  activeColorTokens,
  activePalette,
  palette,
  echarts,
} from '@/theme/echarts'
import type { ECharts } from 'echarts/core'

// 注册 light / dark 两套 MDS 图表主题（幂等）；实例主题名由 chartThemeName 决定。
// 测试环境 echarts 被 mock 时 registerTheme 可能缺失，模块内已静默降级。
registerMcThemes()
import {
  clusterApi,
  resourceApi,
  jobApi,
  monitoringApi,
  gpuApi,
  partitionApi,
  schedulerApi,
  tenantApi,
} from '@/api'
import { useFetch } from '@/utils/useFetch'
import type { Cluster, Resource, Job, Alert, GPUDevice, Partition, SchedulerIntegration, Tenant, GPUVendor, MetricsOverview } from '@/types'

// ---------- 数据加载 ----------
const clusters = useFetch<Cluster[]>(() => clusterApi.list())
const resources = useFetch<Resource[]>(() => resourceApi.list())
const jobs = useFetch<Job[]>(() => jobApi.list())
const alerts = useFetch<Alert[]>(() => monitoringApi.alerts())
const dashboardF = useFetch<MetricsOverview>(() => monitoringApi.dashboard())
const gpuDevicesF = useFetch<GPUDevice[]>(() => gpuApi.devices({ page_size: 1000 }))
const partitionsF = useFetch<Partition[]>(() => partitionApi.list({}))
const schedulersF = useFetch<SchedulerIntegration[]>(() => schedulerApi.list())
const tenantsF = useFetch<Tenant[]>(() => tenantApi.list())

const clustersData = computed(() => clusters.data.value ?? [])
const resourcesData = computed(() => resources.data.value ?? [])
const jobsData = computed(() => jobs.data.value ?? [])
const alertsData = computed(() => alerts.data.value ?? [])
const dashboardStats = computed(() => dashboardF.data.value ?? {})
const gpuDevices = computed(() => gpuDevicesF.data.value ?? [])
const partitions = computed(() => partitionsF.data.value ?? [])
const schedulers = computed(() => schedulersF.data.value ?? [])
const tenants = computed(() => tenantsF.data.value ?? [])

// 全部 9 个数据源任一 pending 即视为 loading，任一报错即展示错误，
// 避免 5 个次要数据源（dashboardF/gpuDevicesF/partitionsF/schedulersF/tenantsF）
// 仍在加载时 anyLoading 已 false、用户看到半成品 0 值。
const allFetchers = [clusters, resources, jobs, alerts, dashboardF, gpuDevicesF, partitionsF, schedulersF, tenantsF]
const anyLoading = computed(() => allFetchers.some((f) => f.loading.value))
const anyError = computed(() => allFetchers.map((f) => f.error.value).find(Boolean) ?? '')
const hasData = computed(
  () => clustersData.value.length + resourcesData.value.length + jobsData.value.length + alertsData.value.length > 0,
)

function refetchAll() {
  clusters.refetch()
  resources.refetch()
  jobs.refetch()
  alerts.refetch()
  dashboardF.refetch()
  gpuDevicesF.refetch()
  partitionsF.refetch()
  schedulersF.refetch()
  tenantsF.refetch()
  // 手动/自动刷新均重置自动刷新倒计时
  resetCountdown()
}

// ---------- 自动轮询（动态呈现：数据按周期自动刷新） ----------
// 倒计时 / setInterval / visibilitychange / 生命周期清理统一收口到 useAutoRefresh。
const { autoRefreshCountdown, resetCountdown } = useAutoRefresh(refetchAll)




// ---------- 时间范围 ----------
const range = ref<'today' | '7d' | '30d'>('7d')
const rangeLabel = computed(
  () => ({ today: '今日', '7d': '近 7 天', '30d': '近 30 天' })[range.value],
)

// ---------- KPI 计算 ----------
// GPU 利用率以 monitoring/dashboard 聚合结果为准：
// total_gpus = gpu_devices 总数，allocated_gpus = gpu_allocations 占用数。
// （不再从 resources 表聚合——该表为资源池定义，初始为空会导致 KPI 恒为 0。）
const gpuTotal = computed(() => Number(dashboardStats.value.total_gpus) || 0)
const gpuUsed = computed(() => Number(dashboardStats.value.allocated_gpus) || 0)
const utilization = computed(() =>
  gpuTotal.value > 0 ? Math.round((gpuUsed.value / gpuTotal.value) * 100) : 0,
)
const runningJobs = computed(() => jobsData.value.filter((j) => j.status === 'running').length)
const pendingJobs = computed(() => jobsData.value.filter((j) => j.status === 'pending').length)
const totalJobs = computed(() => jobsData.value.length)
const alertCount = computed(() => alertsData.value.length)

const partitionStats = computed(() => ({
  total: partitions.value.length,
  active: partitions.value.filter((p) => p.status === 'active').length,
  maintenance: partitions.value.filter((p) => p.status === 'maintenance').length,
}))
const schedulerStats = computed(() => ({
  total: schedulers.value.length,
  active: schedulers.value.filter((s) => s.status === 'active').length,
  error: schedulers.value.filter((s) => s.status === 'error').length,
}))

const kpiCards = computed(() => [
  {
    title: '集群数量',
    value: clustersData.value.length,
    suffix: ' 个',
    icon: Coin,
    bg: 'var(--mc-brand-subtle)',
    iconColor: 'var(--mc-brand-fg)',
    footer: '统一纳管的计算集群',
    to: '/cluster',
  },
  {
    title: 'GPU 利用率',
    value: utilization.value,
    suffix: '%',
    icon: Cloudy,
    bg: 'var(--mc-brand-subtle)',
    iconColor: 'var(--mc-brand-fg)',
    footer: `已用 ${gpuUsed.value} / 共 ${gpuTotal.value}`,
    to: '/gpus',
    ring: true,
  },
  {
    title: '运行中作业',
    value: runningJobs.value,
    suffix: ` / ${totalJobs.value}`,
    icon: Timer,
    bg: 'var(--mc-brand-subtle)',
    iconColor: 'var(--mc-brand-fg)',
    footer: `排队中 ${pendingJobs.value} 个`,
    to: '/job',
  },
  {
    title: '活跃告警',
    value: alertCount.value,
    suffix: ' 条',
    icon: Bell,
    bg: alertCount.value > 0 ? 'var(--mc-danger-soft)' : 'var(--mc-brand-subtle)',
    iconColor: alertCount.value > 0 ? 'var(--mc-danger-fg)' : 'var(--mc-brand-fg)',
    footer: alertCount.value > 0 ? '需要关注' : '运行正常',
    footerDanger: alertCount.value > 0,
    to: '/monitoring',
  },
  {
    title: '租户数量',
    value: tenants.value.length,
    suffix: ' 个',
    icon: UserFilled,
    bg: 'var(--mc-brand-subtle)',
    iconColor: 'var(--mc-brand-fg)',
    footer: '多租户资源隔离与配额',
    to: '/tenant',
  },
  {
    title: '分区状态',
    value: partitionStats.value.total,
    suffix: ' 个',
    icon: Grid,
    bg: 'var(--mc-brand-subtle)',
    iconColor: 'var(--mc-brand-fg)',
    footer: `活跃 ${partitionStats.value.active} · 维护 ${partitionStats.value.maintenance}`,
    to: '/partitions',
  },
  {
    title: '调度器集成',
    value: schedulerStats.value.total,
    suffix: ' 个',
    icon: Operation,
    bg: 'var(--mc-brand-subtle)',
    iconColor: 'var(--mc-brand-fg)',
    footer: `活跃 ${schedulerStats.value.active}${schedulerStats.value.error > 0 ? ` · 异常 ${schedulerStats.value.error}` : ''}`,
    to: '/schedulers',
  },
])

// ---------- GPU 利用率动态环 ----------
const RING_RADIUS = 18
const RING_CIRC = 2 * Math.PI * RING_RADIUS
function ringOffset(value: number) {
  return RING_CIRC * (1 - Math.min(Math.max(value, 0), 100) / 100)
}

// ---------- 图表 ----------
const VENDOR_LABEL: Record<string, string> = {
  nvidia: 'NVIDIA',
  enflame: '燧原',
  moore_threads: '摩尔线程',
  domestic_x: '国产X',
}
// 厂商分类色取自 MDS chartPalette（饼图色块，非文字）
const VENDOR_COLOR: Record<string, string> = {
  nvidia: palette[0],
  enflame: palette[1],
  moore_threads: palette[6],
  domestic_x: palette[3],
}
const STATUS_TEXT: Record<string, string> = {
  running: '运行中',
  pending: '排队中',
  completed: '已完成',
  failed: '失败',
  cancelled: '已取消',
  active: '活跃',
  maintenance: '维护',
  info: '信息',
  warning: '警告',
  error: '错误',
  critical: '严重',
}
// 状态色取自 MDS chartPalette（饼图色块）
const STATUS_COLOR: Record<string, string> = {
  running: palette[0],
  pending: palette[3],
  completed: palette[2],
  failed: palette[4],
  cancelled: palette[9],
  active: palette[0],
  maintenance: palette[3],
  info: palette[6],
  warning: palette[3],
  error: palette[4],
  critical: palette[4],
}

const jobStatusData = computed(() => {
  const groups: Record<string, number> = {}
  jobsData.value.forEach((j) => {
    const key = j.status ?? 'unknown'
    groups[key] = (groups[key] ?? 0) + 1
  })
  return Object.entries(groups).map(([status, value]) => ({
    value,
    name: STATUS_TEXT[status] ?? status,
    itemStyle: { color: STATUS_COLOR[status] ?? palette[9] },
  }))
})

const gpuVendorData = computed(() => {
  const groups: Record<string, number> = {}
  gpuDevices.value.forEach((d) => {
    const key = (d.vendor ?? 'unknown') as GPUVendor | 'unknown'
    groups[key] = (groups[key] ?? 0) + 1
  })
  return Object.entries(groups).map(([vendor, value]) => ({
    value,
    name: VENDOR_LABEL[vendor] ?? vendor,
    itemStyle: { color: VENDOR_COLOR[vendor] ?? palette[9] },
  }))
})

function pieOption(data: unknown[], tooltipFmt: string) {
  return {
    tooltip: { trigger: 'item' as const, formatter: tooltipFmt },
    legend: {
      bottom: 0,
      type: 'scroll' as const,
      itemWidth: 10,
      itemHeight: 10,
      itemGap: 12,
      textStyle: { color: activeColorTokens.value.neutral.text2, fontSize: 11 },
    },
    series: [
      {
        type: 'pie' as const,
        radius: ['54%', '76%'],
        center: ['50%', '45%'],
        avoidLabelOverlap: true,
        itemStyle: { borderRadius: 6, borderColor: activeColorTokens.value.neutral.surface, borderWidth: 2 },
        // v3：关闭常驻外部百分比标签。GPU 厂商有 9 类，3%~5% 的小占比标签会
        // 互相重叠糊成一团；改为「图例 + tooltip + hover 时环心显示详情」，
        // 信息不丢失，画面也干净。
        label: { show: false },
        labelLine: { show: false },
        emphasis: {
          scale: true,
          scaleSize: 4,
          label: {
            show: true,
            position: 'center' as const,
            fontSize: 13,
            fontWeight: 600,
            lineHeight: 18,
            color: activeColorTokens.value.neutral.text1,
            formatter: '{b}\n{d}%',
          },
        },
        data,
      },
    ],
  }
}

const resourceOption = computed(() =>
  pieOption(
    [
      { value: gpuUsed.value, name: '已使用', itemStyle: { color: activePalette.value[0] } },
      { value: Math.max(gpuTotal.value - gpuUsed.value, 0), name: '可分配', itemStyle: { color: activeColorTokens.value.neutral.line } },
    ],
    '{b}: {c} ({d}%)',
  ),
)
const jobOption = computed(() => pieOption(jobStatusData.value, '{b}: {c} ({d}%)'))
const vendorOption = computed(() => pieOption(gpuVendorData.value, '{b}: {c} 张 ({d}%)'))

const resourceChartRef = ref<HTMLElement>()
const jobChartRef = ref<HTMLElement>()
const vendorChartRef = ref<HTMLElement>()
let resourceChart: ECharts | null = null
let jobChart: ECharts | null = null
let vendorChart: ECharts | null = null

let resizeObserver: ResizeObserver | null = null
let stopThemeWatch: (() => void) | null = null

function resizeCharts() {
  resourceChart?.resize()
  jobChart?.resize()
  vendorChart?.resize()
}

// 统一在容器真实挂载（v-if 已渲染、ref 已 attach）后再 init；
// 之后的数据更新仅 setOption，避免重复 init。修复此前 watch 为 pre-flush、
// v-if 容器尚未挂载导致 ref 为空、echarts 永不初始化的时序竞争。
function syncCharts() {
  if (resourceChartRef.value && !resourceChart) {
    resourceChart = echarts.init(resourceChartRef.value, chartThemeName.value)
  }
  if (resourceChart) resourceChart.setOption(resourceOption.value, true)

  if (jobChartRef.value && jobStatusData.value.length > 0 && !jobChart) {
    jobChart = echarts.init(jobChartRef.value, chartThemeName.value)
  }
  if (jobChart) jobChart.setOption(jobOption.value, true)

  if (vendorChartRef.value && gpuVendorData.value.length > 0 && !vendorChart) {
    vendorChart = echarts.init(vendorChartRef.value, chartThemeName.value)
  }
  if (vendorChart) vendorChart.setOption(vendorOption.value, true)
}

async function syncChartsAfterTick() {
  await nextTick()
  syncCharts()
}

/**
 * 主题切换时 ECharts 实例不会热更新（init 已把主题色固化），必须整体重建：
 * dispose → 置 null → 重新走 syncCharts() 用 chartThemeName.value 初始化。
 * option 里的中性色来自 activeColorTokens（computed），会随之自动重算。
 */
function rebuildChartsForTheme() {
  resourceChart?.dispose()
  jobChart?.dispose()
  vendorChart?.dispose()
  resourceChart = null
  jobChart = null
  vendorChart = null
  void syncChartsAfterTick()
}

onMounted(async () => {
  await syncChartsAfterTick()
  stopThemeWatch = onChartThemeChange(rebuildChartsForTheme)
  window.addEventListener('resize', resizeCharts)
  // 主内容区尺寸变化（侧栏折叠 / 响应式断点切换）时自适应重绘
  resizeObserver = new ResizeObserver(() => resizeCharts())
  const content = document.querySelector('.mc-app-content')
  if (content) resizeObserver.observe(content)
})

onBeforeUnmount(() => {
  stopThemeWatch?.()
  window.removeEventListener('resize', resizeCharts)
  resizeObserver?.disconnect()
  resourceChart?.dispose()
  jobChart?.dispose()
  vendorChart?.dispose()
})

watch(resourceOption, syncChartsAfterTick, { flush: 'post' })
watch(jobStatusData, syncChartsAfterTick, { flush: 'post' })
watch(gpuVendorData, syncChartsAfterTick, { flush: 'post' })

/**
 * 兜底：图表容器由 v-if 控制，只有数据到位后才挂载。整页直接加载 /dashboard 时，
 * PageState 仍显示骨架屏，上面的数据 watch 可能早于容器挂载触发，
 * 于是 syncCharts() 里 ref 为空 → echarts 永不 init，卡片留下空白。
 * 显式监听容器 ref（flush:'post' 保证 DOM 已更新），挂载后补一次初始化。
 * 该问题在「先到登录页再由守卫跳转」路径下不会复现，故此前目视核验漏掉了。
 */
watch(
  [resourceChartRef, jobChartRef, vendorChartRef],
  () => void syncChartsAfterTick(),
  { flush: 'post' },
)

// ---------- 告警 ----------
const topAlerts = computed(() => alertsData.value.slice(0, 6))
function alertText(level?: string) {
  return STATUS_TEXT[level ?? 'info'] ?? level ?? '信息'
}
function alertLevelClass(level?: string) {
  if (level === 'critical' || level === 'error') return 'failed'
  if (level === 'warning') return 'pending'
  return 'running'
}
</script>

<style scoped>
/* v3：显式列数保证 7 张卡在任意宽度下都成整齐网格，避免 auto-fit 留下半行空位 */
.kpi-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--mc-gap);
}
@media (max-width: 1400px) { .kpi-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); } }
@media (max-width: 1023px) { .kpi-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); } }
@media (max-width: 640px)  { .kpi-grid { grid-template-columns: minmax(0, 1fr); } }
/* 固定 7 张卡：末张补齐末行剩余列，消除「4+3 右侧空一格」的半行破面。
   各断点分别对应 4 / 3 / 2 列，列对齐保持不变。 */
@media (min-width: 1401px) { .kpi-card:nth-child(7):last-child { grid-column: span 2; } }
@media (min-width: 1024px) and (max-width: 1400px) { .kpi-card:nth-child(7):last-child { grid-column: span 3; } }
@media (min-width: 641px) and (max-width: 1023px) { .kpi-card:nth-child(7):last-child { grid-column: span 2; } }

.kpi-card {
  position: relative;
  background: var(--mc-surface);
  border: 1px solid var(--mc-line);
  border-radius: var(--mc-radius-md);
  box-shadow: var(--mc-shadow-1);
  padding: var(--mc-space-4);
  display: flex;
  align-items: center;
  gap: var(--mc-space-3);
  text-decoration: none;
  color: inherit;
  min-width: 0;
  transition: border-color var(--mc-t-fast) var(--mc-ease), box-shadow var(--mc-t-fast) var(--mc-ease), transform var(--mc-t-fast) var(--mc-ease);
}
.kpi-card:hover,
.kpi-card:focus-visible {
  border-color: var(--mc-brand);
  box-shadow: var(--mc-shadow-3);
  transform: translateY(-1px);
}
.kpi-card:active { transform: none; }
.kpi-icon {
  width: 38px;
  height: 38px;
  border-radius: var(--mc-radius-sm);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.kpi-body { flex: 1; min-width: 0; }
.kpi-title { font-size: var(--mc-fs-xs); font-weight: var(--mc-fw-medium); color: var(--mc-text-3); margin-bottom: 2px; letter-spacing: 0.01em; }
.kpi-value { font-size: 28px; font-weight: var(--mc-fw-semibold); letter-spacing: -0.03em; line-height: 1.2; color: var(--mc-text-1); font-variant-numeric: tabular-nums; }
.kpi-suffix { font-size: var(--mc-fs-sm); font-weight: var(--mc-fw-regular); color: var(--mc-text-3); }
.kpi-footer { font-size: var(--mc-fs-xs); color: var(--mc-text-3); margin-top: 2px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.kpi-footer.is-danger { color: var(--mc-danger-fg); font-weight: var(--mc-fw-medium); }
.kpi-go {
  position: absolute;
  right: 8px;
  top: 8px;
  color: var(--mc-text-3);
  opacity: 0;
  transition: opacity var(--mc-t-fast) var(--mc-ease), color var(--mc-t-fast) var(--mc-ease);
}
.kpi-card:hover .kpi-go,
.kpi-card:focus-visible .kpi-go { opacity: 1; color: var(--mc-brand-text); }

/* GPU 利用率动态环 */
.kpi-ring { width: 42px; height: 42px; flex-shrink: 0; transform: rotate(-90deg); }
.kpi-ring-bg { fill: none; stroke: var(--mc-line); stroke-width: 4; }
.kpi-ring-fg {
  fill: none;
  stroke: var(--mc-brand);
  stroke-width: 4;
  stroke-linecap: round;
  transition: stroke-dashoffset 0.9s var(--mc-ease);
}
.kpi-ring-text {
  fill: var(--mc-text-1);
  font-size: 10px;
  font-weight: var(--mc-fw-semibold);
  transform: rotate(90deg);
  transform-origin: 22px 22px;
  text-anchor: middle;
}

.auto-refresh-hint {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--mc-text-3);
  margin-right: 8px;
  transition: color 0.3s ease;
}
.auto-refresh-hint.is-counting .auto-refresh-dot {
  animation: mc-pulse 1s ease-in-out infinite;
}
.auto-refresh-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--mc-brand);
  opacity: 0.85;
}
@keyframes mc-pulse {
  0%, 100% { opacity: 0.35; transform: scale(0.85); }
  50% { opacity: 1; transform: scale(1.15); }
}

.chart-box { height: 260px; }
.chart-card-head { display: flex; justify-content: space-between; align-items: center; gap: var(--mc-space-3); }
.chart-card-head-right { display: inline-flex; align-items: center; gap: var(--mc-space-3); }
.chart-card-title { margin: 0; font-size: var(--mc-fs-md); font-weight: var(--mc-fw-semibold); letter-spacing: -0.01em; color: var(--mc-text-1); }
.chart-card-extra { font-size: var(--mc-fs-xs); color: var(--mc-text-3); }
.chart-card-more {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  font-size: var(--mc-fs-xs);
  font-weight: var(--mc-fw-medium);
  color: var(--mc-brand-text);
  text-decoration: none;
  transition: color var(--mc-t-fast) var(--mc-ease);
}
.chart-card-more:hover { color: var(--mc-brand-700); }

.alert-list { list-style: none; margin: 0; padding: 0; }
.alert-item {
  padding: var(--mc-space-3);
  border-bottom: 1px solid var(--mc-line-subtle);
  border-left: 2px solid transparent;
  border-radius: var(--mc-radius-xs);
  transition: background var(--mc-t-fast) var(--mc-ease);
}
.alert-item:last-child { border-bottom: none; }
.alert-item:hover { background: var(--mc-surface-2); }
.alert-item.failed { border-left-color: var(--mc-danger); }
.alert-item.pending { border-left-color: var(--mc-warning); }
.alert-item.running { border-left-color: var(--mc-brand); }
.alert-line { display: flex; align-items: center; gap: var(--mc-space-3); margin-bottom: var(--mc-space-1); }
.alert-message { font-size: var(--mc-fs-body); color: var(--mc-text-1); }
.alert-detail { font-size: var(--mc-fs-xs); color: var(--mc-text-3); padding-left: 62px; }
</style>
