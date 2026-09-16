<template>
  <div class="mc-page">
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">集群管理</h1>
        <p class="mc-page-desc">
          共 {{ clustersData.length }} 个集群 · 多集群统一调度 / 高可用 / SSH·VNC·WEB 多方式访问
        </p>
      </div>
      <div class="mc-page-head-extra">
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" :icon="Plus" @click="openCreate">创建集群</el-button>
        </Can>
      </div>
    </div>

    <el-card shadow="never">
      <!-- 搜索 + 状态筛选工具栏 -->
      <div class="toolbar">
        <el-input
          v-model="searchText"
          placeholder="搜索集群名称 / 描述 / 位置"
          allow-clear
          class="toolbar-search"
        >
          <template #prefix><el-icon><Search /></el-icon></template>
        </el-input>
        <el-select v-model="statusFilter" placeholder="全部状态" style="width: 140px">
          <el-option label="全部状态" value="" />
          <el-option label="活跃" value="active" />
          <el-option label="非活跃" value="inactive" />
          <el-option label="维护" value="maintenance" />
        </el-select>
      </div>

      <PageState
        :loading="loading"
        :error="error"
        :data="clustersData"
        empty-text="还没有集群，点击右上角创建第一个集群"
        retry-text="重新加载"
        @retry="refetch"
      >
        <el-table :data="pagedClusters" row-key="id" stripe style="width: 100%">
          <el-table-column prop="id" label="ID" width="80">
            <template #default="{ row }"><span class="mc-mono">{{ row.id }}</span></template>
          </el-table-column>
          <el-table-column label="名称" min-width="150">
            <template #default="{ row }">
              <el-link type="primary" :underline="false" @click="openDetail(row)">{{ row.name }}</el-link>
            </template>
          </el-table-column>
          <el-table-column prop="description" label="描述" min-width="180" show-overflow-tooltip />
          <el-table-column label="状态" width="110">
            <template #default="{ row }">
              <el-tag :type="statusTagType(row.status)" size="small">{{ statusText(row.status) }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="nodes" label="节点数" width="90" align="right">
            <template #default="{ row }"><span class="mc-num">{{ row.nodes ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column prop="gpus" label="GPU数" width="90" align="right">
            <template #default="{ row }"><span class="mc-num">{{ row.gpus ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="GPU厂商" width="150">
            <template #default="{ row }">
              <template v-if="row.gpu_vendors">
                <el-tag
                  v-for="v in splitTags(row.gpu_vendors)"
                  :key="v"
                  type="primary"
                  size="small"
                  style="margin-right: 4px"
                >{{ v }}</el-tag>
              </template>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column label="调度器类型" width="150">
            <template #default="{ row }">
              <template v-if="row.scheduler_types">
                <el-tag
                  v-for="s in splitTags(row.scheduler_types)"
                  :key="s"
                  type="warning"
                  size="small"
                  style="margin-right: 4px"
                >{{ s }}</el-tag>
              </template>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column label="多集群" width="90">
            <template #default="{ row }">
              <el-tag v-if="row.multi_cluster_enabled" type="success" size="small">联邦</el-tag>
              <el-tag v-else size="small">未启用</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="高可用" width="100">
            <template #default="{ row }">
              <el-tag v-if="row.ha_enabled || row.scheduler_ha_enabled" type="success" size="small">HA</el-tag>
              <el-tag v-else size="small">未启用</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="访问方式" width="150">
            <template #default="{ row }">
              <template v-if="row.access_methods">
                <el-tag
                  v-for="m in splitTags(row.access_methods)"
                  :key="m"
                  type="info"
                  size="small"
                  style="margin-right: 4px"
                >{{ m.toUpperCase() }}</el-tag>
              </template>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column label="操作" width="220" fixed="right">
            <template #default="{ row }">
              <Can :roles="['admin', 'manager']">
                <el-button link type="primary" size="small" @click="openEdit(row)">编辑</el-button>
                <el-button link type="primary" size="small" @click="openScale(row)">扩容</el-button>
                <el-popconfirm title="重建该集群？运行中的作业会被迁移或排队。" @confirm="handleRebuild(row)">
                  <template #reference>
                    <el-button link type="primary" size="small">重建</el-button>
                  </template>
                </el-popconfirm>
                <el-popconfirm
                  title="删除该集群？节点与配额将一并移除且不可恢复。"
                  confirm-button-text="确认删除"
                  confirm-button-type="danger"
                  @confirm="handleDelete(row)"
                >
                  <template #reference>
                    <el-button link type="danger" size="small">删除</el-button>
                  </template>
                </el-popconfirm>
              </Can>
            </template>
          </el-table-column>
        </el-table>

        <div class="pager">
          <el-pagination
            v-model:current-page="page"
            :page-size="pageSize"
            :total="filteredClusters.length"
            layout="total, prev, pager, next"
          />
        </div>
      </PageState>
    </el-card>

    <!-- 创建 / 编辑集群 -->
    <el-dialog
      v-model="formVisible"
      :title="editing ? '编辑集群' : '创建集群'"
      width="680px"
      destroy-on-close
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-position="vertical">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="名称" prop="name">
              <el-input v-model="form.name" placeholder="例如：gpu-cluster-sh" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="节点数" prop="nodes">
              <el-input-number v-model="form.nodes" :min="1" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="GPU数" prop="gpus">
              <el-input-number v-model="form.gpus" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="GPU厂商（逗号分隔）">
              <el-input v-model="form.gpu_vendors" placeholder="例如：nvidia,enflame" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="调度器类型（逗号分隔）">
              <el-input v-model="form.scheduler_types" placeholder="例如：slurm,lsf" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="联邦ID（多集群）">
              <el-input v-model="form.federation_id" placeholder="例如：fed-001" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="多集群联邦">
              <el-switch v-model="form.multi_cluster_enabled" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="动态扩容">
              <el-switch v-model="form.auto_expand" />
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="多方式访问（SSH / VNC / WEB）">
              <el-select v-model="form.access_methods" multiple allow-clear placeholder="选择支持的访问方式" style="width: 100%">
                <el-option label="SSH" value="ssh" />
                <el-option label="VNC" value="vnc" />
                <el-option label="WEB" value="web" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="管理平台高可用(HA)">
              <el-switch v-model="form.ha_enabled" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="核心调度节点HA">
              <el-switch v-model="form.scheduler_ha_enabled" />
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="描述">
              <el-input v-model="form.description" type="textarea" :rows="3" placeholder="选填，便于后续追溯" />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="formVisible = false">取消</el-button>
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" :loading="submitting" @click="handleSubmit">
            {{ editing ? '保存' : '创建' }}
          </el-button>
        </Can>
      </template>
    </el-dialog>

    <!-- 动态扩容 -->
    <el-dialog v-model="scaleVisible" :title="`动态扩容 - ${scaleTarget?.name ?? ''}`" width="480px">
      <p class="scale-tip">调整后集群将按新的节点 / GPU 容量重新调度，现有作业不受影响。</p>
      <el-row :gutter="16">
        <el-col :span="12">
          <el-form-item label="节点数" label-position="top">
            <el-input-number v-model="scaleNodes" :min="1" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item label="GPU数" label-position="top">
            <el-input-number v-model="scaleGpus" :min="0" style="width: 100%" />
          </el-form-item>
        </el-col>
      </el-row>
      <template #footer>
        <el-button @click="scaleVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="handleSaveScale">下发扩容</el-button>
      </template>
    </el-dialog>

    <!-- 集群详情 -->
    <el-dialog v-model="detailVisible" :title="`集群详情 - ${detailCluster?.name ?? ''}`" width="760px">
      <template v-if="detailCluster">
        <el-row :gutter="16">
          <el-col :span="12"><p><b>状态：</b>{{ statusText(detailCluster.status) }}</p></el-col>
          <el-col :span="12"><p><b>节点数：</b>{{ detailCluster.nodes ?? '-' }}</p></el-col>
          <el-col :span="12"><p><b>GPU数：</b>{{ detailCluster.gpus ?? '-' }}</p></el-col>
          <el-col :span="12"><p><b>CPU数：</b>{{ detailCluster.cpus ?? '-' }}</p></el-col>
          <el-col :span="12"><p><b>内存(GB)：</b>{{ detailCluster.memory ?? '-' }}</p></el-col>
          <el-col :span="12"><p><b>存储(TB)：</b>{{ detailCluster.storage ?? '-' }}</p></el-col>
          <el-col :span="12"><p><b>网络类型：</b>{{ detailCluster.network_type ?? '-' }}</p></el-col>
          <el-col :span="12"><p><b>位置：</b>{{ detailCluster.location ?? '-' }}</p></el-col>
          <el-col :span="12"><p><b>GPU厂商：</b>{{ detailCluster.gpu_vendors ?? '-' }}</p></el-col>
          <el-col :span="12"><p><b>调度器类型：</b>{{ detailCluster.scheduler_types ?? '-' }}</p></el-col>
          <el-col :span="12"><p><b>多集群：</b>{{ detailCluster.multi_cluster_enabled ? '启用' : '未启用' }}</p></el-col>
          <el-col :span="12"><p><b>联邦ID：</b>{{ detailCluster.federation_id ?? '-' }}</p></el-col>
        </el-row>

        <h3 class="detail-h">高可用</h3>
        <div class="detail-tags">
          <el-tag :type="detailCluster.ha_enabled ? 'success' : 'info'">
            管理平台HA {{ detailCluster.ha_enabled ? '开启' : '关闭' }}
          </el-tag>
          <el-tag :type="detailCluster.scheduler_ha_enabled ? 'success' : 'info'">
            调度节点HA {{ detailCluster.scheduler_ha_enabled ? '开启' : '关闭' }}
          </el-tag>
          <el-tag :type="detailCluster.auto_expand ? 'primary' : 'info'">
            动态扩容 {{ detailCluster.auto_expand ? '开启' : '关闭' }}
          </el-tag>
        </div>

        <h3 class="detail-h">访问方式</h3>
        <div class="detail-tags">
          <el-tag v-for="m in accessMethodList(detailCluster.access_methods)" :key="m" type="info">{{ m }}</el-tag>
        </div>

        <p class="detail-desc"><b>描述：</b>{{ detailCluster.description ?? '-' }}</p>

        <h3 class="detail-h">分区列表</h3>
        <el-table :data="clusterPartitions(detailCluster.id)" row-key="id" size="small" style="width: 100%">
          <el-table-column prop="name" label="名称" />
          <el-table-column prop="description" label="描述" show-overflow-tooltip />
          <el-table-column prop="priority" label="优先级" width="80" align="right" />
          <el-table-column prop="node_count" label="节点数" width="80" align="right" />
          <el-table-column prop="gpu_count" label="GPU数" width="80" align="right" />
          <el-table-column label="状态" width="100">
            <template #default="{ row }">
              <el-tag :type="statusTagType(row.status)" size="small">{{ statusText(row.status) }}</el-tag>
            </template>
          </el-table-column>
        </el-table>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Search } from '@element-plus/icons-vue'
import PageState from '@/components/PageState.vue'
import Can from '@/components/Can.vue'
import { clusterApi, partitionApi } from '@/api'
import { useFetch } from '@/utils/useFetch'
import type { Cluster, Partition } from '@/types'

type ClusterRecord = Cluster & {
  ha_enabled?: boolean
  scheduler_ha_enabled?: boolean
  access_methods?: string
  auto_expand?: boolean
}

const { data, loading, error, refetch } = useFetch<ClusterRecord[]>(() => clusterApi.list())
const partitionsF = useFetch<Partition[]>(() => partitionApi.list({}))

const clustersData = computed(() => data.value ?? [])
const partitionsData = computed(() => partitionsF.data.value ?? [])

// ---------- 搜索 / 筛选 ----------
const searchText = ref('')
const statusFilter = ref('')
const filteredClusters = computed(() => {
  const kw = searchText.value.trim().toLowerCase()
  return clustersData.value.filter((c) => {
    const matchKw =
      !kw ||
      c.name.toLowerCase().includes(kw) ||
      (c.description ?? '').toLowerCase().includes(kw) ||
      (c.location ?? '').toLowerCase().includes(kw)
    const matchStatus = !statusFilter.value || c.status === statusFilter.value
    return matchKw && matchStatus
  })
})

// ---------- 分页 ----------
const page = ref(1)
const pageSize = 10
const pagedClusters = computed(() =>
  filteredClusters.value.slice((page.value - 1) * pageSize, page.value * pageSize),
)

// ---------- 创建 / 编辑 ----------
const formVisible = ref(false)
const editing = ref<ClusterRecord | null>(null)
const submitting = ref(false)
const formRef = ref<FormInstance>()

const emptyForm = () => ({
  name: '',
  description: '',
  nodes: 1,
  gpus: 0,
  gpu_vendors: '',
  scheduler_types: '',
  federation_id: '',
  multi_cluster_enabled: false,
  auto_expand: false,
  access_methods: [] as string[],
  ha_enabled: false,
  scheduler_ha_enabled: false,
})
const form = reactive(emptyForm())
const rules: FormRules = {
  name: [{ required: true, message: '请输入集群名称', trigger: 'blur' }],
  nodes: [{ required: true, message: '请输入节点数', trigger: 'blur' }],
  gpus: [{ required: true, message: '请输入 GPU 数', trigger: 'blur' }],
}

function openCreate() {
  editing.value = null
  Object.assign(form, emptyForm())
  formVisible.value = true
}

function openEdit(row: ClusterRecord) {
  editing.value = row
  Object.assign(form, {
    name: row.name,
    description: row.description ?? '',
    nodes: row.nodes ?? 1,
    gpus: row.gpus ?? 0,
    gpu_vendors: row.gpu_vendors ?? '',
    scheduler_types: row.scheduler_types ?? '',
    federation_id: row.federation_id ?? '',
    multi_cluster_enabled: !!row.multi_cluster_enabled,
    auto_expand: !!row.auto_expand,
    access_methods: row.access_methods ? row.access_methods.split(',').map((s) => s.trim()) : [],
    ha_enabled: !!row.ha_enabled,
    scheduler_ha_enabled: !!row.scheduler_ha_enabled,
  })
  formVisible.value = true
}

async function handleSubmit() {
  if (!formRef.value) return
  try {
    await formRef.value.validate()
  } catch {
    return
  }
  submitting.value = true
  const payload = {
    ...form,
    access_methods: form.access_methods.length ? form.access_methods.join(',') : undefined,
  }
  try {
    if (editing.value) {
      await clusterApi.update(editing.value.id, payload)
      ElMessage.success('集群更新成功')
    } else {
      await clusterApi.create(payload)
      ElMessage.success('集群创建成功')
    }
    formVisible.value = false
    refetch()
  } catch {
    ElMessage.error(editing.value ? '集群更新失败，请稍后重试' : '集群创建失败，请检查必填项后重试')
  } finally {
    submitting.value = false
  }
}

async function handleDelete(row: ClusterRecord) {
  try {
    await clusterApi.remove(row.id)
    ElMessage.success('集群删除成功')
    refetch()
  } catch {
    ElMessage.error('集群删除失败，请稍后重试')
  }
}

async function handleRebuild(row: ClusterRecord) {
  try {
    await clusterApi.update(row.id, { status: 'rebuilding' })
    ElMessage.success('集群重建指令已下发')
    refetch()
  } catch {
    ElMessage.error('重建失败，请稍后重试')
  }
}

// ---------- 动态扩容 ----------
const scaleVisible = ref(false)
const scaleTarget = ref<ClusterRecord | null>(null)
const scaleNodes = ref(1)
const scaleGpus = ref(0)

function openScale(row: ClusterRecord) {
  scaleTarget.value = row
  scaleNodes.value = row.nodes ?? 1
  scaleGpus.value = row.gpus ?? 0
  scaleVisible.value = true
}

async function handleSaveScale() {
  if (!scaleTarget.value) return
  submitting.value = true
  try {
    await clusterApi.update(scaleTarget.value.id, {
      nodes: scaleNodes.value,
      gpus: scaleGpus.value,
    })
    ElMessage.success('集群扩容指令已下发')
    scaleVisible.value = false
    refetch()
  } catch {
    ElMessage.error('扩容失败，请稍后重试')
  } finally {
    submitting.value = false
  }
}

// ---------- 详情 ----------
const detailVisible = ref(false)
const detailCluster = ref<ClusterRecord | null>(null)
function openDetail(row: ClusterRecord) {
  detailCluster.value = row
  detailVisible.value = true
}
function clusterPartitions(clusterId: number) {
  return partitionsData.value.filter((p) => p.cluster_id === clusterId)
}

// ---------- 展示辅助 ----------
function splitTags(v: string) {
  return v.split(',').map((s) => s.trim()).filter(Boolean)
}
function accessMethodList(v?: string) {
  return (v ? v.split(',') : ['web']).map((m) => m.trim().toUpperCase())
}
function statusText(status?: string) {
  const map: Record<string, string> = {
    active: '活跃',
    inactive: '非活跃',
    maintenance: '维护',
    rebuilding: '重建中',
    running: '运行中',
    drained: '已排空',
  }
  return map[status ?? ''] ?? status ?? '-'
}
function statusTagType(status?: string): 'success' | 'warning' | 'danger' | 'info' | 'primary' {
  if (status === 'active' || status === 'running') return 'success'
  if (status === 'maintenance' || status === 'rebuilding' || status === 'pending') return 'warning'
  if (status === 'failed' || status === 'error') return 'danger'
  if (status === 'inactive' || status === 'drained') return 'info'
  return 'primary'
}
</script>

<style scoped>
.toolbar { display: flex; gap: 12px; margin-bottom: 16px; flex-wrap: wrap; }
.toolbar-search { flex: 1; min-width: 240px; max-width: 420px; }
.pager { display: flex; justify-content: flex-end; margin-top: 16px; }
.scale-tip { color: var(--mc-text-3); font-size: 12.5px; margin: 0 0 16px; }
.detail-h { margin: 18px 0 8px; font-size: 14px; }
.detail-tags { display: flex; flex-wrap: wrap; gap: 8px; }
.detail-desc { margin-top: 16px; }
.detail-desc :deep(p) { margin: 0 0 8px; }
</style>
