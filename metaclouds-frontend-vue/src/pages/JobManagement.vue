<template>
  <div class="mc-page">
    <!-- 页头：统计 + 新建按钮 -->
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">作业管理</h1>
        <p class="mc-page-desc">
          共 {{ stats.total }} 个作业 · 排队 {{ stats.pending }} 个 · 运行中 {{ stats.running }} 个
        </p>
      </div>
      <div class="mc-page-head-extra">
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" @click="openCreate">新建作业</el-button>
        </Can>
      </div>
    </div>

    <el-card id="job-table">
      <!-- 搜索 + 状态 / 类型 / 分区 筛选工具栏 -->
      <div class="mc-flow mc-mb">
        <el-input
          v-model="searchText"
          placeholder="搜索作业名称 / 描述"
          clearable
          style="width: 240px"
          aria-label="搜索作业"
        />
        <el-select v-model="statusFilter" placeholder="状态" clearable style="width: 130px">
          <el-option v-for="o in STATUS_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
        <el-select v-model="typeFilter" placeholder="类型" clearable style="width: 130px">
          <el-option v-for="o in TYPE_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
        </el-select>
        <el-select v-model="partitionFilter" placeholder="分区" clearable style="width: 160px">
          <el-option v-for="p in partitions" :key="p.id" :label="p.name" :value="p.id" />
        </el-select>
      </div>

      <!-- 子路由驱动的 Tab 切换 -->
      <el-tabs :model-value="activeTab" @tab-change="onTabChange">
        <el-tab-pane label="作业列表" name="/job/list" />
        <el-tab-pane label="任务队列" name="/job/queue" />
        <el-tab-pane label="历史记录" name="/job/history" />
      </el-tabs>

      <!-- 三态守卫：loading / error / empty，其余渲染表格 -->
      <PageState
        :loading="jobsLoading"
        :error="jobsError"
        :data="pagedJobs"
        :empty-text="emptyText"
        @retry="loadJobs"
      >
        <el-table :data="pagedJobs" stripe style="width: 100%" row-key="id">
          <el-table-column prop="id" label="ID" width="80">
            <template #default="{ row }"><span class="mc-mono">{{ row.id }}</span></template>
          </el-table-column>
          <el-table-column label="名称" min-width="160">
            <template #default="{ row }">
              <el-button type="primary" link @click="openDetail(row)">{{ row.name }}</el-button>
            </template>
          </el-table-column>
          <el-table-column prop="type" label="类型" width="90">
            <template #default="{ row }">{{ row.type || '-' }}</template>
          </el-table-column>
          <el-table-column label="状态" width="110">
            <template #default="{ row }">
              <span class="mc-status" :class="statusClass(row.status)">
                <i class="mc-status-dot" />{{ statusText(row.status) }}
              </span>
            </template>
          </el-table-column>
          <el-table-column prop="priority" label="优先级" width="90">
            <template #default="{ row }">
              <el-tag v-if="row.priority != null" :type="priorityTagType(row.priority)">{{ priorityText(row.priority) }}</el-tag>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column prop="gpus" label="GPU数" width="80">
            <template #default="{ row }"><span class="mc-num">{{ row.gpus ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column prop="cpus" label="CPU数" width="80">
            <template #default="{ row }"><span class="mc-num">{{ row.cpus ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column prop="memory" label="内存(GB)" width="100">
            <template #default="{ row }"><span class="mc-num">{{ row.memory ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="进度" width="160">
            <template #default="{ row }">
              <el-progress v-if="row.status === 'running' || row.status === 'pending'"
                :percentage="row.progress ?? 0" :stroke-width="10" />
              <span v-else class="mc-num">{{ row.progress ?? 0 }}%</span>
            </template>
          </el-table-column>
          <el-table-column label="分区" width="120">
            <template #default="{ row }">{{ partitionName(row.partition_id) }}</template>
          </el-table-column>
          <el-table-column label="调度器" width="120">
            <template #default="{ row }">
              <el-tag v-if="schedulerLabel(row.scheduler_type)">{{ schedulerLabel(row.scheduler_type) }}</el-tag>
              <span v-else>-</span>
            </template>
          </el-table-column>
          <el-table-column prop="created_at" label="创建时间" width="170">
            <template #default="{ row }">{{ row.created_at || '-' }}</template>
          </el-table-column>
          <el-table-column v-if="activeTab === '/job/history'" label="错误信息" min-width="160">
            <template #default="{ row }">{{ row.error_msg || '-' }}</template>
          </el-table-column>
          <el-table-column label="操作" width="240" fixed="right">
            <template #default="{ row }">
              <Can :roles="['admin', 'manager']">
                <el-button v-if="row.status === 'pending'" type="primary" link size="small" @click="handleSubmitToK8S(row)">
                  提交到K8S
                </el-button>
              </Can>
              <el-button v-if="row.status === 'running' || row.status === 'pending'" type="danger" link size="small" @click="handleCancel(row)">
                取消
              </el-button>
              <el-button type="primary" link size="small" @click="openDetail(row)">查看详情</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-pagination
          class="mc-mt"
          layout="total, sizes, prev, pager, next"
          :total="filteredJobs.length"
          v-model:current-page="page.current"
          v-model:page-size="page.size"
          :page-sizes="[10, 20, 50]"
          background
        />
      </PageState>
    </el-card>

    <!-- 创建作业 Dialog -->
    <el-dialog v-model="createVisible" title="创建作业" width="860px" destroy-on-close @closed="resetForm">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="130px">
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item label="名称" prop="name">
              <el-input v-model="form.name" placeholder="例如：llama3-70b-finetune" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="类型" prop="type">
              <el-select v-model="form.type" placeholder="请选择类型" style="width: 100%">
                <el-option label="训练" value="training" />
                <el-option label="推理" value="inference" />
                <el-option label="批处理" value="batch" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="优先级" prop="priority">
              <el-select v-model="form.priority" placeholder="请选择优先级" style="width: 100%">
                <el-option v-for="o in PRIORITY_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="GPU数量" prop="gpus">
              <el-input-number v-model="form.gpus" :min="1" :max="8" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="CPU数">
              <el-input-number v-model="form.cpus" :min="1" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="内存(GB)">
              <el-input-number v-model="form.memory" :min="1" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="GPU分数">
              <el-select v-model="form.gpu_fraction" placeholder="请选择分数" clearable style="width: 100%">
                <el-option v-for="o in GPU_FRACTION_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="GPU显存(GB)">
              <el-input-number v-model="form.gpu_memory_gb" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="GPU厂商">
              <el-select v-model="form.gpu_vendor" placeholder="请选择厂商" clearable style="width: 100%">
                <el-option v-for="o in GPU_VENDOR_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="分区">
              <el-select v-model="form.partition_id" placeholder="请选择分区" clearable style="width: 100%">
                <el-option v-for="p in partitions" :key="p.id" :label="p.name" :value="p.id" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="调度器类型">
              <el-select v-model="form.scheduler_type" placeholder="请选择调度器" clearable style="width: 100%">
                <el-option v-for="o in SCHEDULER_TYPE_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="QoS">
              <el-input v-model="form.qos" placeholder="例如：normal" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="请求节点数">
              <el-input-number v-model="form.nodes_requested" :min="1" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="网络需求">
              <el-select v-model="form.network_requirement" placeholder="请选择网络需求" clearable style="width: 100%">
                <el-option v-for="o in NETWORK_REQUIREMENT_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="24">
            <el-form-item label="节点选择器 (JSON)">
              <el-input v-model="form.node_selector" type="textarea" :rows="2" placeholder='例如：{"gpu-type":"a100"}' />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="亲和性 (JSON)">
              <el-input v-model="form.affinity" type="textarea" :rows="2" placeholder="K8s affinity 配置" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="容忍度 (JSON)">
              <el-input v-model="form.tolerations" type="textarea" :rows="2" placeholder="K8s tolerations 配置" />
            </el-form-item>
          </el-col>
          <!-- 弹性训练 -->
          <el-col :span="8">
            <el-form-item label="弹性训练">
              <el-switch v-model="form.elastic_enabled" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="最小GPU">
              <el-input-number v-model="form.min_gpus" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="最大GPU">
              <el-input-number v-model="form.max_gpus" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="扩缩容策略">
              <el-input v-model="form.scaling_policy" placeholder="例如：cpu_utilization" />
            </el-form-item>
          </el-col>
          <!-- 容错训练 -->
          <el-col :span="8">
            <el-form-item label="Checkpoint">
              <el-switch v-model="form.checkpoint_enabled" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="Checkpoint间隔(分钟)">
              <el-input-number v-model="form.checkpoint_interval_minutes" :min="1" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="最大重试次数">
              <el-input-number v-model="form.max_retries" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="容错级别">
              <el-select v-model="form.fault_tolerance_level" placeholder="请选择容错级别" clearable style="width: 100%">
                <el-option v-for="o in FAULT_TOLERANCE_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <!-- 拓扑感知 -->
          <el-col :span="8">
            <el-form-item label="拓扑亲和">
              <el-select v-model="form.topology_affinity" placeholder="请选择拓扑亲和" clearable style="width: 100%">
                <el-option v-for="o in TOPOLOGY_AFFINITY_OPTIONS" :key="o.value" :label="o.label" :value="o.value" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="拓扑反亲和">
              <el-switch v-model="form.topology_anti_affinity" />
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
        <el-button @click="createVisible = false">取消</el-button>
        <Can :roles="['admin', 'manager']">
          <el-button type="primary" :loading="creating" @click="submitCreate">创建</el-button>
        </Can>
      </template>
    </el-dialog>

    <!-- 作业详情 Dialog（含 Checkpoint 子视图） -->
    <el-dialog v-model="detailVisible" title="作业详情" width="720px" destroy-on-close>
      <template v-if="detailJob">
        <el-descriptions :column="2" border size="small">
          <el-descriptions-item label="名称">{{ detailJob.name }}</el-descriptions-item>
          <el-descriptions-item label="类型">{{ detailJob.type || '-' }}</el-descriptions-item>
          <el-descriptions-item label="状态">
            <span class="mc-status" :class="statusClass(detailJob.status)">
              <i class="mc-status-dot" />{{ statusText(detailJob.status) }}
            </span>
          </el-descriptions-item>
          <el-descriptions-item label="优先级">{{ priorityText(detailJob.priority) }}</el-descriptions-item>
          <el-descriptions-item label="GPU数量">{{ detailJob.gpus ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="GPU分数">{{ detailJob.gpu_fraction ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="CPU数">{{ detailJob.cpus ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="内存(GB)">{{ detailJob.memory ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="GPU显存">{{ detailJob.gpu_memory_gb ? detailJob.gpu_memory_gb + ' GB' : '-' }}</el-descriptions-item>
          <el-descriptions-item label="GPU厂商">{{ detailJob.gpu_vendor || '-' }}</el-descriptions-item>
          <el-descriptions-item label="分区">{{ partitionName(detailJob.partition_id) }}</el-descriptions-item>
          <el-descriptions-item label="调度器类型">{{ detailJob.scheduler_type || '-' }}</el-descriptions-item>
          <el-descriptions-item label="调度器作业ID">{{ detailJob.scheduler_job_id || '-' }}</el-descriptions-item>
          <el-descriptions-item label="QoS">{{ detailJob.qos || '-' }}</el-descriptions-item>
          <el-descriptions-item label="请求节点数">{{ detailJob.nodes_requested ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="弹性训练">{{ detailJob.elastic_enabled ? '开启' : '关闭' }}</el-descriptions-item>
          <template v-if="detailJob.elastic_enabled">
            <el-descriptions-item label="最小GPU">{{ detailJob.min_gpus ?? '-' }}</el-descriptions-item>
            <el-descriptions-item label="最大GPU">{{ detailJob.max_gpus ?? '-' }}</el-descriptions-item>
          </template>
          <el-descriptions-item label="Checkpoint">{{ detailJob.checkpoint_enabled ? '开启' : '关闭' }}</el-descriptions-item>
          <el-descriptions-item v-if="detailJob.checkpoint_enabled" label="Checkpoint间隔">
            {{ detailJob.checkpoint_interval_minutes ?? '-' }} 分钟
          </el-descriptions-item>
          <el-descriptions-item label="最大重试次数">{{ detailJob.max_retries ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="已重试次数">{{ detailJob.retry_count ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="容错级别">{{ detailJob.fault_tolerance_level || '-' }}</el-descriptions-item>
          <el-descriptions-item label="拓扑亲和">{{ detailJob.topology_affinity || '-' }}</el-descriptions-item>
          <el-descriptions-item label="拓扑反亲和">{{ detailJob.topology_anti_affinity ? '开启' : '关闭' }}</el-descriptions-item>
          <el-descriptions-item label="网络需求">{{ detailJob.network_requirement || '-' }}</el-descriptions-item>
          <el-descriptions-item label="节点选择器" :span="2">{{ detailJob.node_selector || '-' }}</el-descriptions-item>
          <el-descriptions-item label="亲和性" :span="2">{{ detailJob.affinity || '-' }}</el-descriptions-item>
          <el-descriptions-item label="容忍度" :span="2">{{ detailJob.tolerations || '-' }}</el-descriptions-item>
          <el-descriptions-item label="进度">{{ detailJob.progress ?? 0 }}%</el-descriptions-item>
          <el-descriptions-item label="描述" :span="2">{{ detailJob.description || '-' }}</el-descriptions-item>
        </el-descriptions>

        <!-- Checkpoint 列表子视图 -->
        <el-divider content-position="left">Checkpoint 记录</el-divider>
        <el-table :data="checkpoints" size="small" stripe>
          <el-table-column prop="id" label="ID" width="70" />
          <el-table-column prop="path" label="路径" min-width="200" show-overflow-tooltip />
          <el-table-column prop="step" label="Step" width="80" />
          <el-table-column prop="epoch" label="Epoch" width="80" />
          <el-table-column prop="size_mb" label="大小(MB)" width="100" />
          <el-table-column prop="format" label="格式" width="110" />
          <el-table-column label="状态" width="110">
            <template #default="{ row }">
              <el-tag :type="checkpointTagType(row.status)">{{ row.status || '-' }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="最新" width="80">
            <template #default="{ row }">
              <el-tag v-if="row.is_latest" type="success">最新</el-tag>
            </template>
          </el-table-column>
        </el-table>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { jobApi, partitionApi, checkpointApi } from '@/api'
import type { Job, JobStatus, JobType, JobPriority, GPUVendor, Checkpoint, Partition } from '@/types'
import { useFetch, useMutation } from '@/utils/useFetch'
import PageState from '@/components/PageState.vue'
import Can from '@/components/Can.vue'

const route = useRoute()
const router = useRouter()

// ---------- 常量 ----------
const STATUS_OPTIONS: { label: string; value: JobStatus }[] = [
  { label: '排队中', value: 'pending' },
  { label: '运行中', value: 'running' },
  { label: '已完成', value: 'completed' },
  { label: '失败', value: 'failed' },
  { label: '已取消', value: 'cancelled' },
]
const TYPE_OPTIONS: { label: string; value: JobType }[] = [
  { label: '训练', value: 'training' },
  { label: '推理', value: 'inference' },
  { label: '批处理', value: 'batch' },
]
const PRIORITY_OPTIONS: { label: string; value: JobPriority }[] = [
  { label: '低', value: 0 },
  { label: '中', value: 1 },
  { label: '高', value: 2 },
  { label: '紧急', value: 3 },
]
const SCHEDULER_TYPE_OPTIONS = [
  { label: 'K8s Native', value: 'k8s_native' },
  { label: 'Slurm', value: 'slurm' },
  { label: 'LSF', value: 'lsf' },
  { label: 'SGE', value: 'sge' },
]
const GPU_VENDOR_OPTIONS: { label: string; value: GPUVendor }[] = [
  { label: 'NVIDIA', value: 'nvidia' },
  { label: '燧原', value: 'enflame' },
  { label: '摩尔线程', value: 'moore_threads' },
  { label: '国产 X', value: 'domestic_x' },
]
const GPU_FRACTION_OPTIONS = [
  { label: '1 GPU', value: 1 },
  { label: '1/2 GPU', value: 0.5 },
  { label: '1/4 GPU', value: 0.25 },
]
const FAULT_TOLERANCE_OPTIONS = [
  { label: '无', value: 'none' },
  { label: '节点级', value: 'node' },
  { label: '机架级', value: 'rack' },
  { label: '交换机级', value: 'switch' },
]
const TOPOLOGY_AFFINITY_OPTIONS = [
  { label: '节点', value: 'node' },
  { label: '机架', value: 'rack' },
  { label: '交换机', value: 'switch' },
  { label: '集群', value: 'cluster' },
]
const NETWORK_REQUIREMENT_OPTIONS = [
  { label: 'RDMA', value: 'rdma' },
  { label: '以太网', value: 'ethernet' },
  { label: '任意', value: 'any' },
]

// ---------- 数据 ----------
const { data: jobsRaw, loading: jobsLoading, error: jobsError, execute: loadJobs } = useFetch<Job[]>(() => jobApi.list())
const { data: partitionsRaw, execute: loadPartitions } = useFetch<Partition[]>(() => partitionApi.list())
const jobs = computed<Job[]>(() => jobsRaw.value ?? [])
const partitions = computed<Partition[]>(() => partitionsRaw.value ?? [])

const createMut = useMutation<Job, Partial<Job>>((data) => jobApi.create(data))
const cancelMut = useMutation<void, number>((id) => jobApi.cancel(id))
const submitMut = useMutation<void, number>((id) => jobApi.submitToK8S(id))
const creating = computed(() => createMut.loading.value)

// ---------- Tab 由 route.path 决定 ----------
const activeTab = computed(() => {
  const p = route.path
  if (p.startsWith('/job/queue')) return '/job/queue'
  if (p.startsWith('/job/history')) return '/job/history'
  return '/job/list'
})
function onTabChange(name: string) {
  router.push(name)
}

// ---------- 搜索 / 筛选 ----------
const searchText = ref('')
const statusFilter = ref<JobStatus | ''>('')
const typeFilter = ref<JobType | ''>('')
const partitionFilter = ref<number | undefined>(undefined)

const emptyText = computed(() => {
  if (activeTab.value === '/job/queue') return '队列为空：当前没有排队中的作业'
  if (activeTab.value === '/job/history') return '暂无历史记录：作业完成、失败或取消后会归档到这里'
  return '还没有作业：提交第一个作业后，就可以在这里跟踪它的运行状态'
})

const filteredJobs = computed(() => {
  let list = jobs.value
  if (activeTab.value === '/job/queue') {
    list = list.filter((j) => j.status === 'pending')
  } else if (activeTab.value === '/job/history') {
    list = list.filter((j) => ['completed', 'failed', 'cancelled'].includes(j.status))
  }
  const kw = searchText.value.trim().toLowerCase()
  return list.filter((j) => {
    const matchKw = !kw || j.name.toLowerCase().includes(kw) || (j.description ?? '').toLowerCase().includes(kw)
    const matchStatus = !statusFilter.value || j.status === statusFilter.value
    const matchType = !typeFilter.value || j.type === typeFilter.value
    const matchPartition = partitionFilter.value == null || j.partition_id === partitionFilter.value
    return matchKw && matchStatus && matchType && matchPartition
  })
})

// ---------- 分页 ----------
const page = reactive({ current: 1, size: 10 })
const pagedJobs = computed(() => {
  const start = (page.current - 1) * page.size
  return filteredJobs.value.slice(start, start + page.size)
})

// ---------- 页头统计 ----------
const stats = computed(() => ({
  total: jobs.value.length,
  pending: jobs.value.filter((j) => j.status === 'pending').length,
  running: jobs.value.filter((j) => j.status === 'running').length,
}))

// ---------- 展示辅助 ----------
function statusClass(s?: string) {
  return ['running', 'pending', 'completed', 'failed', 'cancelled'].includes(s ?? '') ? (s as string) : 'idle'
}
function statusText(s?: string) {
  const map: Record<string, string> = {
    pending: '排队中', running: '运行中', completed: '已完成', failed: '失败', cancelled: '已取消',
  }
  return map[s ?? ''] ?? s ?? '-'
}
function priorityText(p?: number) {
  const map: Record<number, string> = { 0: '低', 1: '中', 2: '高', 3: '紧急' }
  return p == null ? '-' : map[p] ?? String(p)
}
function priorityTagType(p?: number) {
  if (p === 3) return 'danger'
  if (p === 2) return 'warning'
  if (p === 1) return 'primary'
  return 'info'
}
function partitionName(id?: number) {
  if (id == null) return '-'
  return partitions.value.find((p) => p.id === id)?.name ?? '-'
}
function schedulerLabel(v?: string) {
  return SCHEDULER_TYPE_OPTIONS.find((o) => o.value === v)?.label
}
function checkpointTagType(s?: string) {
  if (s === 'completed') return 'success'
  if (s === 'failed' || s === 'corrupted') return 'danger'
  if (s === 'in_progress') return 'warning'
  return 'info'
}

// ---------- 创建作业 ----------
const createVisible = ref(false)
const formRef = ref<FormInstance>()
const form = reactive({
  name: '',
  type: 'training' as JobType,
  priority: 1 as JobPriority,
  gpus: 1,
  cpus: undefined as number | undefined,
  memory: undefined as number | undefined,
  description: '',
  partition_id: undefined as number | undefined,
  scheduler_type: undefined as string | undefined,
  gpu_fraction: undefined as number | undefined,
  gpu_memory_gb: undefined as number | undefined,
  gpu_vendor: undefined as GPUVendor | undefined,
  qos: '',
  nodes_requested: undefined as number | undefined,
  node_selector: '',
  affinity: '',
  tolerations: '',
  elastic_enabled: false,
  min_gpus: undefined as number | undefined,
  max_gpus: undefined as number | undefined,
  scaling_policy: '',
  checkpoint_enabled: false,
  checkpoint_interval_minutes: undefined as number | undefined,
  max_retries: undefined as number | undefined,
  fault_tolerance_level: undefined as string | undefined,
  topology_affinity: undefined as string | undefined,
  topology_anti_affinity: false,
  network_requirement: undefined as string | undefined,
})
const rules: FormRules = {
  name: [{ required: true, message: '请输入作业名称', trigger: 'blur' }],
  type: [{ required: true, message: '请选择作业类型', trigger: 'change' }],
  gpus: [{ required: true, message: '请输入 GPU 数量', trigger: 'blur' }],
}

function openCreate() {
  createVisible.value = true
  void loadPartitions()
}
function resetForm() {
  formRef.value?.resetFields()
}
async function submitCreate() {
  if (!formRef.value) return
  await formRef.value.validate()
  try {
    await createMut.mutate({ ...form } as Partial<Job>)
    ElMessage.success('作业创建成功')
    createVisible.value = false
    loadJobs()
  } catch {
    ElMessage.error('作业创建失败，请检查必填项后重试')
  }
}

// ---------- 取消 / 提交 K8S ----------
async function handleCancel(row: Job) {
  try {
    await ElMessageBox.confirm('作业将立即终止且不可恢复。', '取消该作业？', {
      confirmButtonText: '确认取消',
      cancelButtonText: '返回',
      type: 'warning',
    })
  } catch {
    return
  }
  try {
    await cancelMut.mutate(row.id)
    ElMessage.success('作业已取消')
    loadJobs()
  } catch {
    ElMessage.error('作业取消失败，请稍后重试')
  }
}
async function handleSubmitToK8S(row: Job) {
  try {
    await submitMut.mutate(row.id)
    ElMessage.success('作业已提交到K8S')
    loadJobs()
  } catch {
    ElMessage.error('提交作业到K8S失败')
  }
}

// ---------- 详情 + Checkpoint ----------
const detailVisible = ref(false)
const detailJob = ref<Job | null>(null)
const checkpoints = ref<Checkpoint[]>([])

async function openDetail(row: Job) {
  detailJob.value = row
  detailVisible.value = true
  checkpoints.value = []
  try {
    checkpoints.value = await checkpointApi.list({ job_id: row.id })
  } catch {
    ElMessage.warning('Checkpoint 加载失败')
  }
}
</script>
