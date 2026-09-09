package models

import (
	"time"

	"gorm.io/gorm"
)

// Dataset 数据集模型（Fluid 数据加速）
type Dataset struct {
	ID           uint           `gorm:"primaryKey" json:"id"`
	CreatedAt    time.Time      `json:"created_at"`
	UpdatedAt    time.Time      `json:"updated_at"`
	DeletedAt    gorm.DeletedAt `gorm:"index" json:"-"`
	Name         string         `gorm:"size:255;not null;unique" json:"name"`
	Description  string         `gorm:"size:1000" json:"description"`
	SourceType   string         `gorm:"size:50;not null" json:"source_type"` // ceph, nfs, s3, glusterfs, lustre, beegfs
	SourcePath   string         `gorm:"size:1000" json:"source_path"`
	MountPath    string         `gorm:"size:500" json:"mount_path"`
	SizeGB       int            `gorm:"default:0" json:"size_gb"`
	MountOptions string         `gorm:"size:1000" json:"mount_options"` // JSON
	AccessMode   string         `gorm:"size:20;default:'ReadWriteMany'" json:"access_mode"`
	Status       string         `gorm:"size:20;default:'active'" json:"status"`
	TenantID     uint           `gorm:"index" json:"tenant_id"`
}

// FluidCache Fluid 分布式缓存配置
type FluidCache struct {
	ID                          uint           `gorm:"primaryKey" json:"id"`
	CreatedAt                   time.Time      `json:"created_at"`
	UpdatedAt                   time.Time      `json:"updated_at"`
	DeletedAt                   gorm.DeletedAt `gorm:"index" json:"-"`
	DatasetID                   uint           `gorm:"index;not null" json:"dataset_id"`
	Dataset                     *Dataset       `gorm:"foreignKey:DatasetID" json:"dataset,omitempty"`
	RuntimeType                 string         `gorm:"size:50;default:'alluxio'" json:"runtime_type"` // alluxio, jindofs
	CacheCapacityGB             int            `gorm:"default:0" json:"cache_capacity_gb"`
	Replicas                    int            `gorm:"default:1" json:"replicas"`
	MediumType                  string         `gorm:"size:20;default:'memory'" json:"medium_type"` // memory, disk, mixed
	PrefetchEnabled             bool           `gorm:"default:false" json:"prefetch_enabled"`
	PrefetchPolicy              string         `gorm:"size:500" json:"prefetch_policy"` // JSON
	CompressionEnabled          bool           `gorm:"default:false" json:"compression_enabled"`
	MetadataAccelerationEnabled bool           `gorm:"default:false" json:"metadata_acceleration_enabled"`
	Status                      string         `gorm:"size:20;default:'inactive'" json:"status"`
	CacheHitRate                float64        `json:"cache_hit_rate"`
}

// DistributedTrainingConfig 分布式训练配置
type DistributedTrainingConfig struct {
	ID                   uint      `gorm:"primaryKey" json:"id"`
	CreatedAt            time.Time `json:"created_at"`
	UpdatedAt            time.Time `json:"updated_at"`
	JobID                uint      `gorm:"index;not null" json:"job_id"`
	ParallelStrategy     string    `gorm:"size:30;default:'data'" json:"parallel_strategy"` // data, model, pipeline, tensor, hybrid
	WorldSize            int       `gorm:"default:1" json:"world_size"`
	TensorParallelSize   int       `gorm:"default:1" json:"tensor_parallel_size"`
	PipelineStages       int       `gorm:"default:1" json:"pipeline_stages"`
	DeepSpeedEnabled     bool      `gorm:"default:false" json:"deepspeed_enabled"`
	DeepSpeedConfigPath  string    `gorm:"size:500" json:"deepspeed_config_path"`
	MegatronEnabled      bool      `gorm:"default:false" json:"megatron_enabled"`
	NCCLConfig           string    `gorm:"size:2000" json:"nccl_config"` // JSON: NCCL env vars
	GradientCompression  bool      `gorm:"default:false" json:"gradient_compression"`
	CommunicationBackend string    `gorm:"size:20;default:'nccl'" json:"communication_backend"` // nccl, gloo, mpi
}

// InferenceConfig 推理加速配置
type InferenceConfig struct {
	ID                   uint      `gorm:"primaryKey" json:"id"`
	CreatedAt            time.Time `json:"created_at"`
	UpdatedAt            time.Time `json:"updated_at"`
	JobID                uint      `gorm:"index;not null" json:"job_id"`
	Backend              string    `gorm:"size:30;default:'pytorch'" json:"backend"` // tensorrt, onnx_runtime, pytorch, tensorflow
	Precision            string    `gorm:"size:10;default:'fp32'" json:"precision"` // fp32, fp16, int8, int4
	QuantizationEnabled  bool      `gorm:"default:false" json:"quantization_enabled"`
	CalibrationDataset   string    `gorm:"size:500" json:"calibration_dataset"`
	BatchSize            int       `gorm:"default:1" json:"batch_size"`
	DynamicBatching      bool      `gorm:"default:false" json:"dynamic_batching"`
	MaxBatchSize         int       `gorm:"default:32" json:"max_batch_size"`
	MaxLatencyMs         int       `gorm:"default:0" json:"max_latency_ms"`
	ModelPath            string    `gorm:"size:1000" json:"model_path"`
	TensorRTEnginePath   string    `gorm:"size:500" json:"tensorrt_engine_path"`
	GPUUtilizationTarget float64   `gorm:"default:0.8" json:"gpu_utilization_target"`
}

// Checkpoint 检查点模型
type Checkpoint struct {
	ID        uint           `gorm:"primaryKey" json:"id"`
	CreatedAt time.Time      `json:"created_at"`
	UpdatedAt time.Time      `json:"updated_at"`
	DeletedAt gorm.DeletedAt `gorm:"index" json:"-"`
	JobID     uint           `gorm:"index;not null" json:"job_id"`
	Job       *Job           `gorm:"foreignKey:JobID" json:"job,omitempty"`
	Path      string         `gorm:"size:1000;not null" json:"path"`
	Step      int            `gorm:"default:0" json:"step"`
	Epoch     int            `gorm:"default:0" json:"epoch"`
	SizeMB    int            `gorm:"default:0" json:"size_mb"`
	Format    string         `gorm:"size:50;default:'pytorch'" json:"format"` // pytorch, tensorflow, deepspeed, megatron
	IsLatest  bool           `gorm:"default:false" json:"is_latest"`
	Status    string         `gorm:"size:20;default:'completed'" json:"status"` // in_progress, completed, failed, corrupted
}
