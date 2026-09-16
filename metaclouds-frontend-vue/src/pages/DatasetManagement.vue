<template>
  <div class="mc-page">
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">数据集与 Fluid 缓存管理</h1>
        <p class="mc-page-desc">共 {{ datasets.length }} 个数据集 · 分布式缓存加速训练数据读取</p>
      </div>
      <div class="mc-page-head-extra">
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="openCreate">新增数据集</el-button>
        </Can>
      </div>
    </div>

    <el-card>
      <!-- 搜索 + 源类型/状态筛选 -->
      <div class="mc-toolbar">
        <el-input
          v-model="keyword"
          placeholder="搜索名称 / 路径"
          clearable
          style="width: 240px"
          aria-label="搜索数据集"
        />
        <el-select v-model="sourceTypeFilter" placeholder="源类型筛选" clearable style="width: 170px">
          <el-option v-for="o in sourceTypeOptions" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
        <el-select v-model="statusFilter" placeholder="状态筛选" clearable style="width: 150px">
          <el-option label="活跃" value="active" />
          <el-option label="非活跃" value="inactive" />
          <el-option label="维护" value="maintenance" />
        </el-select>
      </div>

      <PageState
        :loading="listLoading"
        :error="listError"
        :data="filteredDatasets"
        :empty-text="keyword ? '没有匹配的数据集，换个关键词试试。' : '暂无数据集，登记数据集以配置 Fluid 分布式缓存。'"
        @retry="fetchDatasets"
      >
        <el-table :data="pagedDatasets" row-key="id" border stripe style="width: 100%">
          <el-table-column prop="id" label="ID" width="70">
            <template #default="{ row }"><span class="mc-mono">{{ row.id }}</span></template>
          </el-table-column>
          <el-table-column prop="name" label="名称" min-width="140" />
          <el-table-column prop="description" label="描述" min-width="160" show-overflow-tooltip />
          <el-table-column label="源类型" width="110">
            <template #default="{ row }">
              <el-tag type="primary">{{ sourceTypeLabel(row.source_type) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="source_path" label="源路径" min-width="180" show-overflow-tooltip />
          <el-table-column prop="mount_path" label="挂载路径" min-width="160" show-overflow-tooltip />
          <el-table-column label="大小(GB)" width="100">
            <template #default="{ row }"><span class="mc-num">{{ row.size_gb ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column prop="access_mode" label="访问模式" width="110" />
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag :type="statusTagType(row.status)">{{ row.status || '-' }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="220" fixed="right">
            <template #default="{ row }">
              <Can :roles="['admin', 'manager']">
                <el-button link type="primary" size="small" @click="openEdit(row)">编辑</el-button>
              </Can>
              <el-button link type="primary" size="small" @click="openCacheDrawer(row)">管理缓存</el-button>
              <Can :roles="['admin', 'manager']">
                <el-button link type="danger" size="small" @click="handleDelete(row)">删除</el-button>
              </Can>
            </template>
          </el-table-column>
        </el-table>
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :total="filteredDatasets.length"
          :page-sizes="[10, 20, 50]"
          layout="total, sizes, prev, pager, next"
          style="margin-top: 12px; justify-content: flex-end"
        />
      </PageState>
    </el-card>

    <!-- 新建/编辑数据集 Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="editing ? '编辑数据集' : '新增数据集'"
      width="680px"
      destroy-on-close
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-position="top">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="名称" prop="name">
              <el-input v-model="form.name" placeholder="例如：imagenet-1k" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="源类型" prop="source_type">
              <el-select v-model="form.source_type" placeholder="请选择源类型" style="width: 100%">
                <el-option v-for="o in sourceTypeOptions" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="描述">
              <el-input v-model="form.description" type="textarea" :rows="2" placeholder="选填" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="源路径">
              <el-input v-model="form.source_path" placeholder="例如：ceph://data/imagenet" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="挂载路径">
              <el-input v-model="form.mount_path" placeholder="例如：/mnt/data/imagenet" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="大小 (GB)">
              <el-input-number v-model="form.size_gb" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="访问模式">
              <el-input v-model="form.access_mode" placeholder="例如：ReadWriteMany" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="状态">
              <el-select v-model="form.status" placeholder="请选择状态" style="width: 100%">
                <el-option label="活跃" value="active" />
                <el-option label="非活跃" value="inactive" />
                <el-option label="维护" value="maintenance" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="挂载选项">
              <el-input v-model="form.mount_options" placeholder="例如：vers=4.1,rsize=1048576" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="租户 ID">
              <el-input-number v-model="form.tenant_id" :min="1" style="width: 100%" />
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

    <!-- Fluid 缓存管理 Drawer -->
    <el-drawer
      v-model="cacheDrawerVisible"
      :title="`Fluid 缓存管理 - ${cacheTarget?.name ?? ''}`"
      size="920px"
    >
      <div class="drawer-toolbar">
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="openCreateCache">新增缓存</el-button>
        </Can>
      </div>

      <el-table
        v-loading="cachesLoading"
        :data="caches"
        row-key="id"
        size="small"
        border
        style="width: 100%"
      >
        <template #empty>
          <el-empty description="暂无缓存，为该数据集新增一个 Fluid 分布式缓存以加速读取。" :image-size="60" />
        </template>
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column label="运行时类型" width="110">
          <template #default="{ row }">
            <el-tag type="warning">{{ runtimeTypeLabel(row.runtime_type) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="缓存容量(GB)" width="110">
          <template #default="{ row }"><span class="mc-num">{{ row.cache_capacity_gb ?? '-' }}</span></template>
        </el-table-column>
        <el-table-column label="副本数" width="80">
          <template #default="{ row }"><span class="mc-num">{{ row.replicas ?? '-' }}</span></template>
        </el-table-column>
        <el-table-column label="介质类型" width="90">
          <template #default="{ row }">{{ mediumTypeLabel(row.medium_type) }}</template>
        </el-table-column>
        <el-table-column label="预取开关" width="90">
          <template #default="{ row }">
            <el-tag :type="row.prefetch_enabled ? 'success' : 'info'">{{ row.prefetch_enabled ? '开' : '关' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="prefetch_policy" label="预取策略" width="110" show-overflow-tooltip />
        <el-table-column label="压缩开关" width="90">
          <template #default="{ row }">
            <el-tag :type="row.compression_enabled ? 'success' : 'info'">{{ row.compression_enabled ? '开' : '关' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="元数据加速" width="100">
          <template #default="{ row }">
            <el-tag :type="row.metadata_acceleration_enabled ? 'success' : 'info'">
              {{ row.metadata_acceleration_enabled ? '开' : '关' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="状态" width="90">
          <template #default="{ row }">
            <el-tag :type="statusTagType(row.status)">{{ row.status || '-' }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column label="缓存命中率" width="130">
          <template #default="{ row }">
            <el-progress :percentage="row.cache_hit_rate ?? 0" :stroke-width="10" />
          </template>
        </el-table-column>
        <el-table-column label="操作" width="240" fixed="right">
          <template #default="{ row }">
            <Can :roles="['admin', 'manager']">
              <el-button link type="primary" size="small" @click="openEditCache(row)">编辑</el-button>
              <el-button
                v-if="row.status === 'active'"
                link
                type="warning"
                size="small"
                @click="handleDisableCache(row)"
              >禁用</el-button>
              <el-button v-else link type="success" size="small" @click="handleEnableCache(row)">启用</el-button>
              <el-button link type="primary" size="small" @click="handleTriggerPrefetch(row)">触发预取</el-button>
              <el-button link type="danger" size="small" @click="handleDeleteCache(row)">删除</el-button>
            </Can>
          </template>
        </el-table-column>
      </el-table>
    </el-drawer>

    <!-- 新建/编辑缓存 Dialog（内嵌于 Drawer） -->
    <el-dialog
      v-model="cacheDialogVisible"
      :title="editingCache ? '编辑 Fluid 缓存' : '新增 Fluid 缓存'"
      width="600px"
      append-to-body
      destroy-on-close
    >
      <el-form ref="cacheFormRef" :model="cacheForm" label-position="top">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="运行时类型">
              <el-select v-model="cacheForm.runtime_type" placeholder="请选择运行时" clearable style="width: 100%">
                <el-option v-for="o in runtimeTypeOptions" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="缓存容量 (GB)">
              <el-input-number v-model="cacheForm.cache_capacity_gb" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="副本数">
              <el-input-number v-model="cacheForm.replicas" :min="1" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="介质类型">
              <el-select v-model="cacheForm.medium_type" placeholder="请选择介质类型" clearable style="width: 100%">
                <el-option v-for="o in mediumTypeOptions" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="预取开关">
              <el-switch v-model="cacheForm.prefetch_enabled" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="压缩开关">
              <el-switch v-model="cacheForm.compression_enabled" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="元数据加速">
              <el-switch v-model="cacheForm.metadata_acceleration_enabled" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="预取策略">
              <el-input v-model="cacheForm.prefetch_policy" placeholder="例如：aggressive" />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" :loading="cacheSaving" @click="handleSubmitCache">
            {{ editingCache ? '保存' : '创建' }}
          </el-button>
        </Can>
        <el-button @click="cacheDialogVisible = false">取消</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, reactive } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import PageState from '@/components/PageState.vue'
import Can from '@/components/Can.vue'
import { datasetApi } from '@/api'
import type { Dataset, FluidCache } from '@/types'

const sourceTypeOptions = [
  { label: 'Ceph', value: 'ceph' },
  { label: 'NFS', value: 'nfs' },
  { label: 'S3', value: 's3' },
  { label: 'GlusterFS', value: 'glusterfs' },
  { label: 'Lustre', value: 'lustre' },
  { label: 'BeeGFS', value: 'beegfs' },
]

const runtimeTypeOptions = [
  { label: 'Alluxio', value: 'alluxio' },
  { label: 'JindoFS', value: 'jindofs' },
]

const mediumTypeOptions = [
  { label: '内存', value: 'memory' },
  { label: '磁盘', value: 'disk' },
  { label: '混合', value: 'mixed' },
]

function sourceTypeLabel(v?: string) {
  return sourceTypeOptions.find((o) => o.value === v)?.label ?? v ?? '-'
}
function runtimeTypeLabel(v?: string) {
  return runtimeTypeOptions.find((o) => o.value === v)?.label ?? v ?? '-'
}
function mediumTypeLabel(v?: string) {
  return mediumTypeOptions.find((o) => o.value === v)?.label ?? v ?? '-'
}

// ==================== 数据集 ====================
const datasets = ref<Dataset[]>([])
const listLoading = ref(false)
const listError = ref('')
const keyword = ref('')
const sourceTypeFilter = ref('')
const statusFilter = ref('')
const page = ref(1)
const pageSize = ref(10)

async function fetchDatasets() {
  listLoading.value = true
  listError.value = ''
  try {
    datasets.value = await datasetApi.list()
  } catch (e) {
    listError.value = e instanceof Error ? e.message : '加载数据集失败'
    datasets.value = []
  } finally {
    listLoading.value = false
  }
}
fetchDatasets()

const filteredDatasets = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  return datasets.value.filter((d) => {
    const matchKw =
      !kw ||
      d.name.toLowerCase().includes(kw) ||
      (d.description ?? '').toLowerCase().includes(kw) ||
      (d.source_path ?? '').toLowerCase().includes(kw)
    const matchSource = !sourceTypeFilter.value || d.source_type === sourceTypeFilter.value
    const matchStatus = !statusFilter.value || d.status === statusFilter.value
    return matchKw && matchSource && matchStatus
  })
})

const pagedDatasets = computed(() => {
  const start = (page.value - 1) * pageSize.value
  return filteredDatasets.value.slice(start, start + pageSize.value)
})

interface DatasetForm {
  name: string
  description: string
  source_type: Dataset['source_type'] | ''
  source_path: string
  mount_path: string
  size_gb: number | undefined
  mount_options: string
  access_mode: string
  status: string
  tenant_id: number | undefined
}

const dialogVisible = ref(false)
const editing = ref<Dataset | null>(null)
const saving = ref(false)
const formRef = ref<FormInstance>()

const emptyForm = (): DatasetForm => ({
  name: '',
  description: '',
  source_type: '',
  source_path: '',
  mount_path: '',
  size_gb: undefined,
  mount_options: '',
  access_mode: '',
  status: 'active',
  tenant_id: undefined,
})

const form = reactive<DatasetForm>(emptyForm())

const rules: FormRules<DatasetForm> = {
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  source_type: [{ required: true, message: '请选择源类型', trigger: 'change' }],
}

function openCreate() {
  editing.value = null
  Object.assign(form, emptyForm())
  dialogVisible.value = true
}

function openEdit(row: Dataset) {
  editing.value = row
  Object.assign(form, {
    name: row.name,
    description: row.description ?? '',
    source_type: row.source_type,
    source_path: row.source_path ?? '',
    mount_path: row.mount_path ?? '',
    size_gb: row.size_gb,
    mount_options: row.mount_options ?? '',
    access_mode: row.access_mode ?? '',
    status: row.status ?? 'active',
    tenant_id: row.tenant_id,
  })
  dialogVisible.value = true
}

async function handleSubmit() {
  if (!formRef.value) return
  await formRef.value.validate()
  saving.value = true
  try {
    const payload = { ...form } as Partial<Dataset>
    if (editing.value) {
      await datasetApi.update(editing.value.id, payload)
      ElMessage.success('数据集更新成功')
    } else {
      await datasetApi.create(payload)
      ElMessage.success('数据集创建成功')
    }
    dialogVisible.value = false
    await fetchDatasets()
  } catch {
    ElMessage.error('操作失败，请检查必填项后重试')
  } finally {
    saving.value = false
  }
}

async function handleDelete(row: Dataset) {
  try {
    await ElMessageBox.confirm(
      '数据集配置与关联缓存将一并移除，且不可恢复。',
      '删除该数据集？',
      { type: 'warning', confirmButtonText: '确认删除', cancelButtonText: '返回', confirmButtonClass: 'el-button--danger' },
    )
  } catch {
    return
  }
  try {
    await datasetApi.remove(row.id)
    ElMessage.success('数据集删除成功')
    await fetchDatasets()
  } catch {
    ElMessage.error('删除失败，请稍后重试')
  }
}

// ==================== Fluid 缓存 ====================
const cacheDrawerVisible = ref(false)
const cacheTarget = ref<Dataset | null>(null)
const caches = ref<FluidCache[]>([])
const cachesLoading = ref(false)

async function loadCaches(datasetId: number) {
  cachesLoading.value = true
  try {
    caches.value = await datasetApi.fluidCaches(datasetId)
  } catch {
    caches.value = []
    ElMessage.error('加载缓存列表失败')
  } finally {
    cachesLoading.value = false
  }
}

function openCacheDrawer(row: Dataset) {
  cacheTarget.value = row
  cacheDrawerVisible.value = true
  loadCaches(row.id)
}

interface CacheForm {
  runtime_type: FluidCache['runtime_type'] | undefined
  cache_capacity_gb: number | undefined
  replicas: number | undefined
  medium_type: FluidCache['medium_type'] | undefined
  prefetch_enabled: boolean
  prefetch_policy: string
  compression_enabled: boolean
  metadata_acceleration_enabled: boolean
}

const cacheDialogVisible = ref(false)
const editingCache = ref<FluidCache | null>(null)
const cacheSaving = ref(false)
const cacheFormRef = ref<FormInstance>()

const emptyCacheForm = (): CacheForm => ({
  runtime_type: undefined,
  cache_capacity_gb: undefined,
  replicas: 1,
  medium_type: undefined,
  prefetch_enabled: false,
  prefetch_policy: '',
  compression_enabled: false,
  metadata_acceleration_enabled: false,
})

const cacheForm = reactive<CacheForm>(emptyCacheForm())

function openCreateCache() {
  editingCache.value = null
  Object.assign(cacheForm, emptyCacheForm())
  cacheDialogVisible.value = true
}

function openEditCache(row: FluidCache) {
  editingCache.value = row
  Object.assign(cacheForm, {
    runtime_type: row.runtime_type,
    cache_capacity_gb: row.cache_capacity_gb,
    replicas: row.replicas,
    medium_type: row.medium_type,
    prefetch_enabled: !!row.prefetch_enabled,
    prefetch_policy: row.prefetch_policy ?? '',
    compression_enabled: !!row.compression_enabled,
    metadata_acceleration_enabled: !!row.metadata_acceleration_enabled,
  })
  cacheDialogVisible.value = true
}

async function handleSubmitCache() {
  if (!cacheTarget.value) return
  cacheSaving.value = true
  try {
    const payload = { ...cacheForm }
    if (editingCache.value) {
      await datasetApi.updateFluidCache(editingCache.value.id, payload)
      ElMessage.success('缓存更新成功')
    } else {
      await datasetApi.createFluidCache(cacheTarget.value.id, payload)
      ElMessage.success('缓存创建成功')
    }
    cacheDialogVisible.value = false
    await loadCaches(cacheTarget.value.id)
  } catch {
    ElMessage.error('操作失败，请检查必填项后重试')
  } finally {
    cacheSaving.value = false
  }
}

async function handleDeleteCache(row: FluidCache) {
  try {
    await ElMessageBox.confirm('删除该缓存？', '提示', {
      type: 'warning',
      confirmButtonText: '确认',
      cancelButtonText: '取消',
    })
  } catch {
    return
  }
  try {
    await datasetApi.deleteFluidCache(row.id)
    ElMessage.success('缓存删除成功')
    if (cacheTarget.value) await loadCaches(cacheTarget.value.id)
  } catch {
    ElMessage.error('删除失败，请稍后重试')
  }
}

async function handleEnableCache(row: FluidCache) {
  try {
    await datasetApi.enableFluidCache(row.id)
    ElMessage.success('缓存已启用')
    if (cacheTarget.value) await loadCaches(cacheTarget.value.id)
  } catch {
    ElMessage.error('操作失败，请稍后重试')
  }
}

async function handleDisableCache(row: FluidCache) {
  try {
    await datasetApi.disableFluidCache(row.id)
    ElMessage.success('缓存已禁用')
    if (cacheTarget.value) await loadCaches(cacheTarget.value.id)
  } catch {
    ElMessage.error('操作失败，请稍后重试')
  }
}

async function handleTriggerPrefetch(row: FluidCache) {
  try {
    await datasetApi.triggerPrefetch(row.id)
    ElMessage.success('预取已触发')
  } catch {
    ElMessage.error('触发失败，请稍后重试')
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
.drawer-toolbar {
  margin-bottom: 16px;
}
.mc-mono {
  font-family: var(--mc-mono-font, monospace);
}
.mc-num {
  font-variant-numeric: tabular-nums;
}
</style>
