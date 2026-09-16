<template>
  <div class="mc-page">
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">调度器集成管理</h1>
        <p class="mc-page-desc">共 {{ schedulersData.length }} 个调度器集成 · 统一纳管 Slurm/LSF/SGE/K8s 调度器</p>
      </div>
      <div class="mc-page-head-extra">
        <el-tag v-if="healthResult" :type="healthResult.type" size="large" style="margin-right: 8px">
          {{ healthCheckName }} 健康检查：{{ healthResult.label }}
        </el-tag>
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="openCreate">新增调度器</el-button>
        </Can>
      </div>
    </div>

    <el-card shadow="never" id="scheduler-table">
      <div class="toolbar mc-mb">
        <el-input v-model="searchText" placeholder="搜索调度器名称 / 端点" clearable style="flex: 1; min-width: 220px" />
        <el-select v-model="typeFilter" placeholder="调度器类型" clearable style="width: 160px">
          <el-option v-for="o in SCHEDULER_TYPE_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
        <el-select v-model="statusFilter" placeholder="状态" clearable style="width: 140px">
          <el-option v-for="o in SCHED_STATUS_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
      </div>

      <PageState
        :loading="schedulers.loading.value"
        :error="schedulers.error.value"
        :data="pagedData"
        empty-text="暂无调度器集成，配置后可统一纳管"
        @retry="schedulers.refetch"
      >
        <el-table :data="pagedData" stripe border style="width: 100%">
          <el-table-column label="ID" width="70">
            <template #default="{ row }"><span class="mc-mono">{{ row.id }}</span></template>
          </el-table-column>
          <el-table-column prop="name" label="名称" width="150" />
          <el-table-column label="类型" width="120">
            <template #default="{ row }">
              <el-tag type="primary" size="small">{{ typeLabel(row.type) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="endpoint" label="端点" min-width="180" show-overflow-tooltip />
          <el-table-column label="认证类型" width="110">
            <template #default="{ row }">{{ authLabel(row.auth_type) }}</template>
          </el-table-column>
          <el-table-column prop="version" label="版本" width="90" />
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag :type="statusType(row.status)" size="small">{{ statusText(row.status) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="default_partition" label="默认分区" width="110" />
          <el-table-column prop="default_qos" label="默认QoS" width="100" />
          <el-table-column label="最大节点" width="90">
            <template #default="{ row }"><span class="mc-num">{{ row.max_nodes ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="最大作业" width="90">
            <template #default="{ row }"><span class="mc-num">{{ row.max_jobs ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column prop="last_sync_at" label="最后同步时间" width="170" />
          <el-table-column label="操作" width="320" fixed="right">
            <template #default="{ row }">
              <Can :roles="['admin', 'manager']">
                <el-button link type="primary" size="small" @click="openEdit(row)">编辑</el-button>
              </Can>
              <el-button link type="primary" size="small" @click="openQueues(row)">队列</el-button>
              <el-button link type="primary" size="small" @click="openNodes(row)">节点</el-button>
              <Can :roles="['admin', 'manager']">
                <el-button link type="primary" size="small" @click="syncJobs(row.id)">同步作业</el-button>
              </Can>
              <el-button link type="primary" size="small" @click="healthCheck(row)">健康检查</el-button>
              <Can :roles="['admin', 'manager']">
                <el-popconfirm title="删除该调度器集成？集成配置将被移除且不可恢复。" confirm-button-text="确认删除" cancel-button-text="返回" @confirm="handleDelete(row.id)">
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
          :total="filteredSchedulers.length"
          :page-size="pageSize"
          style="margin-top: 12px; justify-content: flex-end"
        />
      </PageState>
    </el-card>

    <!-- 新建/编辑 Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="editingScheduler ? '编辑调度器' : '新增调度器'"
      width="720px"
      destroy-on-close
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-position="top">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="名称" prop="name">
              <el-input v-model="form.name" placeholder="例如：slurm-cluster-a" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="类型" prop="type">
              <el-select v-model="form.type" placeholder="请选择调度器类型" style="width: 100%">
                <el-option v-for="o in SCHEDULER_TYPE_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="16">
            <el-form-item label="端点" prop="endpoint">
              <el-input v-model="form.endpoint" placeholder="例如：slurmctl.example.com:6817" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="认证类型" prop="auth_type">
              <el-select v-model="form.auth_type" placeholder="请选择认证方式" style="width: 100%">
                <el-option v-for="o in AUTH_TYPE_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="认证配置 (JSON)" prop="auth_config">
              <el-input v-model="form.auth_config" type="textarea" :rows="3" placeholder='例如：{"ssh_key":"/path/to/key","user":"slurm"}' />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="版本" prop="version">
              <el-input v-model="form.version" placeholder="例如：23.02" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="默认分区" prop="default_partition">
              <el-input v-model="form.default_partition" placeholder="例如：compute" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="默认 QoS" prop="default_qos">
              <el-input v-model="form.default_qos" placeholder="例如：normal" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="最大节点数" prop="max_nodes">
              <el-input-number v-model="form.max_nodes" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="最大作业数" prop="max_jobs">
              <el-input-number v-model="form.max_jobs" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="状态" prop="status">
              <el-select v-model="form.status" placeholder="请选择状态" style="width: 100%">
                <el-option v-for="o in SCHED_STATUS_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
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
          <el-button type="primary" @click="submitForm">{{ editingScheduler ? '保存' : '创建' }}</el-button>
        </Can>
      </template>
    </el-dialog>

    <!-- 队列 Drawer -->
    <el-drawer v-model="queueVisible" :title="`队列信息 - ${queueTarget?.name ?? ''}`" size="760px">
      <el-table :data="queuesData" size="small" stripe>
        <el-table-column prop="name" label="队列名" width="140" />
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <el-tag :type="statusType(row.status)" size="small">{{ statusText(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="total_nodes" label="总节点" width="90" />
        <el-table-column prop="allocated_nodes" label="已分配节点" width="120" />
        <el-table-column prop="total_gpus" label="总GPU" width="90" />
        <el-table-column prop="allocated_gpus" label="已分配GPU" width="110" />
        <el-table-column prop="pending_jobs" label="等待作业" width="100" />
        <el-table-column prop="running_jobs" label="运行作业" width="100" />
      </el-table>
    </el-drawer>

    <!-- 节点 Drawer -->
    <el-drawer v-model="nodeVisible" :title="`节点信息 - ${nodeTarget?.name ?? ''}`" size="760px">
      <el-table :data="nodesData" size="small" stripe>
        <el-table-column prop="name" label="节点名" width="150" />
        <el-table-column label="状态" width="110">
          <template #default="{ row }">
            <el-tag :type="statusType(row.status)" size="small">{{ statusText(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="partition" label="分区" width="120" />
        <el-table-column prop="cpu" label="CPU" width="80" />
        <el-table-column prop="gpu" label="GPU" width="80" />
        <el-table-column prop="memory" label="内存" width="100" />
        <el-table-column prop="gres" label="GRES" min-width="150" show-overflow-tooltip />
      </el-table>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import PageState from '@/components/PageState.vue'
import Can from '@/components/Can.vue'
import { schedulerApi } from '@/api'
import { useFetch } from '@/utils/useFetch'
import type { SchedulerIntegration } from '@/types'

const SCHEDULER_TYPE_OPTIONS = [
  { label: 'Slurm', value: 'slurm' },
  { label: 'LSF', value: 'lsf' },
  { label: 'SGE', value: 'sge' },
  { label: 'K8s Native', value: 'k8s_native' },
]
const AUTH_TYPE_OPTIONS = [
  { label: '无认证', value: 'none' },
  { label: 'SSH', value: 'ssh' },
  { label: 'Token', value: 'token' },
  { label: 'mTLS', value: 'mTLS' },
]
const SCHED_STATUS_OPTIONS = [
  { label: '活跃', value: 'active' },
  { label: '未启用', value: 'inactive' },
  { label: '错误', value: 'error' },
]

function typeLabel(v?: unknown) {
  const key = String(v ?? '')
  return SCHEDULER_TYPE_OPTIONS.find((o) => o.value === key)?.label ?? (key || '-')
}
function authLabel(v?: unknown) {
  const key = String(v ?? '')
  return AUTH_TYPE_OPTIONS.find((o) => o.value === key)?.label ?? (key || '-')
}

const STATUS_TYPE: Record<string, string> = {
  active: 'success', available: 'success', completed: 'success', healthy: 'success', running: 'primary',
  inactive: 'info', idle: 'info', pending: 'warning',
  error: 'danger', failed: 'danger', unhealthy: 'danger',
}
function statusType(s?: unknown) {
  return (STATUS_TYPE[String(s ?? '')] ?? 'info') as 'success' | 'warning' | 'danger' | 'info' | 'primary'
}
const STATUS_TEXT: Record<string, string> = {
  active: '活跃', inactive: '未启用', error: '错误', running: '运行中', pending: '排队中',
  completed: '已完成', failed: '失败', healthy: '健康', unhealthy: '异常', available: '可用',
}
function statusText(s?: unknown) {
  const key = String(s ?? '')
  return (key && STATUS_TEXT[key]) || key || '-'
}

// ---------- 筛选 ----------
const searchText = ref('')
const typeFilter = ref<string>('')
const statusFilter = ref<string>('')

// ---------- 数据 ----------
const schedulers = useFetch<SchedulerIntegration[]>(() => schedulerApi.list())
const schedulersData = computed(() => schedulers.data.value ?? [])

const filteredSchedulers = computed(() => {
  const kw = searchText.value.trim().toLowerCase()
  return schedulersData.value.filter((s) => {
    const matchKw = !kw || s.name.toLowerCase().includes(kw) || (s.endpoint ?? '').toLowerCase().includes(kw)
    const matchType = !typeFilter.value || s.type === typeFilter.value
    const matchStatus = !statusFilter.value || s.status === statusFilter.value
    return matchKw && matchType && matchStatus
  })
})

// 分页
const currentPage = ref(1)
const pageSize = 10
const pagedData = computed(() =>
  filteredSchedulers.value.slice((currentPage.value - 1) * pageSize, currentPage.value * pageSize),
)

// ---------- 表单 ----------
const formRef = ref<FormInstance>()
const dialogVisible = ref(false)
const editingScheduler = ref<SchedulerIntegration | null>(null)
const form = reactive({
  name: '',
  type: 'slurm' as SchedulerIntegration['type'],
  endpoint: '',
  auth_type: 'none' as SchedulerIntegration['auth_type'],
  auth_config: '',
  version: '',
  status: 'active' as SchedulerIntegration['status'],
  default_partition: '',
  default_qos: '',
  max_nodes: undefined as number | undefined,
  max_jobs: undefined as number | undefined,
  details: '',
})
const rules: FormRules = {
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择类型', trigger: 'change' }],
}

function resetForm() {
  Object.assign(form, {
    name: '', type: 'slurm', endpoint: '', auth_type: 'none', auth_config: '', version: '',
    status: 'active', default_partition: '', default_qos: '', max_nodes: undefined, max_jobs: undefined, details: '',
  })
}
function openCreate() {
  editingScheduler.value = null
  resetForm()
  dialogVisible.value = true
}
function openEdit(row: SchedulerIntegration) {
  editingScheduler.value = row
  Object.assign(form, {
    name: row.name, type: row.type, endpoint: row.endpoint ?? '', auth_type: row.auth_type ?? 'none',
    auth_config: row.auth_config ?? '', version: row.version ?? '', status: row.status ?? 'active',
    default_partition: row.default_partition ?? '', default_qos: row.default_qos ?? '',
    max_nodes: row.max_nodes, max_jobs: row.max_jobs, details: row.details ?? '',
  })
  dialogVisible.value = true
}
async function submitForm() {
  await formRef.value?.validate()
  try {
    if (editingScheduler.value) {
      await schedulerApi.update(editingScheduler.value.id, { ...form })
      ElMessage.success('调度器更新成功')
    } else {
      await schedulerApi.create({ ...form })
      ElMessage.success('调度器创建成功')
    }
    dialogVisible.value = false
    resetForm()
    schedulers.refetch()
  } catch {
    ElMessage.error('操作失败，请检查必填项后重试')
  }
}
async function handleDelete(id: number) {
  try {
    await schedulerApi.remove(id)
    ElMessage.success('调度器删除成功')
    schedulers.refetch()
  } catch {
    ElMessage.error('删除失败，请稍后重试')
  }
}

// ---------- 队列 / 节点 ----------
const queueVisible = ref(false)
const queueTarget = ref<SchedulerIntegration | null>(null)
const queues = useFetch<Record<string, unknown>[]>(() => schedulerApi.queues(queueTarget.value?.id ?? 0), false)
const queuesData = computed(() => queues.data.value ?? [])
function openQueues(row: SchedulerIntegration) {
  queueTarget.value = row
  queueVisible.value = true
  queues.refetch()
}

const nodeVisible = ref(false)
const nodeTarget = ref<SchedulerIntegration | null>(null)
const nodes = useFetch<Record<string, unknown>[]>(() => schedulerApi.nodes(nodeTarget.value?.id ?? 0), false)
const nodesData = computed(() => nodes.data.value ?? [])
function openNodes(row: SchedulerIntegration) {
  nodeTarget.value = row
  nodeVisible.value = true
  nodes.refetch()
}

// ---------- 同步作业 ----------
async function syncJobs(id: number) {
  try {
    await schedulerApi.syncJobs(id)
    ElMessage.success('作业同步已触发')
  } catch {
    ElMessage.error('同步失败，请稍后重试')
  }
}

// ---------- 健康检查 ----------
const healthCheckName = ref('')
const healthResult = ref<{ type: 'success' | 'danger'; label: string } | null>(null)
async function healthCheck(row: SchedulerIntegration) {
  try {
    const res = await schedulerApi.health(row.id)
    healthCheckName.value = row.name
    healthResult.value = res.healthy
      ? { type: 'success', label: 'healthy' }
      : { type: 'danger', label: 'unhealthy' }
    ElMessage.success(`已获取「${row.name}」健康状态`)
  } catch {
    ElMessage.error('健康检查失败，请稍后重试')
  }
}
</script>

<style scoped>
.toolbar { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; }
</style>
