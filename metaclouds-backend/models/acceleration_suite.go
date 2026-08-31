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
}