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

	// P0 多 GPU 厂商与显存管理新增字段
	Vendor                    string  `gorm:"size:50" json:"vendor"`
	GPUModel                  string  `gorm:"size:100" json:"gpu_model"`
	VRAMTotalMB               int     `gorm:"default:0" json:"vram_total_mb"`
	VRAMUsedMB                int     `gorm:"default:0" json:"vram_used_mb"`
	VRAMOversubscriptionRatio float64 `gorm:"default:1.0" json:"vram_oversubscription_ratio"`
	MIGEnabled                bool    `gorm:"default:false" json:"mig_enabled"`
}
