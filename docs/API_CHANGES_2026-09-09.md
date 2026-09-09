# Metaclouds API 变更记录（2026-09-09）

> **变更日期**：2026-09-09
> **变更类型**：新增 API 端点（P0/P1 差距实施）
> **关联文档**：[skill.md 差距分析报告](./skill-md-gap-analysis-2026-09-09.md)
> **API 版本**：v1
> **基础路径**：`/api/v1`

---

## 变更总览

本次新增 **7 个 API 分组、36 个端点**，覆盖 GPU 细粒度管理、分区管理、多维度配额、调度器集成、拓扑感知、数据集与 Fluid 缓存、Checkpoint 管理。

| API 分组 | 端点数 | 对应差距 | 优先级 |
|----------|--------|----------|--------|
| GPU 管理 | 6 | P0-1 GPU 细粒度分配 | P0 |
| 分区管理 | 8 | P0-4 分区管理 | P0 |
| 配额管理 | 6 | P0-5 多维度配额 | P0 |
| 调度器集成 | 7 | P0-3 Slurm/LSF/SGE 集成 | P0 |
| 拓扑管理 | 5 | P1-1 拓扑感知调度 | P1 |
| 数据集与缓存 | 9 | P1-5 Fluid 数据加速 | P1 |
| Checkpoint | 4 | P1-4 容错训练 | P1 |

---

## 1. GPU 管理（/gpus）

对应 P0-1：GPU 细粒度分配与显存管理。支持 GPU 设备注册、细粒度分配（1/2、1/4 GPU）、显存管理、利用率监控。

### 1.1 GET /gpus

获取 GPU 设备列表。

- **描述**：分页查询所有 GPU 设备，支持按厂商、型号、状态、节点筛选
- **权限**：认证用户（管理员可见全部，普通用户可见已分配的）
- **请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | int | 否 | 页码，默认 1 |
| page_size | int | 否 | 每页数量，默认 10，最大 100 |
| vendor | string | 否 | 厂商筛选：nvidia/enflame/moore_threads/domestic_x |
| model | string | 否 | 型号筛选：A100/T4/ENflame-T20/MTT-S80 |
| status | string | 否 | 状态筛选：available/allocating/allocated/maintenance/offline |
| node_id | int | 否 | 节点 ID 筛选 |
| partition_id | int | 否 | 分区 ID 筛选 |

- **响应格式**：

```json
{
  "success": true,
  "data": {
    "data": [
      {
        "id": 1,
        "vendor": "nvidia",
        "model": "A100",
        "node_id": 10,
        "total_memory_gb": 40,
        "allocatable_memory_gb": 40,
        "used_memory_gb": 16,
        "mig_profiles": ["1g.5gb", "2g.10gb", "3g.20gb", "7g.40gb"],
        "mig_enabled": true,
        "status": "allocated",
        "utilization_gpu": 75,
        "utilization_memory": 40,
        "temperature": 68,
        "power_watts": 250,
        "driver_version": "535.129.03",
        "cuda_version": "12.2",
        "created_at": "2026-09-01T10:00:00Z",
        "updated_at": "2026-09-09T08:30:00Z"
      }
    ],
    "total": 128,
    "page": 1,
    "page_size": 10,
    "total_pages": 13
  },
  "code": "OK",
  "timestamp": 1757400000
}
```

### 1.2 POST /gpus

注册新的 GPU 设备。

- **描述**：向平台注册 GPU 设备（通常由节点 agent 自动上报，管理员也可手动注册）
- **权限**：`resource_write`
- **请求体**：

```json
{
  "vendor": "nvidia",
  "model": "A100",
  "node_id": 10,
  "total_memory_gb": 40,
  "mig_enabled": true,
  "mig_profiles": ["1g.5gb", "2g.10gb", "3g.20gb", "7g.40gb"],
  "driver_version": "535.129.03",
  "cuda_version": "12.2",
  "pci_bus_id": "0000:01:00.0"
}
```

- **响应格式**：201 Created，返回创建的 GPU 设备对象

### 1.3 PUT /gpus/:id

更新 GPU 设备信息。

- **描述**：更新 GPU 设备状态、MIG 配置、维护状态等
- **权限**：`resource_write`
- **请求体**：

```json
{
  "status": "maintenance",
  "mig_enabled": true,
  "allocatable_memory_gb": 32
}
```

- **响应格式**：200 OK，返回更新后的 GPU 设备对象

### 1.4 DELETE /gpus/:id

删除 GPU 设备。

- **描述**：从平台移除 GPU 设备（设备已物理下线时使用）
- **权限**：`resource_write`
- **注意**：设备处于 allocated 状态时不允许删除，需先释放分配

### 1.5 GET /gpus/allocations

获取 GPU 分配记录列表。

- **描述**：查询 GPU 细粒度分配记录，支持按作业、租户、GPU 设备筛选
- **权限**：认证用户
- **请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| page | int | 否 | 页码 |
| page_size | int | 否 | 每页数量 |
| job_id | int | 否 | 作业 ID |
| tenant_id | int | 否 | 租户 ID |
| gpu_id | int | 否 | GPU 设备 ID |
| fraction | float | 否 | 分配粒度筛选：0.25/0.5/1.0 |
| status | string | 否 | active/released |

- **响应格式**：

```json
{
  "success": true,
  "data": {
    "data": [
      {
        "id": 1,
        "gpu_id": 1,
        "job_id": 100,
        "tenant_id": 3,
        "fraction": 0.5,
        "memory_gb": 20,
        "mig_profile": "2g.10gb",
        "status": "active",
        "allocated_at": "2026-09-09T08:00:00Z",
        "released_at": null
      }
    ],
    "total": 45,
    "page": 1,
    "page_size": 10,
    "total_pages": 5
  },
  "code": "OK"
}
```

### 1.6 POST /gpus/allocations

创建 GPU 分配。

- **描述**：为作业分配 GPU 资源，支持细粒度（0.25/0.5/1.0/N GPU）
- **权限**：`job_write`
- **请求体**：

```json
{
  "job_id": 100,
  "tenant_id": 3,
  "gpu_ids": [1, 2],
  "fraction": 0.5,
  "memory_gb": 20,
  "vendor_preference": "nvidia",
  "model_preference": "A100"
}
```

- **响应格式**：201 Created，返回分配记录

### 1.7 DELETE /gpus/allocations/:id

释放 GPU 分配。

- **描述**：释放指定的 GPU 分配，资源归还到资源池
- **权限**：`job_write`

### 1.8 GET /gpus/utilization

获取 GPU 利用率统计。

- **描述**：获取 GPU 利用率聚合统计，支持按时间范围、厂商、分区聚合
- **权限**：认证用户
- **请求参数**：

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| start_time | string | 否 | 开始时间（ISO 8601） |
| end_time | string | 否 | 结束时间（ISO 8601） |
| granularity | string | 否 | 聚合粒度：1m/5m/1h/1d，默认 5m |
| vendor | string | 否 | 厂商筛选 |
| partition_id | int | 否 | 分区筛选 |

- **响应格式**：

```json
{
  "success": true,
  "data": {
    "summary": {
      "total_gpus": 128,
      "allocated_gpus": 85,
      "avg_utilization": 72.5,
      "avg_memory_utilization": 58.3,
      "oversubscription_ratio": 1.2
    },
    "timeseries": [
      {
        "timestamp": "2026-09-09T08:00:00Z",
        "gpu_utilization": 75.0,
        "memory_utilization": 60.0,
        "power_watts": 28500
      }
    ]
  },
  "code": "OK"
}
```

---

## 2. 分区管理（/partitions）

对应 P0-4：分区管理。支持分区 CRUD、优先级调整、最大运行时长配置、权限分配。

### 2.1 GET /partitions

获取分区列表。

- **描述**：分页查询所有分区，支持按集群、状态筛选
- **权限**：认证用户
- **请求参数**：page, page_size, cluster_id, status, name

### 2.2 POST /partitions

创建分区。

- **描述**：创建新的计算分区
- **权限**：`cluster_write`
- **请求体**：

```json
{
  "cluster_id": 1,
  "name": "algorithm-training",
  "description": "算法训练分区（A100 节点池）",
  "priority": 100,
  "max_runtime_minutes": 10080,
  "node_count": 16,
  "cpu_limit": 512,
  "gpu_count": 128,
  "allow_sharing": true,
  "status": "active"
}
```

### 2.3 GET /partitions/:id

获取分区详情。

- **权限**：认证用户

### 2.4 PUT /partitions/:id

更新分区。

- **权限**：`cluster_write`

### 2.5 DELETE /partitions/:id

删除分区。

- **权限**：`cluster_write`
- **注意**：分区内有运行中作业时不允许删除

### 2.6 PUT /partitions/:id/priority

调整分区优先级。

- **描述**：调整分区的调度优先级，高优先级分区的作业优先调度
- **权限**：`cluster_write`
- **请求体**：

```json
{
  "priority": 200
}
```

### 2.7 PUT /partitions/:id/max-runtime

修改分区最大运行时长。

- **描述**：设置分区内作业的最大运行时长（分钟），超时自动终止
- **权限**：`cluster_write`
- **请求体**：

```json
{
  "max_runtime_minutes": 1440
}
```

### 2.8 GET /partitions/:id/permissions

获取分区权限列表。

- **描述**：查询分区的用户/用户组权限分配
- **权限**：认证用户

### 2.9 POST /partitions/:id/permissions

分配分区权限。

- **描述**：为用户或用户组分配分区访问权限
- **权限**：`cluster_write`
- **请求体**：

```json
{
  "user_id": 5,
  "group_id": null,
  "access_level": "submit"
}
```

- **access_level 取值**：view（查看）/ submit（提交作业）/ admin（分区管理）

### 2.10 DELETE /partitions/:id/permissions/:permId

移除分区权限。

- **权限**：`cluster_write`

---

## 3. 配额管理（/quotas）

对应 P0-5：多维度资源配额。支持租户/用户/分区/节点维度的配额管理。

### 3.1 GET /quotas

获取配额列表。

- **描述**：分页查询资源配额，支持按维度（tenant/user/partition/node）筛选
- **权限**：`tenant_read`（管理员），普通用户仅可见自身配额
- **请求参数**：page, page_size, scope_type, scope_id, resource_type

### 3.2 POST /quotas

创建配额。

- **描述**：为指定维度创建资源配额
- **权限**：`tenant_write`
- **请求体**：

```json
{
  "scope_type": "tenant",
  "scope_id": 3,
  "resource_type": "gpu",
  "limit": 8,
  "gpu_fraction_limit": 8.0,
  "description": "算法团队 GPU 配额"
}
```

- **scope_type 取值**：tenant / user / partition / node
- **resource_type 取值**：gpu / cpu / memory / storage

### 3.3 GET /quotas/:id

获取配额详情。

- **权限**：认证用户

### 3.4 PUT /quotas/:id

更新配额。

- **权限**：`tenant_write`

### 3.5 DELETE /quotas/:id

删除配额。

- **权限**：`tenant_write`

### 3.6 GET /quotas/usage

获取配额使用情况。

- **描述**：查询各维度配额的已用/剩余情况
- **权限**：认证用户
- **请求参数**：scope_type, scope_id

- **响应格式**：

```json
{
  "success": true,
  "data": [
    {
      "scope_type": "tenant",
      "scope_id": 3,
      "resource_type": "gpu",
      "limit": 8,
      "used": 5.5,
      "remaining": 2.5,
      "usage_percent": 68.75
    },
    {
      "scope_type": "tenant",
      "scope_id": 3,
      "resource_type": "cpu",
      "limit": 16,
      "used": 12,
      "remaining": 4,
      "usage_percent": 75.0
    }
  ],
  "code": "OK"
}
```

### 3.7 POST /quotas/check

校验配额是否充足。

- **描述**：作业提交前校验指定维度的配额是否满足资源需求
- **权限**：认证用户
- **请求体**：

```json
{
  "scope_type": "tenant",
  "scope_id": 3,
  "requests": {
    "gpu": 2.5,
    "cpu": 8,
    "memory": "32Gi"
  }
}
```

- **响应格式**：

```json
{
  "success": true,
  "data": {
    "allowed": true,
    "checks": [
      {"resource": "gpu", "requested": 2.5, "remaining": 2.5, "passed": true},
      {"resource": "cpu", "requested": 8, "remaining": 4, "passed": false}
    ],
    "message": "CPU 配额不足：请求 8，剩余 4"
  },
  "code": "OK"
}
```

---

## 4. 调度器集成（/schedulers）

对应 P0-3：Slurm/LSF/SGE 作业调度系统集成。

### 4.1 GET /schedulers

获取调度器集成列表。

- **权限**：认证用户

### 4.2 POST /schedulers

创建调度器集成。

- **权限**：`cluster_write`
- **请求体**：

```json
{
  "type": "slurm",
  "name": "slurm-cluster-01",
  "endpoint": "slurmctl.example.com",
  "auth_config": {
    "type": "ssh",
    "username": "metaclouds",
    "private_key_secret": "slurm-ssh-key"
  },
  "version": "23.02",
  "partitions": [
    {"name": "gpu-a100", "gpu_model": "A100", "max_nodes": 32, "max_gpus_per_node": 8}
  ]
}
```

- **type 取值**：slurm / lsf / sge / k8s_native

### 4.3 GET /schedulers/:id

获取调度器详情。

### 4.4 PUT /schedulers/:id

更新调度器配置。

- **权限**：`cluster_write`

### 4.5 DELETE /schedulers/:id

删除调度器集成。

- **权限**：`cluster_write`

### 4.6 GET /schedulers/:id/queues

获取调度器队列/分区信息。

- **描述**：从外部调度器同步队列（Slurm partition / LSF queue / SGE queue）信息
- **权限**：认证用户

### 4.7 GET /schedulers/:id/nodes

获取调度器节点信息。

- **描述**：从外部调度器同步节点状态信息
- **权限**：认证用户

### 4.8 POST /schedulers/:id/sync

同步调度器状态。

- **描述**：触发与外部调度器的作业状态同步
- **权限**：`cluster_write`

### 4.9 GET /schedulers/:id/health

调度器健康检查。

- **描述**：检查与外部调度器的连通性和版本兼容性
- **权限**：认证用户

---

## 5. 拓扑管理（/topology）

对应 P1-1：拓扑感知调度。管理节点拓扑信息（NUMA、GPU NVLink、机架、交换机层级）。

### 5.1 GET /topology

获取拓扑信息列表。

- **权限**：认证用户
- **请求参数**：node_id, rack_id, switch_id, level

### 5.2 POST /topology

创建/注册拓扑信息。

- **权限**：`resource_write`
- **请求体**：

```json
{
  "node_id": 10,
  "rack_id": "rack-01",
  "switch_id": "switch-01",
  "numa_nodes": [
    {"id": 0, "cpus": "0-31", "memory_gb": 256},
    {"id": 1, "cpus": "32-63", "memory_gb": 256}
  ],
  "gpu_topology": {
    "nvlink_matrix": [[0,1,1,0],[1,0,0,1],[1,0,0,1],[0,1,1,0]],
    "nvlink_connections": ["gpu0-gpu1", "gpu0-gpu2", "gpu1-gpu3", "gpu2-gpu3"]
  }
}
```

### 5.3 GET /topology/:id

获取拓扑详情。

### 5.4 PUT /topology/:id

更新拓扑信息。

- **权限**：`resource_write`

### 5.5 DELETE /topology/:id

删除拓扑信息。

- **权限**：`resource_write`

### 5.6 POST /topology/score

计算拓扑调度评分。

- **描述**：根据作业的拓扑偏好和节点拓扑，计算各候选节点的调度评分
- **权限**：认证用户
- **请求体**：

```json
{
  "job_id": 100,
  "affinity_level": "rack",
  "anti_affinity": true,
  "network_requirement": "rdma",
  "candidate_nodes": [10, 11, 12, 13]
}
```

- **响应格式**：

```json
{
  "success": true,
  "data": {
    "scores": [
      {"node_id": 10, "score": 95, "reasons": ["same_rack", "nvlink_available", "rdma_supported"]},
      {"node_id": 11, "score": 85, "reasons": ["same_rack", "rdma_supported"]},
      {"node_id": 12, "score": 60, "reasons": ["cross_rack"]}
    ]
  },
  "code": "OK"
}
```

---

## 6. 数据集与缓存管理（/datasets）

对应 P1-5：Fluid 数据加速集成。管理数据集和 Fluid 缓存配置。

### 6.1 GET /datasets

获取数据集列表。

- **权限**：认证用户
- **请求参数**：page, page_size, source, status

### 6.2 POST /datasets

创建数据集。

- **权限**：`accel_write`
- **请求体**：

```json
{
  "name": "imagenet-training",
  "source": "ceph",
  "path": "/data/training/imagenet",
  "size_gb": 500,
  "mount_options": {"mon_addr": "ceph-mon:6789"},
  "description": "ImageNet 训练数据集"
}
```

- **source 取值**：ceph / nfs / s3 / glusterfs

### 6.3 GET /datasets/:id

获取数据集详情。

### 6.4 PUT /datasets/:id

更新数据集。

- **权限**：`accel_write`

### 6.5 DELETE /datasets/:id

删除数据集。

- **权限**：`accel_write`

### 6.6 GET /datasets/:id/caches

获取数据集的缓存配置列表。

- **权限**：认证用户

### 6.7 POST /datasets/:id/caches

创建缓存配置。

- **权限**：`accel_write`
- **请求体**：

```json
{
  "dataset_id": 1,
  "runtime_type": "alluxio",
  "cache_capacity_gb": 200,
  "replicas": 3,
  "medium_type": "memory+disk",
  "prefetch_enabled": true,
  "compression_enabled": true,
  "metadata_acceleration_enabled": true
}
```

- **runtime_type 取值**：alluxio / jindofs / goosefs
- **medium_type 取值**：memory / ssd / hdd / memory+disk

### 6.8 PUT /datasets/:id/caches/:cacheId

更新缓存配置。

- **权限**：`accel_write`

### 6.9 DELETE /datasets/:id/caches/:cacheId

删除缓存配置。

- **权限**：`accel_write`

### 6.10 POST /datasets/caches/:cacheId/enable

启用缓存。

- **描述**：启用 Fluid 缓存，创建 AlluxioRuntime
- **权限**：`accel_write`

### 6.11 POST /datasets/caches/:cacheId/disable

停用缓存。

- **描述**：停用 Fluid 缓存，删除 AlluxioRuntime（保留 Dataset）
- **权限**：`accel_write`

### 6.12 POST /datasets/caches/:cacheId/prefetch

触发数据预取。

- **描述**：创建 DataLoad CR，将数据预热到缓存层
- **权限**：`accel_write`
- **请求体**：

```json
{
  "path": "/",
  "recursive": true,
  "concurrency": 16
}
```

---

## 7. Checkpoint 管理（/checkpoints）

对应 P1-4：容错训练。管理训练 checkpoint。

### 7.1 GET /checkpoints

获取 Checkpoint 列表。

- **权限**：认证用户
- **请求参数**：page, page_size, job_id, status

### 7.2 POST /checkpoints

创建 Checkpoint 记录。

- **描述**：记录训练 checkpoint 元数据（通常由训练作业自动上报）
- **权限**：`job_write`
- **请求体**：

```json
{
  "job_id": 100,
  "path": "/checkpoints/job-100/step-5000",
  "step": 5000,
  "epoch": 10,
  "size_mb": 2048,
  "metrics": {"loss": 0.15, "accuracy": 0.92}
}
```

### 7.3 DELETE /checkpoints/:id

删除 Checkpoint。

- **权限**：`job_write`

### 7.4 GET /checkpoints/latest/:jobId

获取作业的最新 Checkpoint。

- **描述**：获取指定作业的最新有效 checkpoint，用于故障恢复
- **权限**：认证用户
- **响应格式**：

```json
{
  "success": true,
  "data": {
    "id": 50,
    "job_id": 100,
    "path": "/checkpoints/job-100/step-5000",
    "step": 5000,
    "epoch": 10,
    "size_mb": 2048,
    "created_at": "2026-09-09T10:30:00Z",
    "validation_status": "valid"
  },
  "code": "OK"
}
```

---

## 权限汇总

本次新增 API 需要注册以下权限到 authz 系统：

| 权限 | 说明 | 关联端点 |
|------|------|----------|
| `gpu_read` | GPU 设备读取 | GET /gpus, GET /gpus/allocations, GET /gpus/utilization |
| `gpu_write` | GPU 设备管理 | POST/PUT/DELETE /gpus, POST/DELETE /gpus/allocations |
| `partition_read` | 分区读取 | GET /partitions, GET /partitions/:id/permissions |
| `partition_write` | 分区管理 | POST/PUT/DELETE /partitions, PUT priority/max-runtime, POST/DELETE permissions |
| `quota_read` | 配额读取 | GET /quotas, GET /quotas/usage |
| `quota_write` | 配额管理 | POST/PUT/DELETE /quotas, POST /quotas/check |
| `scheduler_read` | 调度器读取 | GET /schedulers, GET queues/nodes/health |
| `scheduler_write` | 调度器管理 | POST/PUT/DELETE /schedulers, POST /sync |
| `topology_read` | 拓扑读取 | GET /topology, POST /topology/score |
| `topology_write` | 拓扑管理 | POST/PUT/DELETE /topology |
| `dataset_read` | 数据集读取 | GET /datasets, GET /datasets/:id/caches |
| `dataset_write` | 数据集管理 | POST/PUT/DELETE /datasets, 缓存 CRUD/启停/预取 |
| `checkpoint_read` | Checkpoint 读取 | GET /checkpoints, GET /checkpoints/latest |
| `checkpoint_write` | Checkpoint 管理 | POST/DELETE /checkpoints |

---

## 向后兼容性

- 所有新增端点均为新路径，不影响现有 API
- 现有 API 行为不变
- 新增权限默认不授予普通用户，需管理员显式分配
- 数据库迁移使用 `IF NOT EXISTS`，不影响现有表结构

---

*本文档由 Metaclouds 团队维护，最后更新：2026-09-09*
