package models

import (
	"time"

	"gorm.io/gorm"
)

// AccelerationSuite 加速套件模型
type AccelerationSuite struct {
	ID          uint           `gorm:"primaryKey" json:"id"`
	CreatedAt   time.Time      `json:"created_at"`
	UpdatedAt   time.Time      `json:"updated_at"`
	DeletedAt   gorm.DeletedAt `gorm:"index" json:"-"`
	Name        string         `gorm:"uniqueIndex;size:255;not null" json:"name"`
	Description string         `gorm:"size:1000" json:"description"`
	Type        string         `gorm:"size:50;not null" json:"type"` // data, training, inference
	Version     string         `gorm:"size:50;not null" json:"version"`
	Status      string         `gorm:"size:50;not null;default:'active'" json:"status"`
	Enabled     bool           `json:"enabled"`
	Details     string         `gorm:"size:1000" json:"details"`

	// P1 加速套件详细配置新增字段
	ConfigJSON string `gorm:"size:4000" json:"config_json"` // 存储 Fluid/NCCL/TensorRT 特定配置
	Category   string `gorm:"size:50" json:"category"`      // fluid_cache, distributed_training, inference, communication
	Vendor     string `gorm:"size:50" json:"vendor"`        // nvidia, enflame, etc.
}
