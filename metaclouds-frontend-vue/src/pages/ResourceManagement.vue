<template>
  <div class="mc-page">
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">基础资源管理</h1>
        <p class="mc-page-desc">AI 计算资源池 · AI 存储资源 · AI 网络资源 三大基础层总览</p>
      </div>
    </div>

    <!-- GPU 资源概览卡片 -->
    <div class="gpu-overview">
      <el-card shadow="never" v-for="s in vendorStats" :key="s.key">
        <div class="gpu-stat-label">{{ s.label }}</div>
        <div class="gpu-stat-value mc-num">{{ s.value }}<span class="gpu-stat-unit">{{ s.unit }}</span></div>
      </el-card>
    </div>

    <el-card shadow="never" class="mc-mt">
      <!-- 搜索 + 类型筛选工具栏 -->
      <div class="toolbar">
        <el-input
          v-model="keyword"
          placeholder="搜索资源名称 / 型号"
          allow-clear
          class="toolbar-search"
        >
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <el-select v-model="typeFilter" placeholder="资源类型" style="width: 160px">
          <el-option label="全部类型" value="" />
          <el-option label="GPU" value="gpu" />
          <el-option label="CPU" value="cpu" />
          <el-option label="内存" value="memory" />
          <el-option label="存储" value="storage" />
          <el-option label="网络" value="network" />
        </el-select>
      </div>

      <PageState
        :loading="loading"
        :error="error"
        :data="resourcesData"
        empty-text="暂无资源，新增节点后将在此分类汇总"
        retry-text="重新加载"
        @retry="refetch"
      >
        <el-table :data="pagedResources" row-key="id" stripe style="width: 100%">
          <el-table-column prop="name" label="名称" min-width="160" show-overflow-tooltip />
          <el-table-column label="类型" width="100">
            <template #default="{ row }">
              <el-tag :type="row.type === 'cpu' ? 'info' : row.type === 'gpu' ? 'primary' : 'success'" size="small">
                {{ typeLabel(row.type) }}
              </el-tag>
            </template>
          </el-table-column>
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag :type="statusTagType(row.status)" size="small">{{ statusText(row.status) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="总量" width="100" align="right">
            <template #default="{ row }"><span class="mc-num">{{ row.total ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="已用" width="100" align="right">
            <template #default="{ row }"><span class="mc-num">{{ row.used ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="可用" width="100" align="right">
            <template #default="{ row }"><span class="mc-num">{{ row.available ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="利用率" width="160">
            <template #default="{ row }">
              <el-progress :percentage="Math.round(row.utilization ?? 0)" :stroke-width="10" :show-text="true" />
            </template>
          </el-table-column>
          <el-table-column label="厂商" width="110">
            <template #default="{ row }">
              <el-tag v-if="row.vendor" :type="vendorTagType(row.vendor)" size="small">
                {{ vendorLabel(row.vendor) }}
              </el-tag>
              <el-tag v-else size="small">通用</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="gpu_model" label="型号" width="130">
            <template #default="{ row }">{{ row.gpu_model || '-' }}</template>
          </el-table-column>
          <el-table-column label="操作" width="100" fixed="right">
            <template #default="{ row }">
              <Can :roles="['admin', 'manager']">
                <el-button link type="primary" size="small" @click="openEdit(row)">编辑</el-button>
              </Can>
            </template>
          </el-table-column>
        </el-table>

        <div class="pager">
          <el-pagination
            v-model:current-page="page"
            :page-size="pageSize"
            :total="filteredResources.length"
            layout="total, prev, pager, next"
          />
        </div>
      </PageState>
    </el-card>

    <!-- 编辑资源 -->
    <el-dialog v-model="editVisible" :title="`编辑资源 - ${editing?.name ?? ''}`" width="560px" destroy-on-close>
      <el-form ref="formRef" :model="form" :rules="rules" label-position="vertical">
        <el-form-item label="名称" prop="name">
          <el-input v-model="form.name" />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="form.status" style="width: 100%">
            <el-option label="可用" value="available" />
            <el-option label="繁忙" value="busy" />
            <el-option label="维护" value="maintenance" />
            <el-option label="故障" value="fault" />
          </el-select>
        </el-form-item>
        <el-form-item label="总量">
          <el-input-number v-model="form.total" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="已用">
          <el-input-number v-model="form.used" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="可用">
          <el-input-number v-model="form.available" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="利用率(%)">
          <el-input-number v-model="form.utilization" :min="0" :max="100" style="width: 100%" />
        </el-form-item>
        <el-form-item label="详情">
          <el-input v-model="form.details" type="textarea" :rows="2" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="editVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSave">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { Search } from '@element-plus/icons-vue'
import PageState from '@/components/PageState.vue'
import Can from '@/components/Can.vue'
import { resourceApi } from '@/api'
import { useFetch } from '@/utils/useFetch'
import type { Resource, GPUVendor } from '@/types'

const { data, loading, error, refetch } = useFetch<Resource[]>(() => resourceApi.list())
const resourcesData = computed(() => data.value ?? [])

// ---------- 搜索 / 类型筛选 ----------
const keyword = ref('')
const typeFilter = ref('')
const filteredResources = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  return resourcesData.value.filter((r) => {
    const matchKw =
      !kw ||
      r.name.toLowerCase().includes(kw) ||
      (r.gpu_model ?? '').toLowerCase().includes(kw)
    const matchType = !typeFilter.value || r.type === typeFilter.value
    return matchKw && matchType
  })
})

// ---------- 分页 ----------
const page = ref(1)
const pageSize = 10
const pagedResources = computed(() =>
  filteredResources.value.slice((page.value - 1) * pageSize, page.value * pageSize),
)

// ---------- GPU 概览 ----------
const VENDOR_LABEL: Record<GPUVendor, string> = {
  nvidia: 'NVIDIA',
  enflame: '燧原',
  moore_threads: '摩尔线程',
  domestic_x: '国产 X',
}
const vendorStats = computed(() => {
  const gpuList = resourcesData.value.filter((r) => r.type === 'gpu')
  const byVendor = (v: GPUVendor) => gpuList.filter((r) => r.vendor === v).length
  const cpuCores = resourcesData.value
    .filter((r) => r.type === 'cpu')
    .reduce((s, r) => s + (Number(r.total) || 0), 0)
  return [
    { key: 'nvidia', label: 'NVIDIA', value: byVendor('nvidia'), unit: '台' },
    { key: 'enflame', label: '燧原 Enflame', value: byVendor('enflame'), unit: '台' },
    { key: 'moore_threads', label: '摩尔线程', value: byVendor('moore_threads'), unit: '台' },
    { key: 'domestic_x', label: '国产 X', value: byVendor('domestic_x'), unit: '台' },
    { key: 'cpu', label: 'CPU 资源', value: cpuCores, unit: '核' },
  ]
})

// ---------- 编辑 ----------
const editVisible = ref(false)
const editing = ref<Resource | null>(null)
const submitting = ref(false)
const formRef = ref<FormInstance>()

const form = reactive({
  name: '',
  status: 'available',
  total: 0,
  used: 0,
  available: 0,
  utilization: 0,
  details: '',
})
const rules: FormRules = {
  name: [{ required: true, message: '请输入资源名称', trigger: 'blur' }],
}

function openEdit(row: Resource) {
  editing.value = row
  Object.assign(form, {
    name: row.name,
    status: row.status,
    total: row.total ?? 0,
    used: row.used ?? 0,
    available: row.available ?? 0,
    utilization: Math.round(row.utilization ?? 0),
    details: row.details ?? '',
  })
  editVisible.value = true
}

async function handleSave() {
  if (!formRef.value || !editing.value) return
  try {
    await formRef.value.validate()
  } catch {
    return
  }
  submitting.value = true
  try {
    await resourceApi.update(editing.value.id, { ...form })
    ElMessage.success('资源更新成功')
    editVisible.value = false
    refetch()
  } catch {
    ElMessage.error('资源更新失败，请稍后重试')
  } finally {
    submitting.value = false
  }
}

// ---------- 展示辅助 ----------
function typeLabel(t: string) {
  return { gpu: 'GPU', cpu: 'CPU', memory: '内存', storage: '存储', network: '网络' }[t] ?? t
}
function vendorLabel(v: GPUVendor) {
  return VENDOR_LABEL[v] ?? v
}
function vendorTagType(v: GPUVendor): 'success' | 'primary' | 'warning' | 'danger' {
  const map: Record<GPUVendor, 'success' | 'primary' | 'warning' | 'danger'> = { nvidia: 'success', enflame: 'primary', moore_threads: 'warning', domestic_x: 'danger' }
  return map[v] ?? 'primary'
}
function statusText(status?: string) {
  const map: Record<string, string> = {
    available: '可用',
    busy: '繁忙',
    maintenance: '维护',
    fault: '故障',
    active: '活跃',
    inactive: '非活跃',
    running: '运行中',
  }
  return map[status ?? ''] ?? status ?? '-'
}
function statusTagType(status?: string): 'success' | 'warning' | 'danger' | 'info' | 'primary' {
  if (status === 'available' || status === 'active' || status === 'running') return 'success'
  if (status === 'maintenance') return 'warning'
  if (status === 'fault' || status === 'failed' || status === 'error') return 'danger'
  if (status === 'busy') return 'primary'
  return 'info'
}
</script>

<style scoped>
.gpu-overview {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(160px, 1fr));
  gap: var(--mc-gap);
}
.gpu-stat-label { font-size: 13px; color: var(--mc-text-3); margin-bottom: 6px; }
.gpu-stat-value { font-size: 28px; font-weight: 680; color: var(--mc-text-1); }
.gpu-stat-unit { font-size: 14px; font-weight: 400; color: var(--mc-text-3); margin-left: 4px; }

.toolbar { display: flex; gap: 12px; margin-bottom: 16px; flex-wrap: wrap; }
.toolbar-search { flex: 1; min-width: 240px; max-width: 380px; }
.pager { display: flex; justify-content: flex-end; margin-top: 16px; }
</style>
