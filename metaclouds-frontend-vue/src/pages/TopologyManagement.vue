<template>
  <div class="mc-page">
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">拓扑管理</h1>
        <p class="mc-page-desc">共 {{ topologiesData.length }} 条拓扑记录 · 机架/交换机/NUMA/GPU 层级管理与评分</p>
      </div>
      <div class="mc-page-head-extra">
        <el-button @click="scoreVisible = true">拓扑评分工具</el-button>
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="openCreate">新增拓扑</el-button>
        </Can>
      </div>
    </div>

    <el-card shadow="never" id="topology-table">
      <div class="toolbar mc-mb">
        <el-input v-model="searchText" placeholder="搜索节点名 / 机架 / 交换机" clearable style="flex: 1; min-width: 220px" />
        <el-select v-model="clusterFilter" placeholder="集群" clearable style="width: 160px">
          <el-option v-for="c in clustersData" :key="c.id" :label="c.name" :value="c.id" />
        </el-select>
        <el-select v-model="networkFilter" placeholder="网络类型" clearable style="width: 150px">
          <el-option v-for="o in NETWORK_TYPE_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
      </div>

      <PageState
        :loading="topologies.loading.value"
        :error="topologies.error.value"
        :data="pagedData"
        empty-text="暂无节点拓扑，登记后可进行拓扑感知调度评分"
        @retry="topologies.refetch"
      >
        <el-table :data="pagedData" stripe border style="width: 100%">
          <el-table-column label="ID" width="70">
            <template #default="{ row }"><span class="mc-mono">{{ row.id }}</span></template>
          </el-table-column>
          <el-table-column label="节点名" width="150">
            <template #default="{ row }">
              <el-button link type="primary" @click="openDetail(row)">{{ row.hostname }}</el-button>
            </template>
          </el-table-column>
          <el-table-column label="集群" width="120">
            <template #default="{ row }">{{ clusterName(row.cluster_id) }}</template>
          </el-table-column>
          <el-table-column prop="rack_id" label="机架ID" width="100">
            <template #default="{ row }">{{ row.labels?.rack ?? '-' }}</template>
          </el-table-column>
          <el-table-column prop="switch_id" label="交换机ID" width="110">
            <template #default="{ row }">{{ row.switch_id ?? '-' }}</template>
          </el-table-column>
          <el-table-column prop="pod_id" label="Pod ID" width="100">
            <template #default="{ row }">{{ row.pod_id ?? '-' }}</template>
          </el-table-column>
          <el-table-column label="NUMA节点" width="160" show-overflow-tooltip>
            <template #default="{ row }">{{ row.numa_nodes ?? '-' }}</template>
          </el-table-column>
          <el-table-column label="GPU拓扑" width="160" show-overflow-tooltip>
            <template #default="{ row }">{{ row.gpu_topology ?? '-' }}</template>
          </el-table-column>
          <el-table-column label="网络类型" width="110">
            <template #default="{ row }">
              <el-tag v-if="netLabel(row.network_type)" type="primary" size="small">{{ netLabel(row.network_type) }}</el-tag>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column label="带宽(Gbps)" width="110">
            <template #default="{ row }"><span class="mc-num">{{ row.network_bandwidth_gbps ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="RDMA" width="80">
            <template #default="{ row }">
              <el-tag :type="row.rdma_enabled ? 'success' : 'info'" size="small">{{ row.rdma_enabled ? '开启' : '关闭' }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column prop="fault_domain" label="故障域" width="110">
            <template #default="{ row }">{{ row.fault_domain ?? '-' }}</template>
          </el-table-column>
          <el-table-column label="操作" width="200" fixed="right">
            <template #default="{ row }">
              <Can :roles="['admin', 'manager']">
                <el-button link type="primary" size="small" @click="openEdit(row)">编辑</el-button>
                <el-popconfirm title="删除该拓扑记录？拓扑配置将被移除且不可恢复。" confirm-button-text="确认删除" cancel-button-text="返回" @confirm="handleDelete(row.id)">
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
          :total="filteredTopologies.length"
          :page-size="pageSize"
          style="margin-top: 12px; justify-content: flex-end"
        />
      </PageState>
    </el-card>

    <!-- 新建/编辑 Dialog -->
    <el-dialog
      v-model="dialogVisible"
      :title="editingTopology ? '编辑拓扑' : '新增拓扑'"
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
            <el-form-item label="集群" prop="cluster_id">
              <el-select v-model="form.cluster_id" placeholder="请选择集群" clearable style="width: 100%">
                <el-option v-for="c in clustersData" :key="c.id" :label="c.name" :value="c.id" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="机架 ID" prop="rack_id">
              <el-input v-model="form.rack_id" placeholder="例如：rack-01" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="交换机 ID" prop="switch_id">
              <el-input v-model="form.switch_id" placeholder="例如：sw-01" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="Pod ID" prop="pod_id">
              <el-input v-model="form.pod_id" placeholder="例如：pod-a" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="网络类型" prop="network_type">
              <el-select v-model="form.network_type" placeholder="请选择网络类型" clearable style="width: 100%">
                <el-option v-for="o in NETWORK_TYPE_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="带宽 (Gbps)" prop="network_bandwidth_gbps">
              <el-input-number v-model="form.network_bandwidth_gbps" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="RDMA 开启" prop="rdma_enabled">
              <el-switch v-model="form.rdma_enabled" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="故障域" prop="fault_domain">
              <el-input v-model="form.fault_domain" placeholder="例如：rack-01" />
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="NUMA 节点 (JSON)" prop="numa_nodes">
              <el-input v-model="form.numa_nodes" type="textarea" :rows="2" placeholder='例如：[{"node":0,"cpus":"0-31"},{"node":1,"cpus":"32-63"}]' />
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="GPU 拓扑 (JSON)" prop="gpu_topology">
              <el-input v-model="form.gpu_topology" type="textarea" :rows="2" placeholder='例如：{"nvlink":[[0,1],[1,2]],"pcie":"x16"}' />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="submitForm">{{ editingTopology ? '保存' : '创建' }}</el-button>
        </Can>
      </template>
    </el-dialog>

    <!-- 详情 Drawer -->
    <el-drawer v-model="detailVisible" title="节点拓扑详情" size="480px">
      <div v-if="detailTopology">
        <div v-for="row in detailRows" :key="row.label" class="detail-row">
          <span class="detail-label">{{ row.label }}</span>
          <span class="detail-value">{{ row.value }}</span>
        </div>
      </div>
    </el-drawer>

    <!-- 拓扑评分工具 Dialog -->
    <el-dialog v-model="scoreVisible" title="拓扑评分工具" width="720px">
      <el-form label-position="top">
        <el-form-item label="选择作业">
          <el-select v-model="scoreJobId" placeholder="请选择作业" clearable style="width: 100%">
            <el-option v-for="j in jobsData" :key="j.id" :label="j.name" :value="j.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="候选节点列表（逗号分隔）">
          <el-input v-model="scoreCandidateNodes" placeholder="例如：gpu-node-01, gpu-node-02, gpu-node-03" />
        </el-form-item>
      </el-form>

      <div v-if="scoreEntries.length" class="mc-mt">
        <h3 class="score-title">拓扑评分结果</h3>
        <div v-for="e in scoreEntries" :key="e.name" class="score-row">
          <span class="score-name">{{ e.name }}</span>
          <el-progress :percentage="e.score" :stroke-width="14" :color="scoreColor(e.score)" style="flex: 1" />
        </div>
      </div>
      <el-empty v-else-if="scored" description="暂无评分结果" :image-size="60" />

      <template #footer>
        <el-button @click="scoreVisible = false; scoreResults = null">关闭</el-button>
        <el-button type="primary" :loading="scoring" @click="calculateScore">计算评分</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue'
import { ElMessage } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import PageState from '@/components/PageState.vue'
import Can from '@/components/Can.vue'
import { topologyApi, clusterApi, jobApi } from '@/api'
import { useFetch } from '@/utils/useFetch'
import type { NodeTopology, Cluster, Job } from '@/types'

const NETWORK_TYPE_OPTIONS = [
  { label: 'RoCE', value: 'roce' },
  { label: 'InfiniBand', value: 'ib' },
  { label: 'Ethernet', value: 'ethernet' },
]
function netLabel(v?: string) {
  return NETWORK_TYPE_OPTIONS.find((o) => o.value === v)?.label
}

// ---------- 筛选 ----------
const searchText = ref('')
const clusterFilter = ref<number | undefined>(undefined)
const networkFilter = ref<string>('')

// ---------- 数据 ----------
const topologies = useFetch<NodeTopology[]>(() =>
  topologyApi.list({ cluster_id: clusterFilter.value, page_size: 1000 }),
)
const clusters = useFetch<Cluster[]>(() => clusterApi.list())
const jobs = useFetch<Job[]>(() => jobApi.list())

watch(clusterFilter, () => topologies.refetch())

const topologiesData = computed(() => topologies.data.value ?? [])
const clustersData = computed(() => clusters.data.value ?? [])
const jobsData = computed(() => jobs.data.value ?? [])

function clusterName(id?: number) {
  return clustersData.value.find((c) => c.id === id)?.name ?? '-'
}

const filteredTopologies = computed(() => {
  const kw = searchText.value.trim().toLowerCase()
  return topologiesData.value.filter((t) => {
    const matchKw =
      !kw ||
      t.node_name.toLowerCase().includes(kw) ||
      (t.rack_id ?? '').toLowerCase().includes(kw) ||
      (t.switch_id ?? '').toLowerCase().includes(kw)
    const matchCluster = clusterFilter.value == null || t.cluster_id === clusterFilter.value
    const matchNetwork = !networkFilter.value || t.network_type === networkFilter.value
    return matchKw && matchCluster && matchNetwork
  })
})

// 分页
const currentPage = ref(1)
const pageSize = 10
const pagedData = computed(() =>
  filteredTopologies.value.slice((currentPage.value - 1) * pageSize, currentPage.value * pageSize),
)
watch(filteredTopologies, () => (currentPage.value = 1))

// ---------- 表单 ----------
const formRef = ref<FormInstance>()
const dialogVisible = ref(false)
const editingTopology = ref<NodeTopology | null>(null)
const form = reactive({
  node_name: '',
  cluster_id: undefined as number | undefined,
  rack_id: '',
  switch_id: '',
  pod_id: '',
  numa_nodes: '',
  gpu_topology: '',
  network_type: undefined as NodeTopology['network_type'],
  network_bandwidth_gbps: undefined as number | undefined,
  rdma_enabled: false,
  fault_domain: '',
})
const rules: FormRules = {
  node_name: [{ required: true, message: '请输入节点名', trigger: 'blur' }],
}

function resetForm() {
  Object.assign(form, {
    node_name: '', cluster_id: undefined, rack_id: '', switch_id: '', pod_id: '',
    numa_nodes: '', gpu_topology: '', network_type: undefined, network_bandwidth_gbps: undefined,
    rdma_enabled: false, fault_domain: '',
  })
}
function openCreate() {
  editingTopology.value = null
  resetForm()
  dialogVisible.value = true
}
function openEdit(row: NodeTopology) {
  editingTopology.value = row
  Object.assign(form, {
    node_name: row.node_name, cluster_id: row.cluster_id, rack_id: row.rack_id ?? '', switch_id: row.switch_id ?? '',
    pod_id: row.pod_id ?? '', numa_nodes: row.numa_nodes ?? '', gpu_topology: row.gpu_topology ?? '',
    network_type: row.network_type, network_bandwidth_gbps: row.network_bandwidth_gbps,
    rdma_enabled: row.rdma_enabled ?? false, fault_domain: row.fault_domain ?? '',
  })
  dialogVisible.value = true
}
async function submitForm() {
  await formRef.value?.validate()
  try {
    if (editingTopology.value) {
      await topologyApi.update(editingTopology.value.id, { ...form })
      ElMessage.success('拓扑更新成功')
    } else {
      await topologyApi.create({ ...form })
      ElMessage.success('拓扑创建成功')
    }
    dialogVisible.value = false
    resetForm()
    topologies.refetch()
  } catch {
    ElMessage.error('操作失败，请检查必填项后重试')
  }
}
async function handleDelete(id: number) {
  try {
    await topologyApi.remove(id)
    ElMessage.success('拓扑删除成功')
    topologies.refetch()
  } catch {
    ElMessage.error('删除失败，请稍后重试')
  }
}

// ---------- 详情 ----------
const detailVisible = ref(false)
const detailTopology = ref<NodeTopology | null>(null)
function openDetail(row: NodeTopology) {
  detailTopology.value = row
  detailVisible.value = true
}
const detailRows = computed(() => {
  const d = detailTopology.value
  if (!d) return []
  return [
    { label: '节点名', value: d.node_name },
    { label: '集群', value: clusterName(d.cluster_id) },
    { label: '机架 ID', value: d.rack_id ?? '-' },
    { label: '交换机 ID', value: d.switch_id ?? '-' },
    { label: 'Pod ID', value: d.pod_id ?? '-' },
    { label: '网络类型', value: d.network_type ?? '-' },
    { label: '带宽', value: `${d.network_bandwidth_gbps ?? '-'} Gbps` },
    { label: 'RDMA', value: d.rdma_enabled ? '开启' : '关闭' },
    { label: '故障域', value: d.fault_domain ?? '-' },
    { label: 'NUMA 节点', value: d.numa_nodes ?? '-' },
    { label: 'GPU 拓扑', value: d.gpu_topology ?? '-' },
  ]
})

// ---------- 拓扑评分 ----------
const scoreVisible = ref(false)
const scoreJobId = ref<number | undefined>(undefined)
const scoreCandidateNodes = ref('')
const scoreResults = ref<Record<string, number> | null>(null)
const scoring = ref(false)
const scored = ref(false)

const scoreEntries = computed(() => {
  if (!scoreResults.value) return []
  return Object.entries(scoreResults.value)
    .map(([name, score]) => ({ name, score }))
    .sort((a, b) => b.score - a.score)
})
function scoreColor(score: number) {
  return score >= 80 ? '#00b8a9' : score >= 50 ? '#2f6bff' : '#faad14'
}

async function calculateScore() {
  if (!scoreJobId.value) {
    ElMessage.warning('请选择作业')
    return
  }
  const candidates = scoreCandidateNodes.value.split(',').map((s) => s.trim()).filter(Boolean)
  if (candidates.length === 0) {
    ElMessage.warning('请输入候选节点列表')
    return
  }
  scoring.value = true
  try {
    const result = await topologyApi.calculateScore({ job_id: scoreJobId.value, candidate_nodes: candidates })
    scoreResults.value = result
    scored.value = true
    ElMessage.success('拓扑评分计算完成')
  } catch {
    ElMessage.error('评分计算失败，请稍后重试')
  } finally {
    scoring.value = false
  }
}
</script>

<style scoped>
.toolbar { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; }
.detail-row { display: flex; padding: 8px 0; border-bottom: 1px solid var(--mc-line); }
.detail-label { width: 110px; color: var(--mc-text-3); flex-shrink: 0; }
.detail-value { flex: 1; word-break: break-all; }
.score-title { margin: 0 0 12px; font-size: 15px; font-weight: 600; }
.score-row { display: flex; align-items: center; gap: 12px; margin-bottom: 10px; }
.score-name { width: 150px; flex-shrink: 0; font-size: 13px; color: var(--mc-text-2); }
</style>
