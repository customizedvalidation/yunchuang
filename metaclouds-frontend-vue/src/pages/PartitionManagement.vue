<template>
  <div class="mc-page">
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">分区管理</h1>
        <p class="mc-page-desc">共 {{ partitionsData.length }} 个分区 · 管理计算资源边界、优先级与权限</p>
      </div>
      <div class="mc-page-head-extra">
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="openCreate">创建分区</el-button>
        </Can>
      </div>
    </div>

    <el-card shadow="never" id="partition-table">
      <!-- 搜索 + 集群 / 状态筛选 -->
      <div class="toolbar mc-mb">
        <el-input v-model="searchText" placeholder="搜索分区名称 / 描述" clearable style="flex: 1; min-width: 220px" />
        <el-select v-model="clusterFilter" placeholder="集群" clearable style="width: 160px">
          <el-option v-for="c in clustersData" :key="c.id" :label="c.name" :value="c.id" />
        </el-select>
        <el-select v-model="statusFilter" placeholder="状态" clearable style="width: 140px">
          <el-option v-for="o in PARTITION_STATUS_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
      </div>

      <PageState
        :loading="partitions.loading.value"
        :error="partitions.error.value"
        :data="pagedData"
        empty-text="暂无分区，创建后可划分计算资源边界"
        @retry="partitions.refetch"
      >
        <el-table :data="pagedData" stripe border style="width: 100%">
          <el-table-column label="ID" width="70">
            <template #default="{ row }"><span class="mc-mono">{{ row.id }}</span></template>
          </el-table-column>
          <el-table-column prop="name" label="名称" width="140" />
          <el-table-column label="集群" width="120">
            <template #default="{ row }">{{ clusterName(row.cluster_id) }}</template>
          </el-table-column>
          <el-table-column prop="description" label="描述" show-overflow-tooltip />
          <el-table-column label="优先级" width="80">
            <template #default="{ row }"><span class="mc-num">{{ row.priority }}</span></template>
          </el-table-column>
          <el-table-column label="最大运行(分钟)" width="130">
            <template #default="{ row }">
              <span v-if="!row.max_runtime_minutes" class="mc-empty-mini">无限</span>
              <span v-else class="mc-num">{{ row.max_runtime_minutes }}</span>
            </template>
          </el-table-column>
          <el-table-column label="节点数" width="80">
            <template #default="{ row }"><span class="mc-num">{{ row.node_count ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="CPU限制" width="90">
            <template #default="{ row }"><span class="mc-num">{{ row.cpu_limit ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="GPU数" width="80">
            <template #default="{ row }"><span class="mc-num">{{ row.gpu_count ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="GPU厂商" width="100">
            <template #default="{ row }"><el-tag size="small" v-if="row.gpu_vendor">{{ row.gpu_vendor }}</el-tag><span v-else>-</span></template>
          </el-table-column>
          <el-table-column label="调度器类型" width="120">
            <template #default="{ row }">
              <el-tag v-if="schedulerOpt(row.scheduler_type)" type="primary" size="small">{{ schedulerOpt(row.scheduler_type) }}</el-tag>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag :type="statusType(row.status)" size="small">{{ statusText(row.status) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="允许共享" width="90">
            <template #default="{ row }">
              <el-tag :type="row.allow_sharing ? 'success' : 'info'" size="small">{{ row.allow_sharing ? '是' : '否' }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="default_qos" label="默认QoS" width="100" />
          <el-table-column label="操作" width="300" fixed="right">
            <template #default="{ row }">
              <Can :roles="['admin', 'manager']">
                <el-button link type="primary" size="small" @click="openEdit(row)">编辑</el-button>
                <el-button link type="primary" size="small" @click="openPriority(row)">优先级</el-button>
                <el-button link type="primary" size="small" @click="openRuntime(row)">运行时长</el-button>
                <el-button link type="primary" size="small" @click="openPerm(row)">权限</el-button>
                <el-popconfirm title="删除该分区？分区配置与权限将一并移除且不可恢复。" confirm-button-text="确认删除" cancel-button-text="返回" @confirm="handleDelete(row.id)">
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
          :total="filteredPartitions.length"
          :page-size="pageSize"
          style="margin-top: 12px; justify-content: flex-end"
        />
      </PageState>
    </el-card>

    <!-- 新建/编辑 Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="editingPartition ? '编辑分区' : '创建分区'"
      width="760px"
      destroy-on-close
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-position="top">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="名称" prop="name">
              <el-input v-model="form.name" placeholder="例如：gpu-high" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="集群" prop="cluster_id">
              <el-select v-model="form.cluster_id" placeholder="请选择集群" style="width: 100%">
                <el-option v-for="c in clustersData" :key="c.id" :label="c.name" :value="c.id" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="描述" prop="description">
              <el-input v-model="form.description" type="textarea" :rows="2" placeholder="选填" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="优先级 (0-100)" prop="priority">
              <el-slider v-model="form.priority" :min="0" :max="100" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="最大运行时长(分钟，0=无限)" prop="max_runtime_minutes">
              <el-input-number v-model="form.max_runtime_minutes" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="节点数" prop="node_count">
              <el-input-number v-model="form.node_count" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="CPU 限制" prop="cpu_limit">
              <el-input-number v-model="form.cpu_limit" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="GPU 数" prop="gpu_count">
              <el-input-number v-model="form.gpu_count" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="GPU 厂商" prop="gpu_vendor">
              <el-select v-model="form.gpu_vendor" placeholder="请选择厂商" clearable style="width: 100%">
                <el-option v-for="o in VENDOR_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="调度器类型" prop="scheduler_type">
              <el-select v-model="form.scheduler_type" placeholder="请选择调度器" clearable style="width: 100%">
                <el-option v-for="o in SCHEDULER_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="状态" prop="status">
              <el-select v-model="form.status" placeholder="请选择状态" style="width: 100%">
                <el-option v-for="o in PARTITION_STATUS_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="默认 QoS" prop="default_qos">
              <el-input v-model="form.default_qos" placeholder="例如：normal" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="允许资源共享" prop="allow_sharing">
              <el-switch v-model="form.allow_sharing" />
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="节点列表 (JSON)" prop="nodes">
              <el-input v-model="form.nodes" type="textarea" :rows="2" placeholder='例如：["node-01","node-02"]' />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="Taints (JSON)" prop="taints">
              <el-input v-model="form.taints" type="textarea" :rows="2" placeholder='例如：[{"key":"gpu","value":"nvidia","effect":"NoSchedule"}]' />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="Labels (JSON)" prop="labels">
              <el-input v-model="form.labels" type="textarea" :rows="2" placeholder='例如：{"gpu-type":"a100"}' />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="submitForm">{{ editingPartition ? '保存' : '创建' }}</el-button>
        </Can>
      </template>
    </el-dialog>

    <!-- 优先级调整 Dialog -->
    <el-dialog v-model="priorityVisible" title="调整分区优先级" width="480px">
      <p style="margin-bottom: 16px">分区：<b>{{ priorityTarget?.name }}</b></p>
      <el-slider v-model="priorityValue" :min="0" :max="100" />
      <div style="text-align: center; margin-top: 8px">当前值：<span class="mc-num">{{ priorityValue }}</span></div>
      <template #footer>
        <el-button @click="priorityVisible = false">取消</el-button>
        <el-button type="primary" @click="savePriority">保存</el-button>
      </template>
    </el-dialog>

    <!-- 最大运行时长 Dialog -->
    <el-dialog v-model="runtimeVisible" title="修改最大运行时长" width="480px">
      <p style="margin-bottom: 16px">分区：<b>{{ runtimeTarget?.name }}</b></p>
      <el-input-number v-model="runtimeValue" :min="0" style="width: 100%" />
      <p style="margin-top: 8px; color: var(--mc-text-3); font-size: 12px">0 表示不限制运行时长（分钟）</p>
      <template #footer>
        <el-button @click="runtimeVisible = false">取消</el-button>
        <el-button type="primary" @click="saveRuntime">保存</el-button>
      </template>
    </el-dialog>

    <!-- 权限管理 Drawer -->
    <el-drawer v-model="permVisible" :title="`分区权限管理 - ${permTarget?.name ?? ''}`" size="620px">
      <el-form :inline="true" :model="permForm" @submit.prevent style="margin-bottom: 16px">
        <el-form-item label="主体类型">
          <el-select v-model="permForm.principal_type" style="width: 110px">
            <el-option label="用户" value="user" />
            <el-option label="用户组" value="group" />
          </el-select>
        </el-form-item>
        <el-form-item label="主体ID">
          <el-input-number v-model="permForm.principal_id" :min="1" style="width: 110px" />
        </el-form-item>
        <el-form-item label="访问级别">
          <el-select v-model="permForm.access_level" style="width: 130px">
            <el-option label="查看" value="view" />
            <el-option label="提交" value="submit" />
            <el-option label="管理" value="admin" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <Can :roles="['admin', 'manager']">
            <el-button type="primary" @click="addPermission">添加</el-button>
          </Can>
        </el-form-item>
      </el-form>

      <el-table :data="permissionsData" size="small" stripe>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column label="主体类型" width="110">
          <template #default="{ row }">
            <el-tag size="small">{{ row.principal_type === 'user' ? '用户' : '用户组' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="principal_id" label="主体ID" width="100" />
        <el-table-column label="访问级别" width="110">
          <template #default="{ row }">
            <el-tag :type="accessType(row.access_level)" size="small">{{ row.access_level }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="90">
          <template #default="{ row }">
            <Can :roles="['admin', 'manager']">
              <el-popconfirm title="移除此权限？" confirm-button-text="确认" cancel-button-text="取消" @confirm="removePermission(row.id)">
                <template #reference>
                  <el-button link type="danger" size="small">移除</el-button>
                </template>
              </el-popconfirm>
            </Can>
          </template>
        </el-table-column>
      </el-table>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import PageState from '@/components/PageState.vue'
import Can from '@/components/Can.vue'
import { partitionApi, clusterApi } from '@/api'
import { useFetch } from '@/utils/useFetch'
import type { Partition, PartitionPermission, GPUVendor, Cluster } from '@/types'

const SCHEDULER_OPTIONS = [
  { label: 'K8s Native', value: 'k8s_native' },
  { label: 'Slurm', value: 'slurm' },
  { label: 'LSF', value: 'lsf' },
  { label: 'SGE', value: 'sge' },
]
const VENDOR_OPTIONS: { label: string; value: GPUVendor }[] = [
  { label: 'NVIDIA', value: 'nvidia' },
  { label: '燧原', value: 'enflame' },
  { label: '摩尔线程', value: 'moore_threads' },
  { label: '国产 X', value: 'domestic_x' },
]
const PARTITION_STATUS_OPTIONS = [
  { label: '活跃', value: 'active' },
  { label: '未启用', value: 'inactive' },
  { label: '维护', value: 'maintenance' },
  { label: '已排空', value: 'drained' },
]

const STATUS_TYPE: Record<string, string> = {
  active: 'success', available: 'success', completed: 'success', healthy: 'success',
  inactive: 'info', disabled: 'info', drained: 'warning', maintenance: 'warning',
  failed: 'danger', error: 'danger',
}
function statusType(s?: string) {
  return (STATUS_TYPE[s ?? ''] ?? 'info') as 'success' | 'warning' | 'danger' | 'info' | 'primary'
}
const STATUS_TEXT: Record<string, string> = {
  active: '活跃', inactive: '未启用', maintenance: '维护', drained: '已排空',
}
function statusText(s?: string) {
  return (s && STATUS_TEXT[s]) ?? s ?? '-'
}
function schedulerOpt(v?: string) {
  return SCHEDULER_OPTIONS.find((o) => o.value === v)?.label
}
function accessType(v: string) {
  return v === 'admin' ? 'danger' : v === 'submit' ? 'primary' : 'info'
}

// ---------- 筛选 ----------
const searchText = ref('')
const clusterFilter = ref<number | undefined>(undefined)
const statusFilter = ref<string>('')

// ---------- 数据 ----------
const partitions = useFetch<Partition[]>(() =>
  partitionApi.list({ cluster_id: clusterFilter.value, status: statusFilter.value || undefined }),
)
const clusters = useFetch<Cluster[]>(() => clusterApi.list())

watch([clusterFilter, statusFilter], () => partitions.refetch())

const partitionsData = computed(() => partitions.data.value ?? [])
const clustersData = computed(() => clusters.data.value ?? [])

function clusterName(id?: number) {
  return clustersData.value.find((c) => c.id === id)?.name ?? '-'
}

const filteredPartitions = computed(() => {
  const kw = searchText.value.trim().toLowerCase()
  return partitionsData.value.filter((p) => {
    const matchKw = !kw || p.name.toLowerCase().includes(kw) || (p.description ?? '').toLowerCase().includes(kw)
    return matchKw
  })
})

// 分页
const currentPage = ref(1)
const pageSize = 10
const pagedData = computed(() =>
  filteredPartitions.value.slice((currentPage.value - 1) * pageSize, currentPage.value * pageSize),
)
watch(filteredPartitions, () => (currentPage.value = 1))

// ---------- 表单 ----------
const formRef = ref<FormInstance>()
const dialogVisible = ref(false)
const editingPartition = ref<Partition | null>(null)
const form = reactive({
  name: '',
  cluster_id: undefined as number | undefined,
  description: '',
  priority: 50,
  max_runtime_minutes: 0,
  node_count: undefined as number | undefined,
  cpu_limit: undefined as number | undefined,
  gpu_count: undefined as number | undefined,
  gpu_vendor: undefined as GPUVendor | undefined,
  scheduler_type: undefined as Partition['scheduler_type'],
  status: 'active' as Partition['status'],
  allow_sharing: false,
  default_qos: '',
  nodes: '',
  taints: '',
  labels: '',
})
const rules: FormRules = {
  name: [{ required: true, message: '请输入分区名称', trigger: 'blur' }],
  cluster_id: [{ required: true, message: '请选择集群', trigger: 'change' }],
}

function resetForm() {
  Object.assign(form, {
    name: '', cluster_id: undefined, description: '', priority: 50, max_runtime_minutes: 0,
    node_count: undefined, cpu_limit: undefined, gpu_count: undefined, gpu_vendor: undefined,
    scheduler_type: undefined, status: 'active', allow_sharing: false, default_qos: '',
    nodes: '', taints: '', labels: '',
  })
}
function openCreate() {
  editingPartition.value = null
  resetForm()
  dialogVisible.value = true
}
function openEdit(row: Partition) {
  editingPartition.value = row
  Object.assign(form, {
    name: row.name, cluster_id: row.cluster_id, description: row.description ?? '',
    priority: row.priority ?? 50, max_runtime_minutes: row.max_runtime_minutes ?? 0,
    node_count: row.node_count, cpu_limit: row.cpu_limit, gpu_count: row.gpu_count,
    gpu_vendor: row.gpu_vendor, scheduler_type: row.scheduler_type, status: row.status ?? 'active',
    allow_sharing: row.allow_sharing ?? false, default_qos: row.default_qos ?? '',
    nodes: row.nodes ?? '', taints: row.taints ?? '', labels: row.labels ?? '',
  })
  dialogVisible.value = true
}
async function submitForm() {
  await formRef.value?.validate()
  try {
    if (editingPartition.value) {
      await partitionApi.update(editingPartition.value.id, { ...form })
      ElMessage.success('分区更新成功')
    } else {
      await partitionApi.create({ ...form })
      ElMessage.success('分区创建成功')
    }
    dialogVisible.value = false
    resetForm()
    partitions.refetch()
  } catch {
    ElMessage.error('操作失败，请检查必填项后重试')
  }
}
async function handleDelete(id: number) {
  try {
    await partitionApi.remove(id)
    ElMessage.success('分区删除成功')
    partitions.refetch()
  } catch {
    ElMessage.error('删除失败，请稍后重试')
  }
}

// ---------- 优先级 ----------
const priorityVisible = ref(false)
const priorityTarget = ref<Partition | null>(null)
const priorityValue = ref(50)
function openPriority(row: Partition) {
  priorityTarget.value = row
  priorityValue.value = row.priority ?? 50
  priorityVisible.value = true
}
async function savePriority() {
  if (!priorityTarget.value) return
  try {
    await partitionApi.updatePriority(priorityTarget.value.id, priorityValue.value)
    ElMessage.success('优先级更新成功')
    priorityVisible.value = false
    partitions.refetch()
  } catch {
    ElMessage.error('操作失败，请稍后重试')
  }
}

// ---------- 最大运行时长 ----------
const runtimeVisible = ref(false)
const runtimeTarget = ref<Partition | null>(null)
const runtimeValue = ref(0)
function openRuntime(row: Partition) {
  runtimeTarget.value = row
  runtimeValue.value = row.max_runtime_minutes ?? 0
  runtimeVisible.value = true
}
async function saveRuntime() {
  if (!runtimeTarget.value) return
  try {
    await partitionApi.updateMaxRuntime(runtimeTarget.value.id, runtimeValue.value)
    ElMessage.success('最大运行时长更新成功')
    runtimeVisible.value = false
    partitions.refetch()
  } catch {
    ElMessage.error('操作失败，请稍后重试')
  }
}

// ---------- 权限 ----------
const permVisible = ref(false)
const permTarget = ref<Partition | null>(null)
const permissions = useFetch<PartitionPermission[]>(
  () => partitionApi.permissions(permTarget.value?.id ?? 0),
  false,
)
const permissionsData = computed(() => permissions.data.value ?? [])
const permForm = reactive({
  principal_type: 'user' as 'user' | 'group',
  principal_id: 1 as number,
  access_level: 'view' as 'view' | 'submit' | 'admin',
})
function openPerm(row: Partition) {
  permTarget.value = row
  permVisible.value = true
  Object.assign(permForm, { principal_type: 'user', principal_id: 1, access_level: 'view' })
  permissions.refetch()
}
async function addPermission() {
  if (!permTarget.value) return
  try {
    await partitionApi.setPermission(permTarget.value.id, { ...permForm })
    ElMessage.success('权限添加成功')
    Object.assign(permForm, { principal_type: 'user', principal_id: 1, access_level: 'view' })
    permissions.refetch()
  } catch {
    ElMessage.error('添加失败，请稍后重试')
  }
}
async function removePermission(id: number) {
  try {
    await partitionApi.removePermission(id)
    ElMessage.success('权限已移除')
    permissions.refetch()
  } catch {
    ElMessage.error('移除失败，请稍后重试')
  }
}
</script>

<style scoped>
.toolbar { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; }
.mc-empty-mini { color: var(--mc-text-3); }
</style>
