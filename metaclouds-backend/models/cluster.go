package models

import (
	"time"

	"gorm.io/gorm"
)

// Cluster 集群模型
type Cluster struct {
	ID          uint           `gorm:"primaryKey" json:"id"`
	CreatedAt   time.Time      `json:"created_at"`
	UpdatedAt   time.Time      `json:"updated_at"`
	DeletedAt   gorm.DeletedAt `gorm:"index" json:"-"`
	Name        string         `gorm:"uniqueIndex;size:255;not null" json:"name"`
	Description string         `gorm:"size:1000" json:"description"`
	Status      string         `gorm:"size:50;not null;default:'active'" json:"status"`
	Nodes       int            `json:"nodes"`
	GPUs        int            `json:"gpus"`
	CPUs        int            `json:"cpus"`
	Memory      int            `json:"memory"` // 内存（GB）
	Storage     int            `json:"storage"` // 存储（TB）
	NetworkType string         `gorm:"size:50" json:"network_type"`
	Location    string         `gorm:"size:255" json:"location"`
	Resources   []Resource     `gorm:"foreignKey:ClusterID" json:"resources,omitempty"`
	Jobs        []Job          `gorm:"foreignKey:ClusterID" json:"jobs,omitempty"`

	// P0 多 GPU 厂商与多集群新增字段
	GPUVendors          string `gorm:"size:500" json:"gpu_vendors"`       // JSON array of vendors
	SchedulerTypes      string `gorm:"size:500" json:"scheduler_types"`   // JSON array
	MultiClusterEnabled bool   `gorm:"default:false" json:"multi_cluster_enabled"`
	FederationID        string `gorm:"size:100" json:"federation_id"`
}
