package models

import (
	"time"

	"gorm.io/gorm"
)

// Partition 分区管理模型
type Partition struct {
	ID                uint           `gorm:"primaryKey" json:"id"`
	CreatedAt         time.Time      `json:"created_at"`
	UpdatedAt         time.Time      `json:"updated_at"`
	DeletedAt         gorm.DeletedAt `gorm:"index" json:"-"`
	ClusterID         uint           `gorm:"index;not null" json:"cluster_id"`
	Cluster           *Cluster       `gorm:"foreignKey:ClusterID" json:"cluster,omitempty"`
	Name              string         `gorm:"size:100;not null" json:"name"`
	Description       string         `gorm:"size:1000" json:"description"`
	Priority          int            `gorm:"default:0" json:"priority"`             // 调度优先级
	MaxRuntimeMinutes int            `gorm:"default:0" json:"max_runtime_minutes"` // 0 = unlimited
	NodeCount         int            `gorm:"default:0" json:"node_count"`
	CPULimit          int            `gorm:"default:0" json:"cpu_limit"` // 0 = unlimited
	GPUCount          int            `gorm:"default:0" json:"gpu_count"`
	GPUVendor         string         `gorm:"size:50" json:"gpu_vendor"` // 该分区主 GPU 厂商
	Status            string         `gorm:"size:50;default:'active'" json:"status"` // active, inactive, maintenance, drained
	AllowSharing      bool           `gorm:"default:false" json:"allow_sharing"`
	DefaultQoS        string         `gorm:"size:50" json:"default_qos"`
	SchedulerType     string         `gorm:"size:50;default:'k8s_native'" json:"scheduler_type"` // k8s_native, slurm, lsf, sge
	Nodes             string         `gorm:"size:2000" json:"nodes"`                              // JSON array of node names
	Taints            string         `gorm:"size:1000" json:"taints"`                             // JSON array of taints
	Labels            string         `gorm:"size:1000" json:"labels"`                             // JSON map of labels
}
