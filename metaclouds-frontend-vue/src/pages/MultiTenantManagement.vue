<template>
  <div class="mc-page">
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">多租户管理</h1>
        <p class="mc-page-desc">共 {{ tenants.length }} 个租户 · 按租户划分命名空间与配额边界</p>
      </div>
      <div class="mc-page-head-extra">
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="openCreate">创建租户</el-button>
        </Can>
      </div>
    </div>

    <el-card>
      <!-- 搜索工具栏 -->
      <div class="mc-toolbar">
        <el-input
          v-model="keyword"
          placeholder="搜索租户名称 / 描述"
          clearable
          style="width: 280px"
          aria-label="搜索租户"
        />
        <el-select
          v-model="statusFilter"
          placeholder="状态筛选"
          clearable
          style="width: 160px"
        >
          <el-option label="活跃" value="active" />
          <el-option label="非活跃" value="inactive" />
          <el-option label="维护" value="maintenance" />
        </el-select>
      </div>

      <el-tabs v-model="activeTab" class="mc-mb">
        <el-tab-pane label="租户列表" name="tenants" />
        <el-tab-pane label="配额管理" name="quotas" />
      </el-tabs>

      <!-- ============ 租户列表 Tab ============ -->
      <PageState
        v-if="activeTab === 'tenants'"
        :loading="listLoading"
        :error="listError"
        :data="filteredTenants"
        empty-text="还没有租户，创建以划分命名空间与配额边界。"
        @retry="fetchTenants"
      >
        <el-table :data="pagedTenants" row-key="id" border stripe style="width: 100%">
          <el-table-column prop="id" label="ID" width="80">
            <template #default="{ row }"><span class="mc-mono">{{ row.id }}</span></template>
          </el-table-column>
          <el-table-column prop="name" label="名称" min-width="140" />
          <el-table-column prop="description" label="描述" min-width="160" show-overflow-tooltip />
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag :type="statusTagType(row.status)">{{ row.status || '-' }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="GPU配额" width="110">
            <template #default="{ row }"><span class="mc-num">{{ row.gpu_quota ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="CPU配额" width="110">
            <template #default="{ row }"><span class="mc-num">{{ row.cpu_quota ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="内存配额(GB)" width="120">
            <template #default="{ row }"><span class="mc-num">{{ row.memory_quota ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="存储配额(TB)" width="120">
            <template #default="{ row }"><span class="mc-num">{{ row.storage_quota ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="已用GPU" width="150">
            <template #default="{ row }">
              <div class="quota-usage">
                <el-progress
                  :percentage="gpuUsagePct(row)"
                  :status="gpuUsagePct(row) > 80 ? 'exception' : ''"
                  :stroke-width="10"
                />
                <span class="mc-num">{{ usedGpu(row) }}</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column label="已用CPU" width="150">
            <template #default="{ row }">
              <div class="quota-usage">
                <el-progress
                  :percentage="cpuUsagePct(row)"
                  :status="cpuUsagePct(row) > 80 ? 'exception' : ''"
                  :stroke-width="10"
                />
                <span class="mc-num">{{ usedCpu(row) }}</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="220" fixed="right">
            <template #default="{ row }">
              <Can :roles="['admin', 'manager']">
                <el-button link type="primary" size="small" @click="goQuota(row)">配额管理</el-button>
                <el-button link type="primary" size="small" @click="openEdit(row)">编辑</el-button>
                <el-button link type="danger" size="small" @click="handleDelete(row)">删除</el-button>
              </Can>
            </template>
          </el-table-column>
        </el-table>
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :total="filteredTenants.length"
          :page-sizes="[10, 20, 50]"
          layout="total, sizes, prev, pager, next"
          style="margin-top: 12px; justify-content: flex-end"
        />
      </PageState>

      <!-- ============ 配额管理 Tab（多维度配额：租户/用户/分区/节点） ============ -->
      <div v-else>
        <div class="mc-flow mc-mb">
          <el-select v-model="quotaScope" placeholder="配额维度" style="width: 150px" @change="loadQuotas">
            <el-option label="租户" value="tenant" />
            <el-option label="用户" value="user" />
            <el-option label="分区" value="partition" />
            <el-option label="节点" value="node" />
          </el-select>
          <el-input v-model="quotaScopeId" placeholder="维度ID（如租户ID）" style="width: 200px" @change="loadQuotas" />
          <Can :roles="['admin', 'manager']">
            <el-button type="primary" @click="openQuotaCreate">新增配额</el-button>
          </Can>
        </div>
        <PageState
          :loading="quotaLoading"
          :error="quotaError"
          :data="quotas"
          empty-text="暂无配额，为该维度新增 GPU / CPU / 内存 / 存储资源配额。"
          @retry="loadQuotas"
        >
          <el-table :data="quotas" row-key="id" border stripe style="width: 100%">
            <el-table-column prop="id" label="ID" width="70">
              <template #default="{ row }"><span class="mc-mono">{{ row.id }}</span></template>
            </el-table-column>
            <el-table-column label="维度" width="90">
              <template #default="{ row }">{{ scopeLabel(row.scope_type) }}</template>
            </el-table-column>
            <el-table-column label="资源类型" width="100">
              <template #default="{ row }">
                <el-tag type="primary" size="small">{{ resourceTypeLabel(row.resource_type) }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="限制" width="90">
              <template #default="{ row }"><span class="mc-num">{{ row.limit }}</span></template>
            </el-table-column>
            <el-table-column label="已用" width="90">
              <template #default="{ row }"><span class="mc-num">{{ row.used }}</span></template>
            </el-table-column>
            <el-table-column label="使用率" width="160">
              <template #default="{ row }">
                <el-progress :percentage="quotaUsagePct(row)" :status="quotaUsagePct(row) > 80 ? 'exception' : ''" :stroke-width="10" />
              </template>
            </el-table-column>
            <el-table-column label="GPU分数限制" width="110">
              <template #default="{ row }">{{ row.gpu_fraction_limit ?? '-' }}</template>
            </el-table-column>
            <el-table-column label="GPU显存(GB)" width="120">
              <template #default="{ row }">{{ row.gpu_memory_limit_gb ?? '-' }}</template>
            </el-table-column>
            <el-table-column label="单Pod CPU(min/max)" width="150">
              <template #default="{ row }">{{ row.min_pod_cpu ?? '-' }} / {{ row.max_pod_cpu ?? '-' }}</template>
            </el-table-column>
            <el-table-column label="单Pod GPU(min/max)" width="150">
              <template #default="{ row }">{{ row.min_pod_gpu ?? '-' }} / {{ row.max_pod_gpu ?? '-' }}</template>
            </el-table-column>
            <el-table-column label="操作" width="140" fixed="right">
              <template #default="{ row }">
                <Can :roles="['admin', 'manager']">
                  <el-button link type="primary" size="small" @click="openQuotaEdit(row)">编辑</el-button>
                  <el-popconfirm title="删除该配额？" confirm-button-text="确认" cancel-button-text="取消" @confirm="handleDeleteQuota(row.id)">
                    <template #reference>
                      <el-button link type="danger" size="small">删除</el-button>
                    </template>
                  </el-popconfirm>
                </Can>
              </template>
            </el-table-column>
          </el-table>
        </PageState>
      </div>
    </el-card>

    <!-- 创建 / 编辑租户 Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="editing ? '编辑租户' : '创建租户'"
      width="640px"
      destroy-on-close
    >
      <el-form
        ref="formRef"
        :model="form"
        :rules="rules"
        label-width="120px"
        label-position="top"
      >
        <el-form-item label="名称" prop="name">
          <el-input v-model="form.name" placeholder="例如：team-vision" />
        </el-form-item>
        <el-form-item label="描述">
          <el-input v-model="form.description" type="textarea" :rows="3" placeholder="选填，便于后续追溯" />
        </el-form-item>
        <el-form-item label="状态" prop="status">
          <el-select v-model="form.status" placeholder="请选择状态" style="width: 100%">
            <el-option label="活跃" value="active" />
            <el-option label="非活跃" value="inactive" />
            <el-option label="维护" value="maintenance" />
          </el-select>
        </el-form-item>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="GPU配额" prop="gpu_quota">
              <el-input-number v-model="form.gpu_quota" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="CPU配额" prop="cpu_quota">
              <el-input-number v-model="form.cpu_quota" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="内存配额(GB)" prop="memory_quota">
              <el-input-number v-model="form.memory_quota" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="存储配额(TB)">
              <el-input-number v-model="form.storage_quota" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" :loading="saving" @click="handleSubmit">
            {{ editing ? '保存' : '创建' }}
          </el-button>
        </Can>
        <el-button @click="dialogVisible = false">取消</el-button>
      </template>
    </el-dialog>

    <!-- 配额 新增/编辑 Dialog -->
    <el-dialog
      v-model="quotaDialogVisible"
      :title="editingQuota ? '编辑配额' : '新增配额'"
      width="680px"
      destroy-on-close
    >
      <el-form ref="quotaFormRef" :model="quotaForm" :rules="quotaRules" label-position="top">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="资源类型" prop="resource_type">
              <el-select v-model="quotaForm.resource_type" style="width: 100%">
                <el-option v-for="o in RESOURCE_TYPE_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="限制值" prop="limit">
              <el-input-number v-model="quotaForm.limit" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="GPU分数限制">
              <el-input-number v-model="quotaForm.gpu_fraction_limit" :min="0" :max="1" :step="0.25" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="GPU显存限制(GB)">
              <el-input-number v-model="quotaForm.gpu_memory_limit_gb" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="单Pod最大CPU">
              <el-input-number v-model="quotaForm.max_pod_cpu" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="单Pod最小CPU">
              <el-input-number v-model="quotaForm.min_pod_cpu" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="单Pod最大内存">
              <el-input-number v-model="quotaForm.max_pod_memory" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="单Pod最小内存">
              <el-input-number v-model="quotaForm.min_pod_memory" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="单Pod最大GPU">
              <el-input-number v-model="quotaForm.max_pod_gpu" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="单Pod最小GPU">
              <el-input-number v-model="quotaForm.min_pod_gpu" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="quotaDialogVisible = false">取消</el-button>
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="handleSubmitQuota">{{ editingQuota ? '保存' : '创建' }}</el-button>
        </Can>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, reactive } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import PageState from '@/components/PageState.vue'
import Can from '@/components/Can.vue'
import { tenantApi, quotaApi } from '@/api'
import type { Tenant, ResourceQuota } from '@/types'

interface TenantForm {
  name: string
  description: string
  status: string
  gpu_quota: number | undefined
  cpu_quota: number | undefined
  memory_quota: number | undefined
  storage_quota: number | undefined
}

const tenants = ref<Tenant[]>([])
const listLoading = ref(false)
const listError = ref('')
const keyword = ref('')
const statusFilter = ref('')
const page = ref(1)
const pageSize = ref(10)

async function fetchTenants() {
  listLoading.value = true
  listError.value = ''
  try {
    tenants.value = await tenantApi.list()
  } catch (e) {
    listError.value = e instanceof Error ? e.message : '加载租户失败'
    tenants.value = []
  } finally {
    listLoading.value = false
  }
}
fetchTenants()

const filteredTenants = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  return tenants.value.filter((t) => {
    const matchKw =
      !kw ||
      t.name.toLowerCase().includes(kw) ||
      (t.description ?? '').toLowerCase().includes(kw)
    const matchStatus = !statusFilter.value || t.status === statusFilter.value
    return matchKw && matchStatus
  })
})

const pagedTenants = computed(() => {
  const start = (page.value - 1) * pageSize.value
  return filteredTenants.value.slice(start, start + pageSize.value)
})

// 三态守卫：PageState 会根据 loading/error/data 自动切换 loading / error / empty / 内容
const dialogVisible = ref(false)
const editing = ref<Tenant | null>(null)
const saving = ref(false)
const formRef = ref<FormInstance>()

const emptyForm = (): TenantForm => ({
  name: '',
  description: '',
  status: 'active',
  gpu_quota: undefined,
  cpu_quota: undefined,
  memory_quota: undefined,
  storage_quota: undefined,
})

const form = reactive<TenantForm>(emptyForm())

const rules: FormRules<TenantForm> = {
  name: [{ required: true, message: '请输入租户名称', trigger: 'blur' }],
  status: [{ required: true, message: '请选择状态', trigger: 'change' }],
  gpu_quota: [{ required: true, message: '请输入 GPU 配额', trigger: 'blur' }],
  cpu_quota: [{ required: true, message: '请输入 CPU 配额', trigger: 'blur' }],
  memory_quota: [{ required: true, message: '请输入内存配额', trigger: 'blur' }],
}

function openCreate() {
  editing.value = null
  Object.assign(form, emptyForm())
  dialogVisible.value = true
}

function openEdit(row: Tenant) {
  editing.value = row
  Object.assign(form, {
    name: row.name,
    description: row.description ?? '',
    status: row.status,
    gpu_quota: row.gpu_quota,
    cpu_quota: row.cpu_quota,
    memory_quota: row.memory_quota,
    storage_quota: row.storage_quota,
  })
  dialogVisible.value = true
}

async function handleSubmit() {
  if (!formRef.value) return
  await formRef.value.validate()
  saving.value = true
  try {
    const payload = { ...form }
    if (editing.value) {
      await tenantApi.update(editing.value.id, payload)
      ElMessage.success('租户更新成功')
    } else {
      await tenantApi.create(payload)
      ElMessage.success('租户创建成功')
    }
    dialogVisible.value = false
    await fetchTenants()
  } catch {
    ElMessage.error(editing.value ? '租户更新失败，请检查必填项后重试' : '租户创建失败，请检查必填项后重试')
  } finally {
    saving.value = false
  }
}

async function handleDelete(row: Tenant) {
  try {
    await ElMessageBox.confirm(
      '租户下的配额与关联资源将一并移除，且不可恢复。',
      '删除该租户？',
      { type: 'warning', confirmButtonText: '确认删除', cancelButtonText: '返回', confirmButtonClass: 'el-button--danger' },
    )
  } catch {
    return // 用户取消
  }
  try {
    await tenantApi.remove(row.id)
    ElMessage.success('租户删除成功')
    await fetchTenants()
  } catch {
    ElMessage.error('租户删除失败，请稍后重试')
  }
}

// 配额使用进度条（后端返回的 used 字段做进度；缺失时回退为 0）
function num(row: Tenant, key: string): number {
  const v = (row as unknown as Record<string, unknown>)[key]
  return typeof v === 'number' ? v : 0
}
const usedGpu = (row: Tenant) => num(row, 'used_gpu')
const usedCpu = (row: Tenant) => num(row, 'used_cpu')
function gpuUsagePct(row: Tenant): number {
  const q = row.gpu_quota ?? 0
  return q > 0 ? Math.round((usedGpu(row) / q) * 100) : 0
}
function cpuUsagePct(row: Tenant): number {
  const q = row.cpu_quota ?? 0
  return q > 0 ? Math.round((usedCpu(row) / q) * 100) : 0
}

// ---------- 多维度配额管理（租户/用户/分区/节点） ----------
const activeTab = ref<'tenants' | 'quotas'>('tenants')
const quotaScope = ref<'tenant' | 'user' | 'partition' | 'node'>('tenant')
const quotaScopeId = ref<number | ''>('')
const quotas = ref<ResourceQuota[]>([])
const quotaLoading = ref(false)
const quotaError = ref('')

const RESOURCE_TYPE_OPTIONS = [
  { label: 'GPU', value: 'gpu' },
  { label: 'CPU', value: 'cpu' },
  { label: '内存', value: 'memory' },
  { label: '存储', value: 'storage' },
]
function resourceTypeLabel(v?: string) {
  return RESOURCE_TYPE_OPTIONS.find((o) => o.value === v)?.label ?? v ?? '-'
}
function scopeLabel(v?: string) {
  const map: Record<string, string> = { tenant: '租户', user: '用户', partition: '分区', node: '节点' }
  return map[v ?? ''] ?? v ?? '-'
}

async function loadQuotas() {
  quotaLoading.value = true
  quotaError.value = ''
  try {
    quotas.value = await quotaApi.list({
      scope_type: quotaScope.value,
      scope_id: quotaScopeId.value === '' ? undefined : Number(quotaScopeId.value),
    })
  } catch (e) {
    quotaError.value = e instanceof Error ? e.message : '加载配额失败'
    quotas.value = []
  } finally {
    quotaLoading.value = false
  }
}

function goQuota(row: Tenant) {
  activeTab.value = 'quotas'
  quotaScope.value = 'tenant'
  quotaScopeId.value = row.id
  void loadQuotas()
}

function quotaUsagePct(row: ResourceQuota): number {
  return row.limit > 0 ? Math.round((row.used / row.limit) * 100) : 0
}

const quotaDialogVisible = ref(false)
const editingQuota = ref<ResourceQuota | null>(null)
const quotaFormRef = ref<FormInstance>()
const quotaForm = reactive({
  resource_type: 'gpu' as ResourceQuota['resource_type'],
  limit: 1,
  gpu_fraction_limit: undefined as number | undefined,
  gpu_memory_limit_gb: undefined as number | undefined,
  max_pod_cpu: undefined as number | undefined,
  min_pod_cpu: undefined as number | undefined,
  max_pod_memory: undefined as number | undefined,
  min_pod_memory: undefined as number | undefined,
  max_pod_gpu: undefined as number | undefined,
  min_pod_gpu: undefined as number | undefined,
})
const quotaRules: FormRules = {
  resource_type: [{ required: true, message: '请选择资源类型', trigger: 'change' }],
  limit: [{ required: true, message: '请输入限制值', trigger: 'blur' }],
}

function openQuotaCreate() {
  editingQuota.value = null
  Object.assign(quotaForm, {
    resource_type: 'gpu', limit: 1, gpu_fraction_limit: undefined, gpu_memory_limit_gb: undefined,
    max_pod_cpu: undefined, min_pod_cpu: undefined, max_pod_memory: undefined, min_pod_memory: undefined,
    max_pod_gpu: undefined, min_pod_gpu: undefined,
  })
  quotaDialogVisible.value = true
}
function openQuotaEdit(row: ResourceQuota) {
  editingQuota.value = row
  Object.assign(quotaForm, {
    resource_type: row.resource_type, limit: row.limit, gpu_fraction_limit: row.gpu_fraction_limit,
    gpu_memory_limit_gb: row.gpu_memory_limit_gb, max_pod_cpu: row.max_pod_cpu, min_pod_cpu: row.min_pod_cpu,
    max_pod_memory: row.max_pod_memory, min_pod_memory: row.min_pod_memory,
    max_pod_gpu: row.max_pod_gpu, min_pod_gpu: row.min_pod_gpu,
  })
  quotaDialogVisible.value = true
}
async function handleSubmitQuota() {
  if (!quotaFormRef.value) return
  await quotaFormRef.value.validate()
  try {
    if (editingQuota.value) {
      await quotaApi.update(editingQuota.value.id, { ...quotaForm })
      ElMessage.success('配额更新成功')
    } else {
      await quotaApi.create({ ...quotaForm, scope_type: quotaScope.value, scope_id: Number(quotaScopeId.value) || 0, used: 0 })
      ElMessage.success('配额创建成功')
    }
    quotaDialogVisible.value = false
    await loadQuotas()
  } catch {
    ElMessage.error('配额保存失败，请检查必填项后重试')
  }
}
async function handleDeleteQuota(id: number) {
  try {
    await quotaApi.remove(id)
    ElMessage.success('配额删除成功')
    await loadQuotas()
  } catch {
    ElMessage.error('配额删除失败，请稍后重试')
  }
}

function statusTagType(status?: string) {
  switch (status) {
    case 'active':
    case 'running':
      return 'success'
    case 'inactive':
    case 'paused':
      return 'info'
    case 'maintenance':
    case 'pending':
      return 'warning'
    case 'error':
    case 'failed':
      return 'danger'
    default:
      return 'info'
  }
}
</script>

<style scoped>
.mc-toolbar {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
.quota-usage {
  display: flex;
  align-items: center;
  gap: 8px;
}
.quota-usage :deep(.el-progress) {
  flex: 1;
}
.mc-mono {
  font-family: var(--mc-mono-font, monospace);
}
.mc-num {
  font-variant-numeric: tabular-nums;
}
</style>
