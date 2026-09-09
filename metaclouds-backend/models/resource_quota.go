package models

import (
	"time"

	"gorm.io/gorm"
)

// ResourceQuota 多维度资源配额模型
type ResourceQuota struct {
	ID                uint           `gorm:"primaryKey" json:"id"`
	CreatedAt         time.Time      `json:"created_at"`
	UpdatedAt         time.Time      `json:"updated_at"`
	DeletedAt         gorm.DeletedAt `gorm:"index" json:"-"`
	ScopeType         string         `gorm:"size:20;not null;index" json:"scope_type"` // tenant, user, partition, node
	ScopeID           uint           `gorm:"not null;index" json:"scope_id"`
	ResourceType      string         `gorm:"size:20;not null" json:"resource_type"` // gpu, cpu, memory, storage
	Limit             int            `gorm:"not null;default:0" json:"limit"`
	Used              int            `gorm:"not null;default:0" json:"used"`
	GPUFractionLimit  float64        `gorm:"default:0" json:"gpu_fraction_limit"` // 用于细粒度 GPU 配额
	GPUMemoryLimitGB  int            `gorm:"default:0" json:"gpu_memory_limit_gb"`
	GPUMemoryUsedGB   int            `gorm:"default:0" json:"gpu_memory_used_gb"`
	MaxPodCPU         int            `gorm:"default:0" json:"max_pod_cpu"` // LimitRange: 单 Pod 最大 CPU
	MinPodCPU         int            `gorm:"default:0" json:"min_pod_cpu"`
	MaxPodMemory      int            `gorm:"default:0" json:"max_pod_memory"` // GB
	MinPodMemory      int            `gorm:"default:0" json:"min_pod_memory"`
	MaxPodGPU         float64        `gorm:"default:0" json:"max_pod_gpu"`
	MinPodGPU         float64        `gorm:"default:0" json:"min_pod_gpu"`
	Status            string         `gorm:"size:20;default:'active'" json:"status"`
}
