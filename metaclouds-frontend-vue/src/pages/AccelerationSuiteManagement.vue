<template>
  <div class="mc-page">
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">加速套件管理</h1>
        <p class="mc-page-desc">共 {{ suites.length }} 个加速套件 · 数据加速 / 分布式训练 / 推理加速 / 通信优化</p>
      </div>
    </div>

    <!-- 分类汇总卡片（点击切换分类筛选） -->
    <el-row :gutter="16" class="stat-row">
      <el-col
        v-for="c in categoryCards"
        :key="c.key"
        :xs="12"
        :sm="12"
        :md="6"
      >
        <el-card
          shadow="hover"
          :class="['stat-card', { 'is-active': categoryFilter === c.key }]"
          @click="toggleCategory(c.key)"
        >
          <div class="stat-card-label">{{ c.label }}</div>
          <div class="stat-card-value">
            {{ categoryStats[c.key]?.total ?? 0 }}
            <span class="stat-card-suffix">启用 {{ categoryStats[c.key]?.enabled ?? 0 }}</span>
          </div>
          <div class="stat-card-hint">{{ c.hint }}</div>
        </el-card>
      </el-col>
    </el-row>

    <el-card>
      <!-- 搜索 + 类型/分类筛选 -->
      <div class="mc-toolbar">
        <el-input
          v-model="keyword"
          placeholder="搜索加速套件名称"
          clearable
          style="width: 260px"
          aria-label="搜索加速套件"
        />
        <el-select v-model="typeFilter" placeholder="类型筛选" clearable style="width: 160px">
          <el-option label="数据加速" value="data" />
          <el-option label="训练加速" value="training" />
          <el-option label="推理加速" value="inference" />
        </el-select>
        <el-select v-model="categoryFilter" placeholder="分类筛选" clearable style="width: 180px">
          <el-option
            v-for="o in categoryOptions"
            :key="o.value"
            :label="o.label"
            :value="o.value"
          />
        </el-select>
      </div>

      <PageState
        :loading="listLoading"
        :error="listError"
        :data="filteredSuites"
        :empty-text="categoryFilter ? '该分类下暂无加速套件。' : '暂无加速套件，还没有登记任何加速套件。'"
        @retry="fetchSuites"
      >
        <el-table :data="pagedSuites" row-key="id" border stripe style="width: 100%">
          <el-table-column prop="id" label="ID" width="70">
            <template #default="{ row }"><span class="mc-mono">{{ row.id }}</span></template>
          </el-table-column>
          <el-table-column prop="name" label="名称" min-width="140" />
          <el-table-column prop="description" label="描述" min-width="160" show-overflow-tooltip />
          <el-table-column label="类型" width="100">
            <template #default="{ row }">
              <el-tag>{{ row.type || '-' }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="version" label="版本" width="90" />
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag :type="statusTagType(row.status)">{{ row.status || '-' }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="启用" width="90">
            <template #default="{ row }">
              <Can :roles="['admin', 'manager']">
                <el-switch
                  :model-value="!!row.enabled"
                  @change="(v: boolean) => handleToggle(row, v)"
                />
              </Can>
            </template>
          </el-table-column>
          <el-table-column label="分类" width="120">
            <template #default="{ row }">
              <el-tag v-if="categoryLabel(row.category)" type="primary">{{ categoryLabel(row.category) }}</el-tag>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column label="厂商" width="100">
            <template #default="{ row }">
              <el-tag v-if="row.vendor" type="success">{{ row.vendor }}</el-tag>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="150" fixed="right">
            <template #default="{ row }">
              <el-button link type="primary" size="small" @click="openDetail(row)">详情</el-button>
              <Can :roles="['admin', 'manager']">
                <el-button link type="primary" size="small" @click="openEdit(row)">编辑</el-button>
              </Can>
            </template>
          </el-table-column>
        </el-table>
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :total="filteredSuites.length"
          :page-sizes="[10, 20, 50]"
          layout="total, sizes, prev, pager, next"
          style="margin-top: 12px; justify-content: flex-end"
        />
      </PageState>
    </el-card>

    <!-- 编辑加速套件 Dialog -->
    <el-dialog
      v-model="editVisible"
      :title="`编辑加速套件 - ${editing?.name ?? ''}`"
      width="640px"
      destroy-on-close
    >
      <el-form ref="editFormRef" :model="editForm" :rules="editRules" label-position="top">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="名称" prop="name">
              <el-input v-model="editForm.name" placeholder="套件名称" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="类型" prop="type">
              <el-select v-model="editForm.type" placeholder="请选择类型" style="width: 100%">
                <el-option label="数据加速" value="data" />
                <el-option label="训练加速" value="training" />
                <el-option label="推理加速" value="inference" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="版本">
              <el-input v-model="editForm.version" placeholder="例如：v1.2.0" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="状态">
              <el-select v-model="editForm.status" placeholder="请选择状态" style="width: 100%">
                <el-option label="活跃" value="active" />
                <el-option label="非活跃" value="inactive" />
                <el-option label="错误" value="error" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="分类">
              <el-select v-model="editForm.category" placeholder="请选择分类" clearable style="width: 100%">
                <el-option
                  v-for="o in categoryOptions"
                  :key="o.value"
                  :label="o.label"
                  :value="o.value"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="厂商">
              <el-select v-model="editForm.vendor" placeholder="请选择厂商" clearable style="width: 100%">
                <el-option v-for="o in vendorOptions" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="启用开关">
              <el-switch v-model="editForm.enabled" />
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="描述">
              <el-input v-model="editForm.description" type="textarea" :rows="2" placeholder="选填" />
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="Config JSON">
              <el-input
                v-model="editForm.config_json"
                type="textarea"
                :rows="6"
                placeholder='例如：{"runtime":"alluxio","cache_capacity_gb":100}'
              />
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="详情">
              <el-input v-model="editForm.details" type="textarea" :rows="3" placeholder="选填，补充说明" />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" :loading="saving" @click="handleSaveEdit">保存</el-button>
        </Can>
        <el-button @click="editVisible = false">取消</el-button>
      </template>
    </el-dialog>

    <!-- 加速套件详情 Drawer -->
    <el-drawer
      v-model="detailVisible"
      :title="`加速套件详情 - ${detail?.name ?? ''}`"
      size="560px"
    >
      <template v-if="detail">
        <el-descriptions :column="1" border size="small">
          <el-descriptions-item label="名称">{{ detail.name }}</el-descriptions-item>
          <el-descriptions-item label="类型">{{ detail.type || '-' }}</el-descriptions-item>
          <el-descriptions-item label="类别">{{ categoryLabel(detail.category) ?? detail.category ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="厂商">{{ detail.vendor ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="版本">{{ detail.version ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag :type="statusTagType(detail.status)">{{ detail.status || '-' }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="描述">{{ detail.description ?? '-' }}</el-descriptions-item>
        </el-descriptions>

        <el-card shadow="never" class="detail-card">
          <template #header>细分能力配置</template>
          <el-descriptions v-if="detailEntries.length" :column="1" size="small" border>
            <el-descriptions-item v-for="[k, v] in detailEntries" :key="k" :label="CONFIG_LABELS[k] ?? k">
              <el-tag v-if="typeof v === 'boolean'" :type="v ? 'success' : 'info'">{{ v ? '开' : '关' }}</el-tag>
              <span v-else>{{ String(v) }}</span>
            </el-descriptions-item>
          </el-descriptions>
          <el-empty v-else description="该套件尚未填写 config_json 能力参数。" :image-size="60" />
        </el-card>
      </template>
    </el-drawer>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, reactive } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import PageState from '@/components/PageState.vue'
import Can from '@/components/Can.vue'
import { accelerationApi } from '@/api'
import type { AccelerationSuite, GPUVendor } from '@/types'

const categoryOptions: { label: string; value: string }[] = [
  { label: '数据加速', value: 'fluid_cache' },
  { label: '分布式训练', value: 'distributed_training' },
  { label: '推理加速', value: 'inference' },
  { label: '通信优化', value: 'communication' },
]

const vendorOptions: { label: string; value: GPUVendor }[] = [
  { label: 'NVIDIA', value: 'nvidia' },
  { label: '燧原', value: 'enflame' },
  { label: '摩尔线程', value: 'moore_threads' },
  { label: '国产 X', value: 'domestic_x' },
]

const categoryCards = [
  { key: 'fluid_cache', label: '数据加速', hint: 'Fluid 缓存 / 预取 / 压缩' },
  { key: 'distributed_training', label: '分布式训练', hint: 'NCCL / DeepSpeed / 并行' },
  { key: 'inference', label: '推理加速', hint: 'TensorRT / 量化 / 批处理' },
  { key: 'communication', label: '通信优化', hint: '梯度通信 / 通信策略' },
]

const CONFIG_LABELS: Record<string, string> = {
  runtime: '运行时',
  cache_capacity_gb: '缓存容量(GB)',
  replicas: '副本数',
  medium_type: '缓存介质',
  prefetch_enabled: '数据预取',
  prefetch_policy: '预取策略',
  compression_enabled: '数据压缩',
  metadata_acceleration_enabled: '元数据加速',
  cache_hit_rate: '缓存命中率',
  parallel_strategy: '并行策略',
  world_size: '世界大小',
  tensor_parallel_size: '张量并行度',
  pipeline_stages: '流水线段数',
  deepspeed_enabled: 'DeepSpeed',
  megatron_enabled: 'Megatron-LM',
  communication_backend: '通信后端',
  nccl_config: 'NCCL 配置',
  gradient_compression: '梯度通信压缩',
  backend: '推理后端引擎',
  precision: '计算精度',
  quantization_enabled: '模型量化',
  batch_size: '批大小',
  dynamic_batching: '动态批处理',
  max_batch_size: '最大批大小',
  max_latency_ms: '最大时延(ms)',
  operator_fusion: '算子融合',
}

const suites = ref<AccelerationSuite[]>([])
const listLoading = ref(false)
const listError = ref('')
const keyword = ref('')
const typeFilter = ref('')
const categoryFilter = ref('')
const page = ref(1)
const pageSize = ref(10)

async function fetchSuites() {
  listLoading.value = true
  listError.value = ''
  try {
    suites.value = await accelerationApi.list()
  } catch (e) {
    listError.value = e instanceof Error ? e.message : '加载加速套件失败'
    suites.value = []
  } finally {
    listLoading.value = false
  }
}
fetchSuites()

const categoryStats = computed(() => {
  const stats: Record<string, { total: number; enabled: number }> = {}
  suites.value.forEach((s) => {
    const key = s.category ?? 'other'
    if (!stats[key]) stats[key] = { total: 0, enabled: 0 }
    stats[key].total += 1
    if (s.enabled) stats[key].enabled += 1
  })
  return stats
})

const filteredSuites = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  return suites.value.filter((s) => {
    const matchKw = !kw || s.name.toLowerCase().includes(kw) || (s.description ?? '').toLowerCase().includes(kw)
    const matchType = !typeFilter.value || s.type === typeFilter.value
    const matchCat = !categoryFilter.value || s.category === categoryFilter.value
    return matchKw && matchType && matchCat
  })
})

const pagedSuites = computed(() => {
  const start = (page.value - 1) * pageSize.value
  return filteredSuites.value.slice(start, start + pageSize.value)
})

function toggleCategory(key: string) {
  categoryFilter.value = categoryFilter.value === key ? '' : key
}

function categoryLabel(cat?: string) {
  return categoryOptions.find((o) => o.value === cat)?.label
}

// 启用 / 禁用切换
async function handleToggle(row: AccelerationSuite, enabled: boolean) {
  try {
    await accelerationApi.update(row.id, { enabled })
    row.enabled = enabled
    ElMessage.success(`加速套件${enabled ? '启用' : '禁用'}成功`)
  } catch {
    ElMessage.error('操作失败，请稍后重试')
  }
}

// 编辑 Dialog
interface SuiteForm {
  name: string
  description: string
  type: string
  version: string
  status: string
  enabled: boolean
  config_json: string
  category: string | undefined
  vendor: GPUVendor | undefined
  details: string
}

const editVisible = ref(false)
const editing = ref<AccelerationSuite | null>(null)
const saving = ref(false)
const editFormRef = ref<FormInstance>()

const emptyEditForm = (): SuiteForm => ({
  name: '',
  description: '',
  type: '',
  version: '',
  status: 'active',
  enabled: true,
  config_json: '',
  category: undefined,
  vendor: undefined,
  details: '',
})

const editForm = reactive<SuiteForm>(emptyEditForm())

const editRules: FormRules<SuiteForm> = {
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择类型', trigger: 'change' }],
}

function openEdit(row: AccelerationSuite) {
  editing.value = row
  Object.assign(editForm, {
    name: row.name,
    description: row.description ?? '',
    type: row.type ?? '',
    version: row.version ?? '',
    status: row.status ?? 'active',
    enabled: !!row.enabled,
    config_json: row.config_json ?? '',
    category: row.category,
    vendor: row.vendor,
    details: row.details ?? '',
  })
  editVisible.value = true
}

async function handleSaveEdit() {
  if (!editFormRef.value || !editing.value) return
  await editFormRef.value.validate()
  saving.value = true
  try {
    await accelerationApi.update(editing.value.id, { ...editForm } as Partial<AccelerationSuite>)
    ElMessage.success('加速套件更新成功')
    editVisible.value = false
    await fetchSuites()
  } catch {
    ElMessage.error('更新失败，请稍后重试')
  } finally {
    saving.value = false
  }
}

// 详情 Drawer
const detailVisible = ref(false)
const detail = ref<AccelerationSuite | null>(null)

function openDetail(row: AccelerationSuite) {
  detail.value = row
  detailVisible.value = true
}

const detailConfig = computed<Record<string, unknown>>(() => {
  if (!detail.value?.config_json) return {}
  try {
    const v = JSON.parse(detail.value.config_json)
    return v && typeof v === 'object' ? (v as Record<string, unknown>) : {}
  } catch {
    return {}
  }
})

const detailEntries = computed<[string, unknown][]>(() => Object.entries(detailConfig.value))

function statusTagType(status?: string) {
  switch (status) {
    case 'active':
    case 'running':
      return 'success'
    case 'inactive':
    case 'paused':
      return 'info'
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
.stat-row {
  margin-bottom: 16px;
}
.stat-card {
  cursor: pointer;
  transition: border-color 0.2s;
}
.stat-card.is-active {
  border-color: var(--el-color-primary);
}
.stat-card-label {
  font-size: 13px;
  color: var(--mc-text-3, #909399);
}
.stat-card-value {
  font-size: 26px;
  font-weight: 600;
  margin: 6px 0 2px;
}
.stat-card-suffix {
  font-size: 12px;
  font-weight: 400;
  color: var(--mc-text-3, #909399);
  margin-left: 8px;
}
.stat-card-hint {
  font-size: 12px;
  color: var(--mc-text-3, #909399);
}
.mc-toolbar {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
.detail-card {
  margin-top: 16px;
}
.mc-mono {
  font-family: var(--mc-mono-font, monospace);
}
</style>
