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

      <PageState
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
          <el-table-column label="操作" width="200" fixed="right">
            <template #default="{ row }">
              <Can :roles="['admin', 'manager']">
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
  </div>
</template>

<script setup lang="ts">
import { ref, computed, reactive } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import PageState from '@/components/PageState.vue'
import Can from '@/components/Can.vue'
import { tenantApi } from '@/api'
import type { Tenant } from '@/types'

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
