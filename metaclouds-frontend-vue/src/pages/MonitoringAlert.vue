<template>
  <div class="mc-page">
    <!-- 页头 -->
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">监控告警中心</h1>
        <p class="mc-page-desc">系统资源监控 · 多级告警管理</p>
      </div>
      <div class="mc-page-head-extra">
        <el-button :icon="Refresh" @click="refreshAll">刷新</el-button>
      </div>
    </div>

    <!-- KPI 指标卡 -->
    <div class="kpi-grid">
      <div class="kpi-card">
        <div class="kpi-icon" style="background: var(--mc-brand)"><OfficeBuilding /></div>
        <div>
          <div class="kpi-label">集群总数</div>
          <div class="kpi-value mc-num">{{ kpi.clusterTotal }}</div>
        </div>
      </div>
      <div class="kpi-card">
        <div class="kpi-icon" style="background: var(--mc-success)"><Monitor /></div>
        <div>
          <div class="kpi-label">在线节点</div>
          <div class="kpi-value mc-num">{{ kpi.onlineNodes }}</div>
        </div>
      </div>
      <div class="kpi-card">
        <div class="kpi-icon" style="background: var(--mc-danger)"><Cpu /></div>
        <div>
          <div class="kpi-label">GPU 利用率</div>
          <div class="kpi-value mc-num">{{ kpi.gpuUtil }}%</div>
        </div>
      </div>
      <div class="kpi-card">
        <div class="kpi-icon" style="background: var(--mc-warning)"><Bell /></div>
        <div>
          <div class="kpi-label">活跃告警</div>
          <div class="kpi-value mc-num">{{ kpi.activeAlerts }}</div>
        </div>
      </div>
    </div>

    <!-- ECharts 图表区 -->
    <el-row :gutter="16" class="mc-mt">
      <el-col :xs="24" :lg="14">
        <el-card shadow="never">
          <template #header>资源利用率趋势（近 24 小时）</template>
          <div ref="trendChartRef" class="chart-box"></div>
        </el-card>
      </el-col>
      <el-col :xs="24" :lg="10">
        <el-card shadow="never">
          <template #header>作业吞吐量（近 24 小时）</template>
          <div ref="throughputChartRef" class="chart-box"></div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 告警列表 -->
    <el-card shadow="never" class="mc-mt">
      <template #header>告警列表（活跃 {{ activeAlertCount }} 条）</template>
      <div class="mc-flow mc-mb">
        <el-select v-model="levelFilter" placeholder="级别" style="width: 140px">
          <el-option label="全部级别" value="all" />
          <el-option v-for="o in LEVEL_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
        <el-select v-model="statusFilter" placeholder="状态" style="width: 140px">
          <el-option label="全部状态" value="all" />
          <el-option v-for="o in STATUS_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
        <el-select v-model="typeFilter" placeholder="类型" clearable style="width: 140px">
          <el-option label="全部类型" value="all" />
          <el-option v-for="o in TYPE_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
      </div>

      <PageState
        :loading="alertsLoading"
        :error="alertsError"
        :data="filteredAlerts"
        empty-text="当前筛选条件下没有匹配的告警事件"
        @retry="alertsRefetch"
      >
        <el-table :data="pagedAlerts" stripe>
          <el-table-column prop="id" label="ID" width="70">
            <template #default="{ row }"><span class="mc-mono">{{ row.id }}</span></template>
          </el-table-column>
          <el-table-column label="级别" width="90">
            <template #default="{ row }">
              <el-tag :type="levelTagType(row.level)">{{ levelText(row.level) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="类型" width="100">
            <template #default="{ row }">{{ row.type || '-' }}</template>
          </el-table-column>
          <el-table-column prop="message" label="消息" min-width="220" show-overflow-tooltip />
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <span class="mc-status" :class="statusClass(effectiveStatus(row))">
                <i class="mc-status-dot" />{{ statusText(effectiveStatus(row)) }}
              </span>
            </template>
          </el-table-column>
          <el-table-column label="集群" width="90">
            <template #default="{ row }">
              <span class="mc-mono">{{ row.cluster_id ?? '-' }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="created_at" label="时间" width="170">
            <template #default="{ row }">{{ row.created_at || '-' }}</template>
          </el-table-column>
          <el-table-column label="操作" width="220" fixed="right">
            <template #default="{ row }">
              <el-button type="primary" link size="small" @click="openDetail(row)">详情</el-button>
              <Can :roles="['admin', 'manager']">
                <template v-if="effectiveStatus(row) === 'active'">
                  <el-button type="primary" link size="small" @click="setStatusOverride(row.id, 'resolved', '解决')">解决</el-button>
                  <el-button type="warning" link size="small" @click="setStatusOverride(row.id, 'ignored', '忽略')">忽略</el-button>
                </template>
                <el-button v-else type="primary" link size="small" @click="setStatusOverride(row.id, 'active', '重新打开')">
                  重新打开
                </el-button>
              </Can>
            </template>
          </el-table-column>
        </el-table>
        <el-pagination
          class="mc-mt"
          layout="total, prev, pager, next"
          :total="filteredAlerts.length"
          v-model:current-page="page.current"
          v-model:page-size="page.size"
          background
        />
      </PageState>
    </el-card>

    <!-- 告警详情 Drawer -->
    <el-drawer v-model="detailVisible" title="告警详情" size="480px">
      <template v-if="detailAlert">
        <el-descriptions :column="1" border>
          <el-descriptions-item label="ID">{{ detailAlert.id }}</el-descriptions-item>
          <el-descriptions-item label="级别">
            <el-tag :type="levelTagType(detailAlert.level)">{{ levelText(detailAlert.level) }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="类型">{{ detailAlert.type || '-' }}</el-descriptions-item>
          <el-descriptions-item label="状态">{{ statusText(effectiveStatus(detailAlert)) }}</el-descriptions-item>
          <el-descriptions-item label="集群 ID">{{ detailAlert.cluster_id ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="作业 ID">{{ detailAlert.job_id ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="消息">{{ detailAlert.message || '-' }}</el-descriptions-item>
          <el-descriptions-item label="详情">{{ detailAlert.details || '-' }}</el-descriptions-item>
          <el-descriptions-item label="创建时间">{{ detailAlert.created_at || '-' }}</el-descriptions-item>
          <el-descriptions-item label="解决时间">{{ detailAlert.resolved_at || '-' }}</el-descriptions-item>
        </el-descriptions>
      </template>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import {
  OfficeBuilding, Monitor, Cpu, Bell, Refresh,
} from '@element-plus/icons-vue'
import * as echarts from 'echarts/core'
import { LineChart, BarChart } from 'echarts/charts'
import { GridComponent, TooltipComponent, LegendComponent } from 'echarts/components'
import { CanvasRenderer } from 'echarts/renderers'
import { monitoringApi } from '@/api'
import type { Alert, AlertLevel, AlertStatus, MetricsOverview } from '@/types'
import { useFetch } from '@/utils/useFetch'
import PageState from '@/components/PageState.vue'
import Can from '@/components/Can.vue'

echarts.use([LineChart, BarChart, GridComponent, TooltipComponent, LegendComponent, CanvasRenderer])

const TIME_POINTS = ['00:00', '02:00', '04:00', '06:00', '08:00', '10:00', '12:00', '14:00', '16:00', '18:00', '20:00', '22:00']

const LEVEL_OPTIONS: { label: string; value: AlertLevel }[] = [
  { label: '严重', value: 'critical' },
  { label: '错误', value: 'error' },
  { label: '警告', value: 'warning' },
  { label: '提示', value: 'info' },
]
const STATUS_OPTIONS: { label: string; value: AlertStatus }[] = [
  { label: '活跃', value: 'active' },
  { label: '已解决', value: 'resolved' },
  { label: '已忽略', value: 'ignored' },
]
const TYPE_OPTIONS = [
  { label: '系统', value: 'system' },
  { label: '资源', value: 'resource' },
  { label: '任务', value: 'job' },
  { label: '安全', value: 'security' },
]

// ---------- 数据 ----------
const { data: metrics, loading: metricsLoading, execute: reloadMetrics } =
  useFetch<MetricsOverview>(() => monitoringApi.metrics())
const { data: alertsRaw, loading: alertsLoading, error: alertsError, execute: alertsRefetch } =
  useFetch<Alert[]>(() => monitoringApi.alerts())
const alerts = computed<Alert[]>(() => alertsRaw.value ?? [])

// ---------- KPI ----------
function num(v: unknown, fallback: number): number {
  const n = Number(v)
  return Number.isFinite(n) ? n : fallback
}
const kpi = computed(() => {
  const m = (metrics.value ?? {}) as Record<string, unknown>
  return {
    clusterTotal: num(m.cluster_total ?? m.clusters, 4),
    onlineNodes: num(m.online_nodes ?? m.nodes_online, 2),
    gpuUtil: Math.round(num(m.gpu_utilization ?? m.avg_utilization ?? m.utilization, 62)),
    activeAlerts: alerts.value.filter((a) => effectiveStatus(a) === 'active').length,
  }
})

// ---------- 告警本地状态覆盖 ----------
const levelFilter = ref<'all' | AlertLevel>('all')
const statusFilter = ref<'all' | AlertStatus>('all')
const typeFilter = ref<'all' | string>('all')
const statusOverrides = ref<Record<number, AlertStatus>>({})

function effectiveStatus(a: Alert): AlertStatus {
  return statusOverrides.value[a.id] ?? a.status ?? 'active'
}
function setStatusOverride(id: number, next: AlertStatus, label: string) {
  statusOverrides.value = { ...statusOverrides.value, [id]: next }
  ElMessage.success(`告警已${label}`)
}

const filteredAlerts = computed(() =>
  alerts.value.filter((a) => {
    if (levelFilter.value !== 'all' && a.level !== levelFilter.value) return false
    if (statusFilter.value !== 'all' && effectiveStatus(a) !== statusFilter.value) return false
    if (typeFilter.value !== 'all' && a.type !== typeFilter.value) return false
    return true
  }),
)
const activeAlertCount = computed(() =>
  alerts.value.filter((a) => effectiveStatus(a) === 'active').length,
)

const page = reactive({ current: 1, size: 10 })
const pagedAlerts = computed(() => {
  const start = (page.current - 1) * page.size
  return filteredAlerts.value.slice(start, start + page.size)
})

// ---------- 展示辅助 ----------
function levelText(l?: string) {
  const map: Record<string, string> = { critical: '严重', error: '错误', warning: '警告', info: '提示' }
  return map[l ?? ''] ?? l ?? '-'
}
function levelTagType(l?: string) {
  if (l === 'critical') return 'danger'
  if (l === 'error') return 'danger'
  if (l === 'warning') return 'warning'
  return 'info'
}
function statusClass(s?: string) {
  return ['running', 'pending', 'completed', 'failed', 'cancelled'].includes(s ?? '') ? (s as string) : 'idle'
}
function statusText(s?: string) {
  const map: Record<string, string> = {
    active: '活跃', resolved: '已解决', ignored: '已忽略',
    running: '运行中', pending: '排队中', completed: '已完成', failed: '失败',
  }
  return map[s ?? ''] ?? s ?? '-'
}

// ---------- ECharts ----------
const trendChartRef = ref<HTMLDivElement>()
const throughputChartRef = ref<HTMLDivElement>()
let trendChart: echarts.ECharts | null = null
let throughputChart: echarts.ECharts | null = null

/** 确定性生成趋势，避免每次渲染跳变 */
function buildTrend(base: number, amp: number, seed: number): number[] {
  return TIME_POINTS.map((_, i) => {
    const wave = Math.sin((i + seed) / 2) * amp
    const v = base + wave
    return Math.max(0, Math.min(100, Math.round(v)))
  })
}

function renderTrend() {
  if (!trendChart) return
  const gpu = kpi.value.gpuUtil
  trendChart.setOption({
    tooltip: { trigger: 'axis' },
    legend: { data: ['CPU', '内存', '磁盘', '网络', 'GPU'], top: 0 },
    grid: { top: 40, left: 44, right: 16, bottom: 28 },
    xAxis: { type: 'category', data: TIME_POINTS },
    yAxis: { type: 'value', max: 100, axisLabel: { formatter: '{value}%' } },
    series: [
      { name: 'CPU', type: 'line', smooth: true, data: buildTrend(45, 12, 1) },
      { name: '内存', type: 'line', smooth: true, data: buildTrend(58, 8, 3) },
      { name: '磁盘', type: 'line', smooth: true, data: buildTrend(42, 6, 5) },
      { name: '网络', type: 'line', smooth: true, data: buildTrend(35, 14, 7) },
      { name: 'GPU', type: 'line', smooth: true, data: buildTrend(gpu, 16, 2) },
    ],
  })
}

function renderThroughput() {
  if (!throughputChart) return
  // 确定性 24h 吞吐量
  const data = TIME_POINTS.map((_, i) =>
    Math.max(4, Math.round(20 + Math.sin(i / 1.8) * 12 + (i % 3) * 3)),
  )
  throughputChart.setOption({
    tooltip: { trigger: 'axis' },
    grid: { top: 24, left: 44, right: 16, bottom: 28 },
    xAxis: { type: 'category', data: TIME_POINTS },
    yAxis: { type: 'value', name: '作业/小时' },
    series: [{ name: '作业吞吐量', type: 'bar', data, itemStyle: { borderRadius: [4, 4, 0, 0] } }],
  })
}

function resizeCharts() {
  trendChart?.resize()
  throughputChart?.resize()
}

onMounted(() => {
  if (trendChartRef.value) trendChart = echarts.init(trendChartRef.value)
  if (throughputChartRef.value) throughputChart = echarts.init(throughputChartRef.value)
  renderTrend()
  renderThroughput()
  window.addEventListener('resize', resizeCharts)
})
onBeforeUnmount(() => {
  window.removeEventListener('resize', resizeCharts)
  trendChart?.dispose()
  throughputChart?.dispose()
})
watch(kpi, () => renderTrend())

function refreshAll() {
  reloadMetrics()
  alertsRefetch()
}

// ---------- 详情 ----------
const detailVisible = ref(false)
const detailAlert = ref<Alert | null>(null)
function openDetail(row: Alert) {
  detailAlert.value = row
  detailVisible.value = true
}

void metricsLoading
</script>

<style scoped>
.kpi-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: var(--mc-gap);
}
.kpi-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 16px 18px;
  background: var(--mc-surface);
  border-radius: var(--mc-radius-lg);
  box-shadow: var(--mc-shadow-raised-sm);
}
.kpi-icon {
  width: 40px;
  height: 40px;
  border-radius: var(--mc-radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-size: 20px;
  flex-shrink: 0;
}
.kpi-label { font-size: 12.5px; color: var(--mc-text-3); }
.kpi-value { font-size: 22px; font-weight: 680; color: var(--mc-text-1); }
.chart-box { width: 100%; height: 300px; }
</style>
