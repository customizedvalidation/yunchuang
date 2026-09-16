<template>
  <div class="mc-page">
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">GPU 细粒度管理</h1>
        <p class="mc-page-desc">
          共 {{ stats.total }} 张 GPU · 已分配 {{ stats.allocated }} · 可用 {{ stats.available }} · 维护中 {{ stats.maintenance }} · 故障 {{ stats.fault }}
        </p>
      </div>
      <div class="mc-page-head-extra">
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="openCreate">新增 GPU 设备</el-button>
        </Can>
      </div>
    </div>

    <!-- GPU 概览卡片 -->
    <el-row :gutter="16" class="mc-mb">
      <el-col :xs="12" :sm="12" :md="8" :lg="4" :xl="4">
        <el-card shadow="never">
          <div class="stat-label">总设备数</div>
          <div class="stat-value">{{ stats.total }} <span class="stat-unit">张</span></div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="12" :md="8" :lg="4" :xl="4">
        <el-card shadow="never">
          <div class="stat-label">可用</div>
          <div class="stat-value" style="color: var(--mc-success)">{{ stats.available }} <span class="stat-unit">张</span></div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="12" :md="8" :lg="4" :xl="4">
        <el-card shadow="never">
          <div class="stat-label">已分配</div>
          <div class="stat-value" style="color: var(--mc-warning)">{{ stats.allocated }} <span class="stat-unit">张</span></div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="12" :md="8" :lg="4" :xl="4">
        <el-card shadow="never">
          <div class="stat-label">维护中</div>
          <div class="stat-value" style="color: var(--mc-brand-fg)">{{ stats.maintenance }} <span class="stat-unit">张</span></div>
        </el-card>
      </el-col>
      <el-col :xs="12" :sm="12" :md="8" :lg="4" :xl="4">
        <el-card shadow="never">
          <div class="stat-label">故障</div>
          <div class="stat-value" style="color: var(--mc-danger)">{{ stats.fault }} <span class="stat-unit">张</span></div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="24" :md="24" :lg="4" :xl="4">
        <el-card shadow="never">
          <div class="stat-label">按厂商分布</div>
          <div class="vendor-dist">
            <el-tag v-for="(count, vendor) in vendorDist" :key="vendor" :type="vendorTagType(vendor as GPUVendor)" size="small">
              {{ vendorLabel(vendor as GPUVendor) }} {{ count }}
            </el-tag>
            <span v-if="!Object.keys(vendorDist).length" class="mc-empty-mini">-</span>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 筛选器 -->
    <el-card shadow="never" class="mc-mb">
      <div class="toolbar">
        <el-input
          v-model="keyword"
          placeholder="搜索节点名 / 型号"
          clearable
          style="width: 240px"
          aria-label="搜索 GPU 节点"
        />
        <el-select v-model="vendorFilter" placeholder="厂商筛选" clearable style="width: 160px">
          <el-option v-for="o in VENDOR_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
        <el-select v-model="statusFilter" placeholder="状态筛选" clearable style="width: 150px">
          <el-option v-for="o in GPU_STATUS_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
        <el-select v-model="clusterFilter" placeholder="集群筛选" clearable style="width: 180px">
          <el-option v-for="c in clustersData" :key="c.id" :label="c.name" :value="c.id" />
        </el-select>
      </div>
    </el-card>

    <!-- 设备表格 -->
    <el-card shadow="never" id="gpu-table">
      <PageState
        :loading="devices.loading.value"
        :error="devices.error.value"
        :data="pagedData"
        empty-text="暂无 GPU 设备，登记后可进行细粒度分配与监控"
        @retry="devices.refetch"
      >
        <el-table :data="pagedData" stripe border style="width: 100%">
          <el-table-column prop="id" label="ID" width="70">
            <template #default="{ row }"><span class="mc-mono">{{ row.id }}</span></template>
          </el-table-column>
          <el-table-column label="节点名" width="150">
            <template #default="{ row }">
              <el-button link type="primary" @click="detailDevice = row">{{ row.node_name }}</el-button>
            </template>
          </el-table-column>
          <el-table-column label="厂商" width="110">
            <template #default="{ row }">
              <el-tag :type="vendorTagType(row.vendor)" size="small">{{ vendorLabel(row.vendor) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="model" label="型号" width="110" />
          <el-table-column label="索引" width="80">
            <template #default="{ row }"><span class="mc-num">{{ row.index ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="总显存(GB)" width="110">
            <template #default="{ row }"><span class="mc-num">{{ row.total_memory_gb ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="已用显存(GB)" width="120">
            <template #default="{ row }"><span class="mc-num">{{ row.used_memory_gb ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="MIG" width="80">
            <template #default="{ row }">
              <el-tag :type="row.mig_enabled ? 'primary' : 'info'" size="small">{{ row.mig_enabled ? '开启' : '关闭' }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="driver_version" label="驱动版本" width="110" />
          <el-table-column prop="cuda_version" label="CUDA版本" width="100" />
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag :type="statusType(row.status)" size="small">{{ statusText(row.status) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="利用率" width="140">
            <template #default="{ row }">
              <el-progress :percentage="row.utilization ?? 0" :stroke-width="8" />
            </template>
          </el-table-column>
          <el-table-column label="温度(°C)" width="90">
            <template #default="{ row }"><span class="mc-num">{{ row.temperature ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="功耗(W)" width="90">
            <template #default="{ row }"><span class="mc-num">{{ row.power_draw ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="操作" width="240" fixed="right">
            <template #default="{ row }">
              <Can :roles="['admin', 'manager']">
                <el-button v-if="row.status === 'available'" link type="primary" size="small" @click="openAllocate(row)">分配</el-button>
              </Can>
              <Can :roles="['admin', 'manager']">
                <el-button link type="primary" size="small" @click="openEdit(row)">编辑</el-button>
              </Can>
              <Can :roles="['admin', 'manager']">
                <el-button link type="primary" size="small" @click="openAllocations(row)">分配记录</el-button>
              </Can>
              <Can :roles="['admin', 'manager']">
                <el-popconfirm title="删除该 GPU 设备？设备记录将被移除且不可恢复。" confirm-button-text="确认删除" cancel-button-text="返回" @confirm="handleDelete(row.id)">
                  <template #reference>
                    <el-button link type="danger" size="small">删除</el-button>
                  </template>
                </el-popconfirm>
              </Can>
            </template>
          </el-table-column>
        </el-table>
        <el-pagination
          v-model:current-page="currentPage"
          background
          layout="prev, pager, next, total"
          :total="searchedDevices.length"
          :page-size="pageSize"
          style="margin-top: 12px; justify-content: flex-end"
        />
      </PageState>
    </el-card>

    <!-- 新建/编辑 Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="editingDevice ? '编辑 GPU 设备' : '新增 GPU 设备'"
      width="720px"
      destroy-on-close
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-position="top">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="节点名" prop="node_name">
              <el-input v-model="form.node_name" placeholder="例如：gpu-node-01" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="厂商" prop="vendor">
              <el-select v-model="form.vendor" placeholder="请选择厂商" style="width: 100%">
                <el-option v-for="o in VENDOR_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="型号" prop="model">
              <el-input v-model="form.model" placeholder="例如：A100-80GB" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="GPU 索引" prop="index">
              <el-input-number v-model="form.index" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="总显存(GB)" prop="total_memory_gb">
              <el-input-number v-model="form.total_memory_gb" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="可分配显存(GB)" prop="allocatable_memory_gb">
              <el-input-number v-model="form.allocatable_memory_gb" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="驱动版本" prop="driver_version">
              <el-input v-model="form.driver_version" placeholder="例如：535.129.03" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="CUDA 版本" prop="cuda_version">
              <el-input v-model="form.cuda_version" placeholder="例如：12.2" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="所属集群" prop="cluster_id">
              <el-select v-model="form.cluster_id" placeholder="请选择集群" clearable style="width: 100%">
                <el-option v-for="c in clustersData" :key="c.id" :label="c.name" :value="c.id" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="状态" prop="status">
              <el-select v-model="form.status" placeholder="请选择状态" style="width: 100%">
                <el-option v-for="o in GPU_STATUS_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="MIG 开启" prop="mig_enabled">
              <el-switch v-model="form.mig_enabled" />
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="详情" prop="details">
              <el-input v-model="form.details" type="textarea" :rows="2" placeholder="选填" />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="submitForm">{{ editingDevice ? '保存' : '创建' }}</el-button>
        </Can>
      </template>
    </el-dialog>

    <!-- 分配 GPU Dialog -->
    <el-dialog
      v-model="allocateVisible"
      :title="`分配 GPU — ${allocatingDevice?.node_name ?? ''} ${allocatingDevice?.model ?? ''}`"
      width="560px"
      destroy-on-close
    >
      <el-form ref="allocFormRef" :model="allocForm" label-position="top">
        <el-form-item label="GPU 分配粒度" prop="fraction" required>
          <el-select v-model="allocForm.fraction" placeholder="请选择分配粒度" style="width: 100%">
            <el-option label="1 GPU（整卡，计算密集型）" :value="1" />
            <el-option label="1/2 GPU（中等需求）" :value="0.5" />
            <el-option label="1/4 GPU（轻量任务）" :value="0.25" />
          </el-select>
        </el-form-item>
        <el-form-item label="分配显存(GB)">
          <el-input-number v-model="allocForm.memory_gb" :min="0" style="width: 100%" placeholder="留空则按粒度自动分配" />
        </el-form-item>
        <el-row :gutter="16">
          <el-col :span="8">
            <el-form-item label="作业ID">
              <el-input-number v-model="allocForm.job_id" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="租户ID">
              <el-input-number v-model="allocForm.tenant_id" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="用户ID">
              <el-input-number v-model="allocForm.user_id" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="allocateVisible = false">取消</el-button>
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="submitAllocate">确认分配</el-button>
        </Can>
      </template>
    </el-dialog>

    <!-- GPU 分配记录 Drawer -->
    <el-drawer v-model="allocDrawerVisible" title="GPU 分配记录" size="720px">
      <el-alert v-if="currentAllocations.length === 0" type="info" :closable="false" title="暂无分配记录" />
      <el-table v-else :data="currentAllocations" size="small" stripe>
        <el-table-column prop="device_id" label="设备ID" width="90" />
        <el-table-column prop="job_id" label="作业ID" width="90" />
        <el-table-column label="分配粒度" width="110">
          <template #default="{ row }">
            <el-tag type="primary" size="small">
              {{ row.fraction >= 1 ? '1 GPU' : row.fraction === 0.5 ? '1/2 GPU' : row.fraction === 0.25 ? '1/4 GPU' : `${row.fraction} GPU` }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="显存(GB)" width="100">
          <template #default="{ row }"><span class="mc-num">{{ row.memory_gb ?? '-' }}</span></template>
        </el-table-column>
        <el-table-column prop="mig_profile" label="MIG Profile" width="130">
          <template #default="{ row }">{{ row.mig_profile ?? '-' }}</template>
        </el-table-column>
        <el-table-column label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="statusType(row.status)" size="small">{{ statusText(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="started_at" label="开始时间" width="170" />
        <el-table-column label="操作" width="90">
          <template #default="{ row }">
            <Can v-if="row.status === 'active'" :roles="['admin', 'manager']">
              <el-popconfirm title="释放该分配？" confirm-button-text="释放" cancel-button-text="取消" @confirm="releaseAllocation(row)">
                <template #reference>
                  <el-button link type="danger" size="small">释放</el-button>
                </template>
              </el-popconfirm>
            </Can>
            <span v-else>-</span>
          </template>
        </el-table-column>
      </el-table>
    </el-drawer>

    <!-- 设备详情 Drawer -->
    <el-drawer v-model="detailVisible" title="GPU 设备详情" size="480px">
      <div v-if="detailDevice">
        <div v-for="row in detailRows" :key="row.label" class="detail-row">
          <span class="detail-label">{{ row.label }}</span>
          <span class="detail-value">{{ row.value }}</span>
        </div>
      </div>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import PageState from '@/components/PageState.vue'
import Can from '@/components/Can.vue'
import { gpuApi, clusterApi } from '@/api'
import { useFetch } from '@/utils/useFetch'
import type { GPUDevice, GPUAllocation, GPUVendor, Cluster } from '@/types'

const VENDOR_OPTIONS: { label: string; value: GPUVendor }[] = [
  { label: 'NVIDIA', value: 'nvidia' },
  { label: '燧原 Enflame', value: 'enflame' },
  { label: '摩尔线程', value: 'moore_threads' },
  { label: '国产 X', value: 'domestic_x' },
]

const GPU_STATUS_OPTIONS = [
  { label: '可用', value: 'available' },
  { label: '已分配', value: 'allocated' },
  { label: '维护中', value: 'maintenance' },
  { label: '故障', value: 'fault' },
]

const VENDOR_LABEL: Record<GPUVendor, string> = {
  nvidia: 'NVIDIA',
  enflame: '燧原',
  moore_threads: '摩尔线程',
  domestic_x: '国产X',
}

function vendorLabel(v?: GPUVendor): string {
  return (v && VENDOR_LABEL[v]) ?? v ?? '-'
}
function vendorTagType(v?: GPUVendor) {
  switch (v) {
    case 'nvidia': return 'success'
    case 'enflame': return 'primary'
    case 'moore_threads': return 'warning'
    case 'domestic_x': return 'danger'
    default: return 'info'
  }
}

const STATUS_TYPE: Record<string, string> = {
  available: 'success', active: 'success', completed: 'success', healthy: 'success', success: 'success', running: 'primary',
  allocated: 'warning', pending: 'warning', maintenance: 'warning', paused: 'warning', warning: 'warning',
  fault: 'danger', failed: 'danger', error: 'danger', unhealthy: 'danger', critical: 'danger',
  inactive: 'info', disabled: 'info', idle: 'info', released: 'info',
}
const STATUS_TEXT: Record<string, string> = {
  available: '可用', allocated: '已分配', maintenance: '维护中', fault: '故障', active: '已启用', inactive: '未启用',
  drained: '已排空', running: '运行中', pending: '排队中', completed: '已完成', failed: '失败', error: '失败',
  healthy: '健康', unhealthy: '异常', active_alloc: '活跃',
}
function statusType(s?: string) {
  return (STATUS_TYPE[s ?? ''] ?? 'info') as 'success' | 'warning' | 'danger' | 'info' | 'primary'
}
function statusText(s?: string) {
  return (s && STATUS_TEXT[s]) ?? s ?? '-'
}

// ---------- 筛选 ----------
const keyword = ref('')
const vendorFilter = ref<GPUVendor | ''>('')
const statusFilter = ref<string>('')
const clusterFilter = ref<number | undefined>(undefined)

// ---------- 数据 ----------
const devices = useFetch<GPUDevice[]>(() =>
  gpuApi.devices({
    vendor: vendorFilter.value || undefined,
    status: statusFilter.value || undefined,
    cluster_id: clusterFilter.value,
  }),
)
const allocations = useFetch<GPUAllocation[]>(() => gpuApi.allocations({}))
const clusters = useFetch<Cluster[]>(() => clusterApi.list())

watch([vendorFilter, statusFilter, clusterFilter], () => devices.refetch())

const devicesData = computed(() => devices.data.value ?? [])
const allocationsData = computed(() => allocations.data.value ?? [])
const clustersData = computed(() => clusters.data.value ?? [])

// 节点名 / 型号模糊搜索
const searchedDevices = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  if (!kw) return devicesData.value
  return devicesData.value.filter(
    (d) => d.node_name.toLowerCase().includes(kw) || (d.model ?? '').toLowerCase().includes(kw),
  )
})

// 分页
const currentPage = ref(1)
const pageSize = 10
const pagedData = computed(() =>
  searchedDevices.value.slice((currentPage.value - 1) * pageSize, currentPage.value * pageSize),
)
watch(searchedDevices, () => (currentPage.value = 1))

// ---------- 统计 ----------
const stats = computed(() => {
  const list = devicesData.value
  return {
    total: list.length,
    available: list.filter((d) => d.status === 'available').length,
    allocated: list.filter((d) => d.status === 'allocated').length,
    maintenance: list.filter((d) => d.status === 'maintenance').length,
    fault: list.filter((d) => d.status === 'fault').length,
  }
})
const vendorDist = computed<Record<string, number>>(() => {
  const dist: Record<string, number> = {}
  for (const d of devicesData.value) {
    if (!d.vendor) continue
    dist[d.vendor] = (dist[d.vendor] ?? 0) + 1
  }
  return dist
})

// ---------- 表单 ----------
const formRef = ref<FormInstance>()
const dialogVisible = ref(false)
const editingDevice = ref<GPUDevice | null>(null)
const form = reactive({
  node_name: '',
  vendor: 'nvidia' as GPUVendor,
  model: '',
  index: undefined as number | undefined,
  total_memory_gb: undefined as number | undefined,
  allocatable_memory_gb: undefined as number | undefined,
  mig_enabled: false,
  driver_version: '',
  cuda_version: '',
  status: 'available' as GPUDevice['status'],
  cluster_id: undefined as number | undefined,
  details: '',
})
const rules: FormRules = {
  node_name: [{ required: true, message: '请输入节点名', trigger: 'blur' }],
  vendor: [{ required: true, message: '请选择厂商', trigger: 'change' }],
}

function resetForm() {
  Object.assign(form, {
    node_name: '', vendor: 'nvidia', model: '', index: undefined, total_memory_gb: undefined,
    allocatable_memory_gb: undefined, mig_enabled: false, driver_version: '', cuda_version: '',
    status: 'available', cluster_id: undefined, details: '',
  })
}
function openCreate() {
  editingDevice.value = null
  resetForm()
  dialogVisible.value = true
}
function openEdit(row: GPUDevice) {
  editingDevice.value = row
  Object.assign(form, {
    node_name: row.node_name, vendor: row.vendor, model: row.model, index: row.index,
    total_memory_gb: row.total_memory_gb, allocatable_memory_gb: row.allocatable_memory_gb,
    mig_enabled: row.mig_enabled ?? false, driver_version: row.driver_version, cuda_version: row.cuda_version,
    status: row.status ?? 'available', cluster_id: row.cluster_id, details: row.details,
  })
  dialogVisible.value = true
}
async function submitForm() {
  await formRef.value?.validate()
  try {
    if (editingDevice.value) {
      await gpuApi.updateDevice(editingDevice.value.id, { ...form })
      ElMessage.success('GPU 设备更新成功')
    } else {
      await gpuApi.createDevice({ ...form })
      ElMessage.success('GPU 设备创建成功')
    }
    dialogVisible.value = false
    resetForm()
    devices.refetch()
  } catch {
    ElMessage.error('操作失败，请检查必填项后重试')
  }
}
async function handleDelete(id: number) {
  try {
    await gpuApi.deleteDevice(id)
    ElMessage.success('GPU 设备删除成功')
    devices.refetch()
  } catch {
    ElMessage.error('删除失败，请稍后重试')
  }
}

// ---------- 分配 ----------
const allocateVisible = ref(false)
const allocatingDevice = ref<GPUDevice | null>(null)
const allocFormRef = ref<FormInstance>()
const allocForm = reactive({
  fraction: 1 as number,
  memory_gb: undefined as number | undefined,
  job_id: undefined as number | undefined,
  tenant_id: undefined as number | undefined,
  user_id: undefined as number | undefined,
})
function openAllocate(row: GPUDevice) {
  allocatingDevice.value = row
  Object.assign(allocForm, { fraction: 1, memory_gb: undefined, job_id: undefined, tenant_id: undefined, user_id: undefined })
  allocateVisible.value = true
}
async function submitAllocate() {
  if (!allocatingDevice.value) return
  try {
    await gpuApi.allocate({
      device_id: allocatingDevice.value.id,
      fraction: allocForm.fraction,
      memory_gb: allocForm.memory_gb,
      job_id: allocForm.job_id,
      tenant_id: allocForm.tenant_id,
      user_id: allocForm.user_id,
      status: 'active',
    })
    const fracText = allocForm.fraction >= 1 ? '1 GPU' : `${Math.round(1 / allocForm.fraction)} 分之一 GPU`
    ElMessage.success(`已按 ${fracText} 粒度分配`)
    allocateVisible.value = false
    devices.refetch()
    allocations.refetch()
  } catch {
    ElMessage.error('分配失败，请稍后重试')
  }
}

// ---------- 分配记录 drawer ----------
const allocDrawerVisible = ref(false)
const allocDeviceId = ref<number>(0)
const currentAllocations = computed(() =>
  allocationsData.value.filter((a) => a.device_id === allocDeviceId.value),
)
function openAllocations(row: GPUDevice) {
  allocDeviceId.value = row.id
  allocDrawerVisible.value = true
}
async function releaseAllocation(alloc: GPUAllocation) {
  try {
    await gpuApi.release(alloc.id)
    ElMessage.success('GPU 已释放')
    allocations.refetch()
  } catch {
    ElMessage.error('释放失败，请稍后重试')
  }
}

// ---------- 详情 ----------
const detailVisible = ref(false)
const detailDevice = ref<GPUDevice | null>(null)
watch(detailDevice, (v) => { detailVisible.value = !!v })
const detailRows = computed(() => {
  const d = detailDevice.value
  if (!d) return []
  return [
    { label: '节点名', value: d.node_name },
    { label: '厂商', value: vendorLabel(d.vendor) },
    { label: '型号', value: d.model ?? '-' },
    { label: '索引', value: d.index ?? '-' },
    { label: '总显存', value: `${d.total_memory_gb ?? '-'} GB` },
    { label: '已用显存', value: `${d.used_memory_gb ?? '-'} GB` },
    { label: '可分配显存', value: `${d.allocatable_memory_gb ?? '-'} GB` },
    { label: 'MIG', value: d.mig_enabled ? '开启' : '关闭' },
    { label: '驱动版本', value: d.driver_version ?? '-' },
    { label: 'CUDA 版本', value: d.cuda_version ?? '-' },
    { label: '状态', value: statusText(d.status) },
    { label: '利用率', value: `${d.utilization ?? 0}%` },
    { label: '温度', value: `${d.temperature ?? '-'} °C` },
    { label: '功耗', value: `${d.power_draw ?? '-'} W` },
    { label: 'MIG Profiles', value: d.mig_profiles ?? '-' },
    { label: '详情', value: d.details ?? '-' },
  ]
})
</script>

<style scoped>
.toolbar { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; }
.stat-label { font-size: 12.5px; color: var(--mc-text-3); margin-bottom: 6px; }
.stat-value { font-size: 24px; font-weight: 660; color: var(--mc-text-1); }
.stat-unit { font-size: 12px; font-weight: 500; color: var(--mc-text-3); }
.vendor-dist { display: flex; flex-wrap: wrap; gap: 6px; }
.detail-row { display: flex; padding: 8px 0; border-bottom: 1px solid var(--mc-line); }
.detail-label { width: 120px; color: var(--mc-text-3); flex-shrink: 0; }
.detail-value { flex: 1; word-break: break-all; }
.mc-empty-mini { color: var(--mc-text-3); }
</style>
