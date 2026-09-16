<template>
  <div class="mc-page">
    <!-- 页头 -->
    <div class="mc-page-head">
      <div class="mc-page-head-main">
        <h1 class="mc-page-title">K8S 管理</h1>
        <p class="mc-page-desc">GPU 节点、运行 Pod 与服务总览</p>
      </div>
    </div>

    <!-- Tab 由 route.path 决定 -->
    <el-tabs :model-value="activeTab" @tab-change="onTabChange">
      <el-tab-pane label="节点管理" name="nodes" />
      <el-tab-pane label="Pod管理" name="pods" />
      <el-tab-pane label="服务管理" name="services" />
    </el-tabs>

    <!-- ============ 节点管理 ============ -->
    <el-card v-if="activeTab === 'nodes'" shadow="never">
      <div class="mc-flow mc-mb">
        <el-input
          v-model="nodeSearch"
          placeholder="搜索节点名称"
          clearable
          style="width: 240px"
          aria-label="搜索节点"
        />
        <el-button type="primary" :icon="Refresh" :loading="gpuLoading" @click="loadGpu">刷新GPU资源</el-button>
      </div>
      <PageState
        :loading="gpuLoading"
        :error="gpuError"
        :data="pagedNodes"
        empty-text="当前集群没有登记 GPU 节点资源"
        @retry="loadGpu"
      >
        <el-table :data="pagedNodes" stripe>
          <el-table-column label="名称" min-width="160">
            <template #default="{ row }">
              <el-button type="primary" link @click="nodeName = row.gpuName">{{ row.gpuName }}</el-button>
            </template>
          </el-table-column>
          <el-table-column label="状态" width="110">
            <template #default="{ row }">
              <span class="mc-status" :class="statusClass(row.status)">
                <i class="mc-status-dot" />{{ statusText(row.status) }}
              </span>
            </template>
          </el-table-column>
          <el-table-column label="CPU请求" width="100">
            <template #default>—</template>
          </el-table-column>
          <el-table-column label="CPU限制" width="100">
            <template #default>—</template>
          </el-table-column>
          <el-table-column label="内存请求" width="100">
            <template #default>—</template>
          </el-table-column>
          <el-table-column label="内存限制" width="100">
            <template #default>—</template>
          </el-table-column>
          <el-table-column label="GPU" width="120">
            <template #default="{ row }">
              <span class="mc-num">{{ row.used ?? 0 }} / {{ row.total ?? 0 }}</span>
            </template>
          </el-table-column>
          <el-table-column label="标签" min-width="120">
            <template #default>—</template>
          </el-table-column>
          <el-table-column label="操作" width="100" fixed="right">
            <template #default="{ row }">
              <el-button type="primary" link size="small" @click="showNodeDetail(row)">详情</el-button>
            </template>
          </el-table-column>
        </el-table>
        <el-pagination
          class="mc-mt"
          layout="total, prev, pager, next"
          :total="filteredNodes.length"
          v-model:current-page="nodePage.current"
          v-model:page-size="nodePage.size"
          background
        />
      </PageState>
    </el-card>

    <!-- ============ Pod 管理 ============ -->
    <el-card v-else-if="activeTab === 'pods'" shadow="never">
      <div class="mc-flow mc-mb">
        <el-input
          v-model="podSearch"
          placeholder="搜索 Pod 名称"
          clearable
          style="width: 240px"
          aria-label="搜索 Pod"
        />
        <el-button type="primary" :icon="Refresh" :loading="jobsLoading" @click="loadJobs">刷新</el-button>
      </div>
      <PageState
        :loading="jobsLoading"
        :error="jobsError"
        :data="pagedPods"
        empty-text="暂无运行中的 Pod：提交并调度作业后，运行中的工作负载会出现在这里"
        @retry="loadJobs"
      >
        <el-table :data="pagedPods" stripe>
          <el-table-column label="名称" min-width="180">
            <template #default="{ row }">{{ row.name }}</template>
          </el-table-column>
          <el-table-column label="命名空间" width="130">
            <template #default>—</template>
          </el-table-column>
          <el-table-column label="状态" width="110">
            <template #default="{ row }">
              <span class="mc-status" :class="statusClass(row.status)">
                <i class="mc-status-dot" />{{ statusText(row.status) }}
              </span>
            </template>
          </el-table-column>
          <el-table-column label="节点" width="140">
            <template #default>—</template>
          </el-table-column>
          <el-table-column label="CPU" width="90">
            <template #default="{ row }"><span class="mc-num">{{ row.cpus ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="内存(GB)" width="100">
            <template #default="{ row }"><span class="mc-num">{{ row.memory ?? '-' }}</span></template>
          </el-table-column>
          <el-table-column label="重启次数" width="90">
            <template #default>0</template>
          </el-table-column>
          <el-table-column label="创建时间" width="170">
            <template #default="{ row }">{{ row.created_at || '-' }}</template>
          </el-table-column>
        </el-table>
        <el-pagination
          class="mc-mt"
          layout="total, prev, pager, next"
          :total="filteredPods.length"
          v-model:current-page="podPage.current"
          v-model:page-size="podPage.size"
          background
        />
      </PageState>
    </el-card>

    <!-- ============ 服务管理 ============ -->
    <el-card v-else shadow="never">
      <div class="mc-flow mc-mb">
        <el-input
          v-model="svcSearch"
          placeholder="搜索服务名称"
          clearable
          style="width: 240px"
          aria-label="搜索服务"
        />
        <el-button type="primary" :icon="Refresh" :loading="svcLoading" @click="loadSvc">刷新</el-button>
      </div>
      <PageState
        :loading="svcLoading"
        :error="svcError"
        :data="pagedSvcs"
        empty-text="暂无服务：当前命名空间下还没有可调度的服务资源"
        @retry="loadSvc"
      >
        <el-table :data="pagedSvcs" stripe>
          <el-table-column label="名称" min-width="160">
            <template #default="{ row }">{{ row.name }}</template>
          </el-table-column>
          <el-table-column label="命名空间" width="130">
            <template #default>—</template>
          </el-table-column>
          <el-table-column label="类型" width="110">
            <template #default="{ row }">
              <el-tag>{{ row.type || '-' }}</el-tag>
            </template>
          </el-table-column>
          <el-table-column label="集群IP" width="140">
            <template #default>—</template>
          </el-table-column>
          <el-table-column label="端口" width="100">
            <template #default>—</template>
          </el-table-column>
          <el-table-column label="选择器" min-width="140">
            <template #default>—</template>
          </el-table-column>
          <el-table-column label="创建时间" width="170">
            <template #default="{ row }">{{ row.created_at || '-' }}</template>
          </el-table-column>
        </el-table>
        <el-pagination
          class="mc-mt"
          layout="total, prev, pager, next"
          :total="filteredSvcs.length"
          v-model:current-page="svcPage.current"
          v-model:page-size="svcPage.size"
          background
        />
      </PageState>
    </el-card>

    <!-- 节点详情 Dialog -->
    <el-dialog v-model="nodeDetailVisible" :title="`节点详情 - ${nodeName}`" width="560px">
      <template v-if="nodeDetail">
        <el-descriptions :column="1" border size="small">
          <el-descriptions-item label="名称">{{ nodeDetail.gpuName }}</el-descriptions-item>
          <el-descriptions-item label="类型">{{ nodeDetail.type || '-' }}</el-descriptions-item>
          <el-descriptions-item label="状态">{{ statusText(nodeDetail.status) }}</el-descriptions-item>
          <el-descriptions-item label="GPU 总量">{{ nodeDetail.total ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="GPU 已用">{{ nodeDetail.used ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="GPU 可用">{{ nodeDetail.available ?? '-' }}</el-descriptions-item>
          <el-descriptions-item label="利用率">
            <el-progress :percentage="nodeDetail.utilization ?? 0" :stroke-width="10" />
          </el-descriptions-item>
          <el-descriptions-item label="详情">{{ nodeDetail.details || '-' }}</el-descriptions-item>
        </el-descriptions>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, reactive, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { ElMessage } from 'element-plus'
import { Refresh } from '@element-plus/icons-vue'
import { jobApi, resourceApi } from '@/api'
import type { Job, GPUResource, Resource } from '@/types'
import { useFetch } from '@/utils/useFetch'
import PageState from '@/components/PageState.vue'

const route = useRoute()
const router = useRouter()

// ---------- Tab 由 route.path 决定 ----------
const activeTab = computed(() => {
  const p = route.path
  if (p.startsWith('/k8s/pods')) return 'pods'
  if (p.startsWith('/k8s/services')) return 'services'
  return 'nodes'
})
function onTabChange(name: string) {
  const map: Record<string, string> = {
    nodes: '/k8s/nodes',
    pods: '/k8s/pods',
    services: '/k8s/services',
  }
  router.push(map[name] ?? '/k8s/nodes')
}

// ---------- 数据 ----------
// 节点：GPU 资源
const {
  data: gpuRaw, loading: gpuLoading, error: gpuError, execute: loadGpu,
} = useFetch<GPUResource[]>(() => resourceApi.gpuResources())
const gpuList = computed<GPUResource[]>(() => gpuRaw.value ?? [])

// Pod：运行中的作业派生
const {
  data: jobsRaw, loading: jobsLoading, error: jobsError, execute: loadJobs,
} = useFetch<Job[]>(() => jobApi.list())
const runningJobs = computed<Job[]>(() => (jobsRaw.value ?? []).filter((j) => j.status === 'running'))

// 服务：资源列表
const {
  data: svcRaw, loading: svcLoading, error: svcError, execute: loadSvc,
} = useFetch<Resource[]>(() => resourceApi.list())
const svcList = computed<Resource[]>(() => svcRaw.value ?? [])

// ---------- 搜索 ----------
const nodeSearch = ref('')
const podSearch = ref('')
const svcSearch = ref('')

const filteredNodes = computed(() => {
  const kw = nodeSearch.value.trim().toLowerCase()
  if (!kw) return gpuList.value
  return gpuList.value.filter((n) => n.gpuName.toLowerCase().includes(kw))
})
const filteredPods = computed(() => {
  const kw = podSearch.value.trim().toLowerCase()
  if (!kw) return runningJobs.value
  return runningJobs.value.filter((j) => j.name.toLowerCase().includes(kw))
})
const filteredSvcs = computed(() => {
  const kw = svcSearch.value.trim().toLowerCase()
  if (!kw) return svcList.value
  return svcList.value.filter((s) => s.name.toLowerCase().includes(kw))
})

// ---------- 分页 ----------
const nodePage = reactive({ current: 1, size: 10 })
const podPage = reactive({ current: 1, size: 10 })
const svcPage = reactive({ current: 1, size: 10 })
const pagedNodes = computed(() => {
  const s = (nodePage.current - 1) * nodePage.size
  return filteredNodes.value.slice(s, s + nodePage.size)
})
const pagedPods = computed(() => {
  const s = (podPage.current - 1) * podPage.size
  return filteredPods.value.slice(s, s + podPage.size)
})
const pagedSvcs = computed(() => {
  const s = (svcPage.current - 1) * svcPage.size
  return filteredSvcs.value.slice(s, s + svcPage.size)
})

// ---------- 展示辅助 ----------
function statusClass(s?: string) {
  return ['running', 'pending', 'completed', 'failed', 'cancelled', 'active', 'online'].includes(s ?? '')
    ? (s as string)
    : 'idle'
}
function statusText(s?: string) {
  const map: Record<string, string> = {
    running: '运行中', pending: '排队中', completed: '已完成', failed: '失败', cancelled: '已取消',
    active: '在线', online: '在线', ready: '就绪', available: '可用',
  }
  return map[s ?? ''] ?? s ?? '-'
}

// ---------- 节点详情 ----------
const nodeDetailVisible = ref(false)
const nodeName = ref('')
const nodeDetail = ref<GPUResource | null>(null)
function showNodeDetail(row: GPUResource) {
  nodeDetail.value = row
  nodeName.value = row.gpuName
  nodeDetailVisible.value = true
  ElMessage.success(`查看节点 ${row.gpuName}`)
}
</script>
