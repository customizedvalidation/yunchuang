package models

import (
	"time"

	"gorm.io/gorm"
)

// Resource 资源模型
type Resource struct {
	ID          uint           `gorm:"primaryKey" json:"id"`
	CreatedAt   time.Time      `json:"created_at"`
	UpdatedAt   time.Time      `json:"updated_at"`
	DeletedAt   gorm.DeletedAt `gorm:"index" json:"-"`
	ClusterID   uint           `json:"cluster_id"`
	Cluster     *Cluster       `gorm:"foreignKey:ClusterID" json:"cluster,omitempty"`
	Type        string         `gorm:"size:50;not null" json:"type"` // gpu, cpu, memory, storage, network
	Name        string         `gorm:"size:255;not null" json:"name"`
	Status      string         `gorm:"size:50;not null;default:'available'" json:"status"`
	Total       int            `json:"total"`
	Used        int            `json:"used"`
	Available   int            `json:"available"`
	Utilization float64        `json:"utilization"`
	Details     string         `gorm:"size:1000" json:"details"`
}