<template>
  <div class="mc-page">
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">安全管理</h1>
        <p class="mc-page-desc">共 {{ policies.length }} 条安全策略 · 访问 / 数据 / 网络 / 应用 / 合规</p>
      </div>
    </div>

    <!-- 安全概览卡片 -->
    <el-row :gutter="16" class="stat-row">
      <el-col v-for="c in overviewCards" :key="c.key" :xs="12" :sm="8" :md="4">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-card-label">{{ c.label }}</div>
          <div class="stat-card-value">{{ c.value }}</div>
        </el-card>
      </el-col>
    </el-row>

    <el-card>
      <!-- 搜索 + 类型/状态筛选 -->
      <div class="mc-toolbar">
        <el-input
          v-model="keyword"
          placeholder="搜索策略名称"
          clearable
          style="width: 260px"
          aria-label="搜索安全策略"
        />
        <el-select v-model="typeFilter" placeholder="类型筛选" clearable style="width: 160px">
          <el-option
            v-for="o in typeOptions"
            :key="o.value"
            :label="o.label"
            :value="o.value"
          />
        </el-select>
        <el-select v-model="statusFilter" placeholder="状态筛选" clearable style="width: 160px">
          <el-option label="活跃" value="active" />
          <el-option label="非活跃" value="inactive" />
          <el-option label="错误" value="error" />
        </el-select>
      </div>

      <PageState
        :loading="listLoading"
        :error="listError"
        :data="filteredPolicies"
        empty-text="暂无安全策略，还没有配置任何安全策略。"
        @retry="fetchPolicies"
      >
        <el-table :data="pagedPolicies" row-key="id" border stripe style="width: 100%">
          <el-table-column prop="id" label="ID" width="70">
            <template #default="{ row }"><span class="mc-mono">{{ row.id }}</span></template>
          </el-table-column>
          <el-table-column prop="name" label="名称" min-width="140" />
          <el-table-column prop="description" label="描述" min-width="180" show-overflow-tooltip />
          <el-table-column label="类型" width="110">
            <template #default="{ row }">
              <el-tag type="primary">{{ typeLabel(row.type) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag :type="statusTagType(row.status)">{{ row.status || '-' }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="启用" width="90">
            <template #default="{ row }">
              <Can :roles="['admin']">
                <el-switch
                  :model-value="!!row.enabled"
                  @change="(v: boolean) => handleToggle(row, v)"
                />
              </Can>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="160" fixed="right">
            <template #default="{ row }">
              <el-button link type="primary" size="small" @click="openRules(row)">规则</el-button>
              <Can :roles="['admin']">
                <el-button link type="primary" size="small" @click="openEdit(row)">编辑</el-button>
              </Can>
            </template>
          </el-table-column>
        </el-table>
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :total="filteredPolicies.length"
          :page-sizes="[10, 20, 50]"
          layout="total, sizes, prev, pager, next"
          style="margin-top: 12px; justify-content: flex-end"
        />
      </PageState>
    </el-card>

    <!-- 编辑策略 Dialog -->
    <el-dialog
      v-model="editVisible"
      :title="`编辑安全策略 - ${editing?.name ?? ''}`"
      width="640px"
      destroy-on-close
    >
      <el-form ref="editFormRef" :model="editForm" :rules="editRules" label-position="top">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="名称" prop="name">
              <el-input v-model="editForm.name" placeholder="策略名称" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="类型" prop="type">
              <el-select v-model="editForm.type" placeholder="请选择类型" style="width: 100%">
                <el-option v-for="o in typeOptions" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
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
            <el-form-item label="规则 JSON">
              <el-input
                v-model="editForm.rules"
                type="textarea"
                :rows="6"
                placeholder='例如：{"effect":"deny","ip_cidrs":["0.0.0.0/0"]}'
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
        <Can :roles="['admin']">
          <el-button type="primary" :loading="saving" @click="handleSaveEdit">保存</el-button>
        </Can>
        <el-button @click="editVisible = false">取消</el-button>
      </template>
    </el-dialog>

    <!-- 策略规则 Drawer -->
    <el-drawer
      v-model="rulesVisible"
      :title="`策略规则 - ${rulesTarget?.name ?? ''}`"
      size="560px"
    >
      <template v-if="rulesTarget">
        <el-descriptions :column="1" border size="small">
          <el-descriptions-item label="名称">{{ rulesTarget.name }}</el-descriptions-item>
          <el-descriptions-item label="类型">{{ typeLabel(rulesTarget.type) }}</el-descriptions-item>
          <el-descriptions-item label="状态">
            <el-tag :type="statusTagType(rulesTarget.status)">{{ rulesTarget.status || '-' }}</el-tag>
          </el-descriptions-item>
          <el-descriptions-item label="描述">{{ rulesTarget.description ?? '-' }}</el-descriptions-item>
        </el-descriptions>

        <el-card shadow="never" class="detail-card">
          <template #header>规则详情</template>
          <el-descriptions v-if="ruleEntries.length" :column="1" size="small" border>
            <el-descriptions-item v-for="[k, v] in ruleEntries" :key="k" :label="k">
              <el-tag v-if="typeof v === 'boolean'" :type="v ? 'success' : 'info'">{{ v ? '开' : '关' }}</el-tag>
              <span v-else>{{ typeof v === 'object' ? JSON.stringify(v) : String(v) }}</span>
            </el-descriptions-item>
          </el-descriptions>
          <pre v-else-if="rulesRawText" class="rules-pre">{{ rulesRawText }}</pre>
          <el-empty v-else description="该策略尚未配置规则。" :image-size="60" />
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
import { securityApi } from '@/api'
import type { SecurityPolicy } from '@/types'

const typeOptions: { label: string; value: string }[] = [
  { label: '访问安全', value: 'access' },
  { label: '网络安全', value: 'network' },
  { label: '数据安全', value: 'data' },
  { label: '应用安全', value: 'system' },
]

function typeLabel(t?: string) {
  return typeOptions.find((o) => o.value === t)?.label ?? t ?? '-'
}

const policies = ref<SecurityPolicy[]>([])
const listLoading = ref(false)
const listError = ref('')
const keyword = ref('')
const typeFilter = ref('')
const statusFilter = ref('')
const page = ref(1)
const pageSize = ref(10)

async function fetchPolicies() {
  listLoading.value = true
  listError.value = ''
  try {
    policies.value = await securityApi.list()
  } catch (e) {
    listError.value = e instanceof Error ? e.message : '加载安全策略失败'
    policies.value = []
  } finally {
    listLoading.value = false
  }
}
fetchPolicies()

// 安全概览卡片
const overviewCards = computed(() => {
  const all = policies.value
  const count = (pred: (p: SecurityPolicy) => boolean) => all.filter(pred).length
  return [
    { key: 'total', label: '策略总数', value: all.length },
    { key: 'enabled', label: '已启用', value: count((p) => !!p.enabled) },
    { key: 'access', label: '访问控制', value: count((p) => p.type === 'access') },
    { key: 'network', label: '网络安全', value: count((p) => p.type === 'network') },
    { key: 'data', label: '数据安全', value: count((p) => p.type === 'data') },
    { key: 'system', label: '系统安全', value: count((p) => p.type === 'system') },
  ]
})

const filteredPolicies = computed(() => {
  const kw = keyword.value.trim().toLowerCase()
  return policies.value.filter((p) => {
    const matchKw = !kw || p.name.toLowerCase().includes(kw) || (p.description ?? '').toLowerCase().includes(kw)
    const matchType = !typeFilter.value || p.type === typeFilter.value
    const matchStatus = !statusFilter.value || p.status === statusFilter.value
    return matchKw && matchType && matchStatus
  })
})

const pagedPolicies = computed(() => {
  const start = (page.value - 1) * pageSize.value
  return filteredPolicies.value.slice(start, start + pageSize.value)
})

// 启用 / 禁用切换
async function handleToggle(row: SecurityPolicy, enabled: boolean) {
  try {
    await securityApi.update(row.id, { enabled })
    row.enabled = enabled
    ElMessage.success(`安全策略${enabled ? '启用' : '禁用'}成功`)
  } catch {
    ElMessage.error('操作失败，请稍后重试')
  }
}

// 编辑 Dialog
interface PolicyForm {
  name: string
  description: string
  type: string
  status: string
  enabled: boolean
  rules: string
  details: string
}

const editVisible = ref(false)
const editing = ref<SecurityPolicy | null>(null)
const saving = ref(false)
const editFormRef = ref<FormInstance>()

const emptyEditForm = (): PolicyForm => ({
  name: '',
  description: '',
  type: 'access',
  status: 'active',
  enabled: true,
  rules: '',
  details: '',
})

const editForm = reactive<PolicyForm>(emptyEditForm())

const editRules: FormRules<PolicyForm> = {
  name: [{ required: true, message: '请输入名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择类型', trigger: 'change' }],
}

function openEdit(row: SecurityPolicy) {
  editing.value = row
  Object.assign(editForm, {
    name: row.name,
    description: row.description ?? '',
    type: row.type ?? 'access',
    status: row.status ?? 'active',
    enabled: !!row.enabled,
    rules: row.rules ?? '',
    details: row.details ?? '',
  })
  editVisible.value = true
}

async function handleSaveEdit() {
  if (!editFormRef.value || !editing.value) return
  await editFormRef.value.validate()
  saving.value = true
  try {
    await securityApi.update(editing.value.id, { ...editForm })
    ElMessage.success('安全策略更新成功')
    editVisible.value = false
    await fetchPolicies()
  } catch {
    ElMessage.error('更新失败，请稍后重试')
  } finally {
    saving.value = false
  }
}

// 规则 Drawer
const rulesVisible = ref(false)
const rulesTarget = ref<SecurityPolicy | null>(null)

function openRules(row: SecurityPolicy) {
  rulesTarget.value = row
  rulesVisible.value = true
}

const ruleConfig = computed<Record<string, unknown>>(() => {
  const raw = rulesTarget.value?.rules
  if (!raw) return {}
  try {
    const v = JSON.parse(raw)
    return v && typeof v === 'object' ? (v as Record<string, unknown>) : {}
  } catch {
    return {}
  }
})

const ruleEntries = computed<[string, unknown][]>(() => Object.entries(ruleConfig.value))

const rulesRawText = computed(() => {
  const raw = rulesTarget.value?.rules
  if (!raw) return ''
  try {
    return JSON.stringify(JSON.parse(raw), null, 2)
  } catch {
    return raw
  }
})

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
.stat-card-label {
  font-size: 13px;
  color: var(--mc-text-3, #909399);
}
.stat-card-value {
  font-size: 26px;
  font-weight: 600;
  margin-top: 6px;
}
.mc-toolbar {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}
.detail-card {
  margin-top: 16px;
}
.rules-pre {
  background: var(--mc-bg-2, #f5f7fa);
  padding: 12px;
  border-radius: 4px;
  font-size: 12px;
  overflow: auto;
  max-height: 480px;
  margin: 0;
}
.mc-mono {
  font-family: var(--mc-mono-font, monospace);
}
</style>
