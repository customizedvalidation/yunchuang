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
                <span>资源分布</span>
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
                <span>作业状态分布</span>
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
                <span>GPU 厂商分布</span>
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
            <span>最近告警</span>
            <span class="chart-card-extra">按严重程度排序</span>
          </div>
        </template>
        <el-empty v-if="alertsData.length === 0" description="暂无告警，系统运行正常" :image-size="80" />
        <ul v-else class="alert-list">
          <li v-for="item in topAlerts" :key="item.id" class="alert-item">
            <div class="alert-line">
              <span class="mc-status" :class="alertLevelClass(item.level)">
                <i class="mc-status-dot" :style="{ background: alertColor(item.level) }" />
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
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import * as echarts from 'echarts'
import { Refresh, Coin, Cloudy, Timer, Bell, UserFilled, Grid, Operation } from '@element-plus/icons-vue'
import PageState from '@/components/PageState.vue'
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
import type { Cluster, Resource, Job, Alert, GPUDevice, Partition, SchedulerIntegration, Tenant, GPUVendor } from '@/types'

// ---------- 数据加载 ----------
const clusters = useFetch<Cluster[]>(() => clusterApi.list())
const resources = useFetch<Resource[]>(() => resourceApi.list())
const jobs = useFetch<Job[]>(() => jobApi.list())
const alerts = useFetch<Alert[]>(() => monitoringApi.alerts())
const gpuDevicesF = useFetch<GPUDevice[]>(() => gpuApi.devices({}))
const partitionsF = useFetch<Partition[]>(() => partitionApi.list({}))
const schedulersF = useFetch<SchedulerIntegration[]>(() => schedulerApi.list())
const tenantsF = useFetch<Tenant[]>(() => tenantApi.list())

const clustersData = computed(() => clusters.data.value ?? [])
const resourcesData = computed(() => resources.data.value ?? [])
const jobsData = computed(() => jobs.data.value ?? [])
const alertsData = computed(() => alerts.data.value ?? [])
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
const gpuUsed = computed(() =>
  resourcesData.value.reduce((s, r) => s + (Number(r.used) || 0), 0),
)
const gpuTotal = computed(() =>
  resourcesData.value.reduce((s, r) => s + (Number(r.total) || 0), 0),
)
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
    bg: 'linear-gradient(135deg,#2f6bff,#5b8bff)',
    footer: '统一纳管的计算集群',
  },
  {
    title: 'GPU 利用率',
    value: utilization.value,
    suffix: '%',
    icon: Cloudy,
    bg: 'linear-gradient(135deg,#1f52e0,#2f6bff)',
    footer: `<span class="mc-num">已用 ${gpuUsed.value} / 共 ${gpuTotal.value}</span>`,
  },
  {
    title: '运行中作业',
    value: runningJobs.value,
    suffix: ` / ${totalJobs.value}`,
    icon: Timer,
    bg: 'linear-gradient(135deg,#5b8bff,#69b1ff)',
    footer: `排队中 <b class="mc-num">${pendingJobs.value}</b> 个`,
  },
  {
    title: '活跃告警',
    value: alertCount.value,
    suffix: ' 条',
    icon: Bell,
    bg:
      alertCount.value > 0
        ? 'linear-gradient(135deg,#ff5c7a,#ff7a9c)'
        : 'linear-gradient(135deg,#2f6bff,#5b8bff)',
    footer: alertCount.value > 0 ? '<span style="color:var(--mc-danger)">需要关注</span>' : '运行正常',
  },
  {
    title: '租户数量',
    value: tenants.value.length,
    suffix: ' 个',
    icon: UserFilled,
    bg: 'linear-gradient(135deg,#2f6bff,#5b8bff)',
    footer: '多租户资源隔离与配额',
  },
  {
    title: '分区状态',
    value: partitionStats.value.total,
    suffix: ' 个',
    icon: Grid,
    bg: 'linear-gradient(135deg,#2f6bff,#5b8bff)',
    footer: `活跃 ${partitionStats.value.active} · 维护 ${partitionStats.value.maintenance}`,
  },
  {
    title: '调度器集成',
    value: schedulerStats.value.total,
    suffix: ' 个',
    icon: Operation,
    bg: 'linear-gradient(135deg,#1f52e0,#2f6bff)',
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
const VENDOR_COLOR: Record<string, string> = {
  nvidia: '#76b900',
  enflame: '#2f6bff',
  moore_threads: '#7c5cff',
  domestic_x: '#fa8c16',
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
const STATUS_COLOR: Record<string, string> = {
  running: '#2f6bff',
  pending: '#ffb020',
  completed: '#16c784',
  failed: '#ff5c7a',
  cancelled: '#647189',
  active: '#2f6bff',
  maintenance: '#ffb020',
  info: '#2f6bff',
  warning: '#ffb020',
  error: '#ff5c7a',
  critical: '#ff5c7a',
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
    itemStyle: { color: STATUS_COLOR[status] ?? '#647189' },
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
    itemStyle: { color: VENDOR_COLOR[vendor] ?? '#647189' },
  }))
})

function pieOption(data: unknown[], tooltipFmt: string) {
  return {
    tooltip: { trigger: 'item' as const, formatter: tooltipFmt },
    legend: { bottom: 0, itemWidth: 10, itemHeight: 10, textStyle: { color: '#647189', fontSize: 12 } },
    series: [
      {
        type: 'pie' as const,
        radius: ['52%', '74%'],
        center: ['50%', '44%'],
        avoidLabelOverlap: true,
        itemStyle: { borderRadius: 8, borderColor: '#fff', borderWidth: 2 },
        label: { show: true, color: '#647189', fontSize: 12, formatter: '{d}%' },
        data,
      },
    ],
  }
}

const resourceOption = computed(() =>
  pieOption(
    [
      { value: gpuUsed.value, name: '已使用', itemStyle: { color: '#2f6bff' } },
      { value: Math.max(gpuTotal.value - gpuUsed.value, 0), name: '可分配', itemStyle: { color: '#e4eaf3' } },
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

function resizeCharts() {
  resourceChart?.resize()
  jobChart?.resize()
  vendorChart?.resize()
}

onMounted(() => {
  if (resourceChartRef.value) {
    resourceChart = echarts.init(resourceChartRef.value)
    resourceChart.setOption(resourceOption.value)
  }
  if (jobChartRef.value && jobStatusData.value.length > 0) {
    jobChart = echarts.init(jobChartRef.value)
    jobChart.setOption(jobOption.value)
  }
  if (vendorChartRef.value && gpuVendorData.value.length > 0) {
    vendorChart = echarts.init(vendorChartRef.value)
    vendorChart.setOption(vendorOption.value)
  }
  window.addEventListener('resize', resizeCharts)
})

onBeforeUnmount(() => {
  window.removeEventListener('resize', resizeCharts)
  resourceChart?.dispose()
  jobChart?.dispose()
  vendorChart?.dispose()
})

watch(resourceOption, (opt) => {
  if (resourceChart) resourceChart.setOption(opt, true)
  else if (resourceChartRef.value && !resourceChart) {
    resourceChart = echarts.init(resourceChartRef.value)
    resourceChart.setOption(opt)
  }
})
watch(jobOption, (opt) => {
  if (jobChart) jobChart.setOption(opt, true)
  else if (jobChartRef.value && jobStatusData.value.length > 0) {
    jobChart = echarts.init(jobChartRef.value)
    jobChart.setOption(opt)
  }
})
watch(vendorOption, (opt) => {
  if (vendorChart) vendorChart.setOption(opt, true)
  else if (vendorChartRef.value && gpuVendorData.value.length > 0) {
    vendorChart = echarts.init(vendorChartRef.value)
    vendorChart.setOption(opt)
  }
})

// ---------- 告警 ----------
const topAlerts = computed(() => alertsData.value.slice(0, 6))
function alertText(level?: string) {
  return STATUS_TEXT[level ?? 'info'] ?? level ?? '信息'
}
function alertColor(level?: string) {
  return STATUS_COLOR[level ?? 'info'] ?? '#647189'
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
  padding: 20px;
  display: flex;
  align-items: center;
  gap: 14px;
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
.chart-card-extra { font-size: 12px; color: var(--mc-text-3); font-weight: 400; }

.alert-list { list-style: none; margin: 0; padding: 0; }
.alert-item { padding: 12px 0; border-bottom: 1px solid var(--mc-line); }
.alert-item:last-child { border-bottom: none; }
.alert-line { display: flex; align-items: center; gap: 10px; margin-bottom: 4px; }
.alert-message { font-size: 14px; color: var(--mc-text-1); }
.alert-detail { font-size: 12.5px; color: var(--mc-text-3); padding-left: 62px; }
</style>
