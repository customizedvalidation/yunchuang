# Metaclouds 与 skill.md 规范差距分析报告

> 生成日期：2026-09-09
> 规范来源：`docs/archive/root-md-2026-09-04/skill.md`（13 章，801 行）
> 代码基线：commit cfc481f（a11y）

## 一、差距总览

| 优先级 | 数量 | 核心领域 |
|--------|------|----------|
| P0（核心差距） | 5 项 | GPU 细粒度管理、多 GPU 厂商、Slurm/LSF/SGE 集成、分区管理、多维度配额 |
| P1（重要差距） | 7 项 | 拓扑感知、亲和调度、弹性训练、容错训练、Fluid 数据加速、NCCL/DeepSpeed、TensorRT/ONNX |
| P2（增强差距） | 6 项 | 多集群统一调度、SSH/VNC 访问、存储管理、资源共享迁移、RDMA 管理、集合通信优化 |

## 二、P0 核心差距详解

### P0-1 GPU 细粒度分配与显存管理

**skill.md 要求（§4.2）：**
- GPU 分配粒度：1 GPU、1/2 GPU、1/4 GPU、N GPU
- 显存隔离 + 显存超发（oversubscription）
- 编解码实例隔离
- 用户态/内核态虚拟化隔离引擎

**当前实现：**
- `models/resource.go`：仅 `Total/Used/Available int` 整数级资源计数，无分数 GPU 概念
- `models/job.go`：`GPUs int` 仅支持整数 GPU 请求
- 无显存（VRAM）独立字段，无超发配置
- 无 MIG（Multi-Instance GPU）/vGPU 切分模型

**差距等级：P0**
**实施建议：**
1. 新增 `GPUDevice` 模型：`vendor`、`model`、`total_memory_gb`、`allocatable_memory_gb`、`mig_profiles`、`status`
2. 新增 `GPUAllocation` 模型：`device_id`、`fraction`（1.0/0.5/0.25）、`memory_gb`、`job_id`、`tenant_id`
3. Job 模型新增 `gpu_fraction float64`、`gpu_memory_gb int`、`gpu_vendor string`
4. Resource 模型新增 `vram_total_mb`、`vram_used_mb`、`vram_oversubscription_ratio float64`
5. 数据库迁移 `000004_gpu_fine_grained.up.sql`

### P0-2 多 GPU 厂商支持

**skill.md 要求（§4.1, §2.3）：**
- NVIDIA（CUDA/cuDNN）
- 燧原（Enflame，国产 AI 加速库）
- 摩尔线程（Moore Threads）
- 国产 X（其他国产 GPU）
- 兼容性：支持多种 GPU 类型和厂商

**当前实现：**
- `models/resource.go`：无 `vendor` 字段，Type 仅 `gpu/cpu/memory/storage/network`
- `models/cluster.go`：`NetworkType` 存在但无 GPU 厂商聚合信息
- 前端 `GPUResource.type` 仅为卡型（A100/V100），无厂商维度
- K8s 清单无多厂商节点选择器/污点示例

**差距等级：P0**
**实施建议：**
1. Resource/Cluster 模型新增 `gpu_vendor string`（nvidia/enflame/moore_threads/domestic_x）
2. 新增 `VendorDriverConfig` 模型：厂商驱动版本、CUDA 版本、加速库版本
3. 前端资源管理页面增加厂商筛选和厂商维度统计
4. K8s 清单增加多厂商节点 `nodeSelector`/`tolerations` 示例 overlay

### P0-3 Slurm/LSF/SGE 作业调度系统集成

**skill.md 要求（§4.3, §3.2）：**
- 支持与 Slurm 23.02+、LSF 10.2+、SGE 8.1.9+ 集成
- 提供 WEB 化作业界面和图形化工具应用界面
- 作业调度系统作为核心组件

**当前实现：**
- `models/job.go`：自有 Job 模型，状态机 pending/running/completed/failed/cancelled
- `services/scheduler.go` + `pkg/priorityscheduler/`：自研优先级调度器，无外部调度器适配层
- 无 `SchedulerIntegration` 配置模型
- 无作业提交到 Slurm 的命令生成/状态同步逻辑
- 前端 JobManagement 页面无调度器类型选择

**差距等级：P0**
**实施建议：**
1. 新增 `SchedulerIntegration` 模型：`type`（slurm/lsf/sge/k8s_native）、`endpoint`、`auth_config`、`status`、`version`
2. 新增 `SchedulerAdapter` 接口：`SubmitJob()`、`CancelJob()`、`GetJobStatus()`、`GetQueueInfo()`、`GetNodeInfo()`
3. 各调度器实现适配器（Slurm 适配器优先，含 `sbatch`/`squeue`/`scancel` 命令抽象）
4. Job 模型新增 `scheduler_type`、`scheduler_job_id`、`partition`、`qos`、`nodes_requested`
5. 前端作业提交表单增加调度器选择、分区选择、QoS 配置

### P0-4 分区管理

**skill.md 要求（§4.3）：**
- 管理员支持配置管理分区（增删分区）
- 调整分区优先级
- 修改分区的作业最大运行时长
- 分区与用户/用户组权限分配
- 分区间计算资源共享、分区可用 CPU 限制
- 资源跨分区分级别迁移

**当前实现：**
- 完全缺失：无 `Partition` 模型、无分区控制器、无前端页面
- Cluster 模型直接管理节点，无分区中间层
- Job 模型无 `partition_id` 字段

**差距等级：P0**
**实施建议：**
1. 新增 `Partition` 模型：`cluster_id`、`name`、`description`、`priority`、`max_runtime_minutes`、`node_count`、`cpu_limit`、`gpu_count`、`status`、`allow_sharing`
2. 新增 `PartitionPermission` 模型：`partition_id`、`user_id`/`group_id`、`access_level`（view/submit/admin）
3. 新增 `PartitionResourceShare` 模型：源分区、目标分区、共享 CPU/GPU 数量、迁移级别
4. Job 模型新增 `partition_id`
5. 前端新增 `PartitionManagement.tsx` 页面：分区 CRUD、优先级调整、运行时长配置、权限分配、资源共享配置

### P0-5 多维度资源配额

**skill.md 要求（§4.3）：**
- 支持从用户、节点、和分区等多种维度进行资源配额管理
- ResourceQuota 按租户 GPU 配额（team-infra: 2 GPU, team-data: 4 GPU, team-algorithm: 8 GPU）
- LimitRange 限制单 Pod 资源范围

**当前实现：**
- `models/tenant.go`：仅租户级 `GPUQuota/CPUQuota/MemoryQuota/StorageQuota`
- 无用户级配额（User 模型无配额字段）
- 无节点级配额（Resource 模型无 per-node 配额）
- 无分区级配额（Partition 模型尚不存在）
- K8s 清单无 ResourceQuota/LimitRange 示例

**差距等级：P0**
**实施建议：**
1. 新增 `ResourceQuota` 统一模型：`scope_type`（tenant/user/partition/node）、`scope_id`、`resource_type`（gpu/cpu/memory/storage）、`limit`、`used`、`gpu_fraction_limit`
2. User 模型关联配额记录
3. Partition 模型包含配额字段
4. 新增配额校验中间件：作业提交时检查 scope 维度剩余配额
5. K8s 清单新增 `12-resourcequota.yaml`、`13-limitrange.yaml`
6. 前端租户/用户/分区管理页面增加配额配置 Tab

## 三、P1 重要差距详解

### P1-1 拓扑感知调度

**skill.md 要求（§4.3）：**
- 根据硬件拓扑结构优化任务分配，减少网络通信开销
- 超大规模集群组网：高速通信、低延时高吞吐

**当前实现：**
- `pkg/priorityscheduler/`：仅按优先级 FIFO 调度，无拓扑感知
- 无节点拓扑模型（NUMA、GPU NVLink、机架、网络交换机层级）
- Job 无拓扑偏好配置

**实施建议：**
1. 新增 `NodeTopology` 模型：`node_id`、`rack_id`、`switch_id`、`numa_nodes`、`gpu_topology`（NVLink 连接矩阵）
2. 新增 `TopologyPreference` 嵌入 Job：`affinity_level`（node/rack/switch/cluster）、`anti_affinity`、`network_requirement`（rdma/ethernet）
3. 调度器增加拓扑评分插件：计算通信距离，优先同 NVLink/同机架/同交换机
4. 前端作业提交增加拓扑偏好配置

### P1-2 亲和调度

**skill.md 要求（§4.3）：**
- 根据任务特性和资源特性进行优化调度，提高资源利用率
- Taints/Tolerations 实现 Pod 到特定 GPU 节点调度

**当前实现：**
- 无亲和/反亲和配置模型
- K8s 清单无 Taints/Tolerations 示例
- Job 无节点选择器/亲和性字段

**实施建议：**
1. Job 模型新增 `node_selector`（JSON map）、`affinity`（JSON）、`tolerations`（JSON）
2. 新增 `SchedulingProfile` 模型：预定义调度策略（GPU 亲和、数据局部性、负载均衡）
3. 调度器集成亲和性评分
4. K8s 清单增加 Taints/Tolerations 节点池示例

### P1-3 弹性训练

**skill.md 要求（§4.3）：**
- 支持训练过程中的资源动态调整，适应任务需求变化

**当前实现：**
- Job 模型 `GPUs/CPUs` 在创建后固定，无动态调整接口
- 无训练弹性配置（最小/最大副本数、扩缩容策略）
- 无 checkpoint 协调机制

**实施建议：**
1. Job 模型新增 `elastic_enabled bool`、`min_gpus int`、`max_gpus int`、`scaling_policy`、`checkpoint_path`
2. 新增 `JobElasticEvent` 模型：记录扩缩容事件（时间、原 GPU 数、新 GPU 数、原因）
3. 作业控制器新增 `PATCH /jobs/:id/scale` 端点
4. 前端作业详情增加弹性配置和扩缩容历史

### P1-4 容错训练

**skill.md 要求（§4.3）：**
- 在节点故障时保证训练任务的持续运行，提高系统可靠性

**当前实现：**
- Job 失败后无自动重试/恢复机制
- 无 checkpoint 管理模型
- 无故障域感知调度

**实施建议：**
1. Job 模型新增 `max_retries int`、`retry_count int`、`checkpoint_enabled bool`、`checkpoint_interval_minutes int`、`fault_tolerance_level`
2. 新增 `Checkpoint` 模型：`job_id`、`path`、`step`、`created_at`、`size_mb`
3. 调度器增加故障域感知：避免同一作业的 Pod 分布在同一故障域
4. 作业服务增加失败自动重试逻辑（从最新 checkpoint 恢复）

### P1-5 Fluid 数据加速集成

**skill.md 要求（§4.4）：**
- Fluid 分布式缓存系统，训练效率提升 5-10 倍
- 元数据加速、数据预取、数据压缩

**当前实现：**
- `models/acceleration_suite.go`：通用加速套件模型，Type 仅 data/training/inference
- 无 Fluid 特定配置（Dataset、Runtime、缓存配置、预取策略）
- 无数据集管理模型

**实施建议：**
1. 新增 `Dataset` 模型：`name`、`source`（ceph/nfs/s3/glusterfs）、`path`、`size_gb`、`mount_options`
2. 新增 `FluidCache` 模型：`dataset_id`、`runtime_type`（alluxio/jindofs）、`cache_capacity_gb`、`replicas`、`medium_type`（memory/disk）、`prefetch_enabled`、`compression_enabled`、`metadata_acceleration_enabled`
3. AccelerationSuite 模型新增 `config_json` 字段存储 Fluid/NCCL/TensorRT 特定配置
4. 前端加速套件页面增加 Fluid 缓存配置表单、数据集管理

### P1-6 NCCL/DeepSpeed 分布式训练加速

**skill.md 要求（§4.4）：**
- 梯度通信、通信策略、高性能通信库 NCCL（吞吐提升 50%-391%）
- 模型并行、流水线并行、DeepSpeed/Megatron-LM

**当前实现：**
- 无分布式训练配置模型
- Job 无并行策略字段（数据并行/模型并行/流水线并行/张量并行）
- 无 NCCL 配置（NCCL_DEBUG、NCCL_IB_DISABLE、NCCL_SOCKET_IFNAME 等）

**实施建议：**
1. 新增 `DistributedTrainingConfig` 模型：`job_id`、`parallel_strategy`（data/model/pipeline/tensor/hybrid）、`world_size`、`tensor_parallel_size`、`pipeline_stages`、`deepspeed_enabled`、`deepspeed_config_path`、`megatron_enabled`、`nccl_config_json`
2. Job 模型关联分布式训练配置
3. 前端作业提交增加分布式训练配置面板（并行策略、NCCL 参数、DeepSpeed 配置）

### P1-7 TensorRT/ONNX 推理加速

**skill.md 要求（§4.4）：**
- 框架加速、后端引擎 TensorRT/ONNX Runtime
- 高性能算子库（时延降低 40%-90%）
- 模型量化、批处理优化

**当前实现：**
- 无推理服务配置模型
- Job Type 有 `inference` 但无推理特定配置
- 无模型量化/批处理配置

**实施建议：**
1. 新增 `InferenceConfig` 模型：`job_id`、`backend`（tensorrt/onnx_runtime/pytorch/tensorflow）、`precision`（fp32/fp16/int8/int4）、`quantization_enabled`、`calibration_dataset`、`batch_size`、`dynamic_batching`、`max_batch_size`、`model_path`、`tensorrt_engine_path`
2. Job 模型关联推理配置
3. 前端推理作业提交增加推理加速配置面板

## 四、P2 增强差距

| 编号 | 差距项 | skill.md 章节 | 当前状态 | 实施建议 |
|------|--------|---------------|----------|----------|
| P2-1 | 多集群统一调度 | §4.3 | Cluster 模型存在，无跨集群调度 | 新增 `ClusterFederation` 模型、跨集群作业路由 |
| P2-2 | SSH/VNC/WEB 多方式访问 | §4.3 | 无访问方式管理 | 新增 `AccessSession` 模型、Web Terminal/VNC 代理接口 |
| P2-3 | 存储管理（swap 创建） | §4.3 | 无存储管理接口 | 新增 `StorageVolume` 模型、swap 配置管理 |
| P2-4 | 资源共享与迁移 | §4.3 | 无分区资源共享 | 随 P0-4 分区管理一并实现 `PartitionResourceShare` |
| P2-5 | RDMA 网络管理 | §4.1 | Cluster.NetworkType 存在，无详细配置 | 新增 `NetworkConfig` 模型（RoCE/IB、MTU、PFC、ECN） |
| P2-6 | 集合通信效率优化 | §4.3 | 无通信优化配置 | 新增 `CommunicationOptimization` 配置（RDMA、通信压缩、拓扑优化） |

## 五、已对齐项（无需改动）

| 领域 | skill.md 要求 | 当前实现状态 |
|------|---------------|-------------|
| 容器编排 | Kubernetes 1.28+ | ✅ K8s 生产清单 16 yaml，Deployment+HPA+PDB+NetworkPolicy |
| 多租户隔离 | Namespaces + ResourceQuota | ✅ Tenant 模型 + RBAC fail-closed |
| 监控系统 | Prometheus + Grafana | ✅ 13 业务指标 + 16 告警规则 + trace_id 结构化日志 |
| 安全 | RBAC + TLS + 数据加密 | ✅ httpOnly Cookie + CSRF 双提交 + RBAC fail-closed |
| 可观测性 | 系统/资源/任务监控 | ✅ MonitoringController + 告警管理 |
| CI/CD | 自动化构建测试部署 | ✅ 10 Job CI/CD pipeline |
| 前端技术栈 | React + TypeScript | ✅ React 18 + TS + antd 5 + RTK Query |
| 后端技术栈 | Go 1.20+ | ✅ Go + Gin + GORM + PostgreSQL |
| OpenAPI | API 文档 | ✅ 28 路径/44 方法，Swagger 100% 对齐 |

## 六、实施路线图

### 第一阶段：P0 核心能力（本次实施）
1. 数据模型层：GPUDevice、GPUAllocation、Partition、ResourceQuota、SchedulerIntegration、VendorDriverConfig
2. 数据库迁移：000004 新增表 + 000005 索引
3. 后端服务：GPU 细粒度分配服务、分区管理服务、配额校验服务、调度器适配器接口
4. 后端控制器：GPU 管理、分区管理、配额管理、调度器集成 CRUD
5. 前端页面：分区管理页、GPU 细粒度管理组件、配额配置 Tab、调度器选择
6. K8s 清单：ResourceQuota、LimitRange、多厂商节点选择器 overlay

### 第二阶段：P1 高级调度与加速（本次实施）
1. 拓扑感知：NodeTopology 模型 + 调度器拓扑评分插件
2. 亲和调度：Job 亲和性字段 + SchedulingProfile
3. 弹性训练：弹性配置 + 扩缩容接口
4. 容错训练：checkpoint 管理 + 自动重试
5. 加速套件：Dataset + FluidCache + DistributedTrainingConfig + InferenceConfig

### 第三阶段：P2 增强（后续迭代，本次仅预留接口）
- 多集群联邦、SSH/VNC 访问、存储管理、RDMA 配置

## 七、约束与边界

1. **安全不可降级**：所有新接口必须走 JWT 认证 + RBAC 权限校验，新增权限需注册到 authz
2. **技术栈保持**：Go 1.20+ / React 18 + TS / PostgreSQL（GORM）
3. **硬件依赖功能**：实际 RDMA 配置、Fluid 部署、Slurm 集群连接需要真实硬件环境，本次实现接口抽象 + 配置管理 + 前端管理界面，文档中说明集成方式
4. **向后兼容**：新增字段使用默认值，现有 API 行为不变，迁移脚本使用 IF NOT EXISTS
5. **构建门禁**：go build/vet/test + tsc + vite build 必须全量通过
