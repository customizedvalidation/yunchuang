package models

import (
	"time"

	"gorm.io/gorm"
)

// GPUDevice GPU 细粒度设备模型
type GPUDevice struct {
	ID                  uint           `gorm:"primaryKey" json:"id"`
	CreatedAt           time.Time      `json:"created_at"`
	UpdatedAt           time.Time      `json:"updated_at"`
	DeletedAt           gorm.DeletedAt `gorm:"index" json:"-"`
	ClusterID           uint           `gorm:"index" json:"cluster_id"`
	NodeName            string         `gorm:"size:255" json:"node_name"`
	Vendor              string         `gorm:"size:50;not null" json:"vendor"`              // nvidia, enflame, moore_threads, domestic_x
	Model               string         `gorm:"size:100" json:"model"`                       // A100, V100, T4, etc.
	Index               int            `json:"index"`                                       // GPU index on node
	TotalMemoryGB       int            `json:"total_memory_gb"`
	AllocatableMemoryGB int            `json:"allocatable_memory_gb"`
	UsedMemoryGB        int            `json:"used_memory_gb"`
	MIGEnabled          bool           `json:"mig_enabled"`
	MIGProfiles         string         `gorm:"size:500" json:"mig_profiles"` // JSON array of available MIG slices
	DriverVersion       string         `gorm:"size:50" json:"driver_version"`
	CUDAVersion         string         `gorm:"size:50" json:"cuda_version"`
	Status              string         `gorm:"size:50;default:'available'" json:"status"` // available, allocated, maintenance, fault
	Utilization         float64        `json:"utilization"`
	Temperature         int            `json:"temperature"`
	PowerDraw           int            `json:"power_draw"` // watts
	Details             string         `gorm:"size:1000" json:"details"`
}
