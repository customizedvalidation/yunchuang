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
}