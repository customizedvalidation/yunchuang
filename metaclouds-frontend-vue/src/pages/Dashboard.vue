<template>
  <div class="mc-page">
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">算力总览</h1>
        <p class="mc-page-desc">{{ rangeLabel }} · 实时掌握全平台算力水位、作业吞吐与风险告警</p>
      </div>
      <div class="mc-page-head-extra">
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
      <!-- KPI 卡片行 -->
      <div class="kpi-grid">
        <div v-for="card in kpiCards" :key="card.title" class="kpi-card">
          <div class="kpi-icon" :style="{ background: card.bg }">
            <el-icon :size="22" color="#fff"><component :is="card.icon" /></el-icon>
          </div>
          <div class="kpi-body">
            <div class="kpi-title">{{ card.title }}</div>
            <div class="kpi-value">
              {{ card.value }}<span class="kpi-suffix">{{ card.suffix }}</span>
            </div>
            <div class="kpi-footer" v-html="card.footer"></div>
          </div>
        </div>
      </div>

      <!-- 三个饼图 -->
      <el-row :gutter="16" class="mc-mt">
        <el-col :xs="24" :sm="24" :md="8">
          <el-card shadow="never">
            <template #header>
              <div class="chart-card-head">
                <h3 class="chart-card-title">资源分布</h3>
                <span class="chart-card-extra">GPU 卡</span>
              </div>
            </template>
            <div v-if="gpuTotal > 0" ref="resourceChartRef" class="chart-box" />
            <el-empty v-else description="暂无资源数据" :image-size="80" />
          </el-card>
        </el-col>
        <el-col :xs="24" :sm="24" :md="8">
          <el-card shadow="never">
            <template #header>
              <div class="chart-card-head">
                <h3 class="chart-card-title">作业状态分布</h3>
                <span class="chart-card-extra">共 {{ totalJobs }} 个</span>
              </div>
            </template>
            <div v-if="jobStatusData.length > 0" ref="jobChartRef" class="chart-box" />
            <el-empty v-else description="还没有作业" :image-size="80" />
          </el-card>
        </el-col>
        <el-col :xs="24" :sm="24" :md="8">
          <el-card shadow="never">
            <template #header>
              <div class="chart-card-head">
                <h3 class="chart-card-title">GPU 厂商分布</h3>
                <span class="chart-card-extra">共 {{ gpuDevices.length }} 张</span>
              </div>
            </template>
            <div v-if="gpuVendorData.length > 0" ref="vendorChartRef" class="chart-box" />
            <el-empty v-else description="暂无 GPU 设备" :image-size="80" />
          </el-card>
        </el-col>
      </el-row>

      <!-- 最近告警 -->
      <el-card shadow="never" class="mc-mt">
        <template #header>
          <div class="chart-card-head">
            <h3 class="chart-card-title">最近告警</h3>
            <span class="chart-card-extra">按严重程度排序</span>
          </div>
        </template>
        <el-empty v-if="alertsData.length === 0" description="暂无告警，系统运行正常" :image-size="80" />
        <ul v-else class="alert-list">
          <li v-for="item in topAlerts" :key="item.id" class="alert-item">
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
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as echarts from 'echarts'
import { Refresh, Coin, Cloudy, Timer, Bell, UserFilled, Grid, Operation } from '@element-plus/icons-vue'
import PageState from '@/components/PageState.vue'
import { registerMcLightTheme, palette } from '@/theme/echarts'
import { colorTokens } from '@/theme/tokens'

// 注册 MDS 图表主题（幂等）；所有 ECharts 实例统一使用 'mc-light'。
// 测试环境 echarts 被 mock（仅 init），无 registerTheme，此处静默降级。
try {
  registerMcLightTheme()
} catch {
  /* noop in test env */
}
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

const anyLoading = computed(
  () => clusters.loading.value || resources.loading.value || jobs.loading.value || alerts.loading.value,
)
const anyError = computed(() => {
  const e = clusters.error.value || resources.error.value || jobs.error.value || alerts.error.value
  return e || ''
})
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
}

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
    bg: 'var(--mc-brand-grad)',
    footer: '统一纳管的计算集群',
  },
  {
    title: 'GPU 利用率',
    value: utilization.value,
    suffix: '%',
    icon: Cloudy,
    bg: 'var(--mc-brand-grad)',
    footer: `<span class="mc-num">已用 ${gpuUsed.value} / 共 ${gpuTotal.value}</span>`,
  },
  {
    title: '运行中作业',
    value: runningJobs.value,
    suffix: ` / ${totalJobs.value}`,
    icon: Timer,
    bg: 'var(--mc-brand-grad)',
    footer: `排队中 <b class="mc-num">${pendingJobs.value}</b> 个`,
  },
  {
    title: '活跃告警',
    value: alertCount.value,
    suffix: ' 条',
    icon: Bell,
    bg: alertCount.value > 0 ? 'var(--mc-danger-fg)' : 'var(--mc-brand-grad)',
    footer: alertCount.value > 0 ? '<span style="color:var(--mc-danger-fg)">需要关注</span>' : '运行正常',
  },
  {
    title: '租户数量',
    value: tenants.value.length,
    suffix: ' 个',
    icon: UserFilled,
    bg: 'var(--mc-brand-grad)',
    footer: '多租户资源隔离与配额',
  },
  {
    title: '分区状态',
    value: partitionStats.value.total,
    suffix: ' 个',
    icon: Grid,
    bg: 'var(--mc-brand-grad)',
    footer: `活跃 ${partitionStats.value.active} · 维护 ${partitionStats.value.maintenance}`,
  },
  {
    title: '调度器集成',
    value: schedulerStats.value.total,
    suffix: ' 个',
    icon: Operation,
    bg: 'var(--mc-brand-grad)',
    footer: `活跃 ${schedulerStats.value.active}${schedulerStats.value.error > 0 ? ` · 异常 ${schedulerStats.value.error}` : ''}`,
  },
])

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
    legend: { bottom: 0, itemWidth: 10, itemHeight: 10 },
    series: [
      {
        type: 'pie' as const,
        radius: ['52%', '74%'],
        center: ['50%', '44%'],
        avoidLabelOverlap: true,
        itemStyle: { borderRadius: 8, borderColor: colorTokens.neutral.surface, borderWidth: 2 },
        label: { show: true, fontSize: 12, formatter: '{d}%' },
        data,
      },
    ],
  }
}

const resourceOption = computed(() =>
  pieOption(
    [
      { value: gpuUsed.value, name: '已使用', itemStyle: { color: palette[0] } },
      { value: Math.max(gpuTotal.value - gpuUsed.value, 0), name: '可分配', itemStyle: { color: colorTokens.neutral.line } },
    ],
    '{b}: {c} ({d}%)',
  ),
)
const jobOption = computed(() => pieOption(jobStatusData.value, '{b}: {c} ({d}%)'))
const vendorOption = computed(() => pieOption(gpuVendorData.value, '{b}: {c} 张 ({d}%)'))

const resourceChartRef = ref<HTMLElement>()
const jobChartRef = ref<HTMLElement>()
const vendorChartRef = ref<HTMLElement>()
let resourceChart: echarts.ECharts | null = null
let jobChart: echarts.ECharts | null = null
let vendorChart: echarts.ECharts | null = null

let resizeObserver: ResizeObserver | null = null

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
    resourceChart = echarts.init(resourceChartRef.value, 'mc-light')
  }
  if (resourceChart) resourceChart.setOption(resourceOption.value, true)

  if (jobChartRef.value && jobStatusData.value.length > 0 && !jobChart) {
    jobChart = echarts.init(jobChartRef.value, 'mc-light')
  }
  if (jobChart) jobChart.setOption(jobOption.value, true)

  if (vendorChartRef.value && gpuVendorData.value.length > 0 && !vendorChart) {
    vendorChart = echarts.init(vendorChartRef.value, 'mc-light')
  }
  if (vendorChart) vendorChart.setOption(vendorOption.value, true)
}

async function syncChartsAfterTick() {
  await nextTick()
  syncCharts()
}

onMounted(async () => {
  await syncChartsAfterTick()
  window.addEventListener('resize', resizeCharts)
  // 主内容区尺寸变化（侧栏折叠 / 响应式断点切换）时自适应重绘
  resizeObserver = new ResizeObserver(() => resizeCharts())
  const content = document.querySelector('.mc-app-content')
  if (content) resizeObserver.observe(content)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', resizeCharts)
  resizeObserver?.disconnect()
  resourceChart?.dispose()
  jobChart?.dispose()
  vendorChart?.dispose()
})

watch(resourceOption, syncChartsAfterTick, { flush: 'post' })
watch(jobStatusData, syncChartsAfterTick, { flush: 'post' })
watch(gpuVendorData, syncChartsAfterTick, { flush: 'post' })

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
.kpi-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(210px, 1fr));
  gap: var(--mc-gap);
}
.kpi-card {
  background: var(--mc-surface);
  border-radius: var(--mc-radius-lg);
  box-shadow: var(--mc-shadow-raised-sm);
  padding: var(--mc-gap);
  display: flex;
  align-items: center;
  gap: var(--mc-gap);
}
.kpi-icon {
  width: 44px;
  height: 44px;
  border-radius: var(--mc-radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.kpi-body { flex: 1; min-width: 0; }
.kpi-title { font-size: 13px; color: var(--mc-text-3); margin-bottom: 2px; }
.kpi-value { font-size: 26px; font-weight: 680; letter-spacing: -0.5px; color: var(--mc-text-1); }
.kpi-suffix { font-size: 14px; font-weight: 400; color: var(--mc-text-3); }
.kpi-footer { font-size: 12.5px; color: var(--mc-text-3); margin-top: 4px; }

.chart-box { height: 280px; }
.chart-card-head { display: flex; justify-content: space-between; align-items: center; }
.chart-card-title { margin: 0; font-size: var(--mc-fs-h2); font-weight: 650; color: var(--mc-text-1); }
.chart-card-extra { font-size: 12px; color: var(--mc-text-3); font-weight: 400; }

.alert-list { list-style: none; margin: 0; padding: 0; }
.alert-item { padding: 12px 0; border-bottom: 1px solid var(--mc-line); }
.alert-item:last-child { border-bottom: none; }
.alert-line { display: flex; align-items: center; gap: 10px; margin-bottom: 4px; }
.alert-message { font-size: 14px; color: var(--mc-text-1); }
.alert-detail { font-size: 12.5px; color: var(--mc-text-3); padding-left: 62px; }
</style>
