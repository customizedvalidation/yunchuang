package models

import (
	"time"

	"gorm.io/gorm"
)

// SecurityPolicy 安全策略模型
type SecurityPolicy struct {
	ID          uint           `gorm:"primaryKey" json:"id"`
	CreatedAt   time.Time      `json:"created_at"`
	UpdatedAt   time.Time      `json:"updated_at"`
	DeletedAt   gorm.DeletedAt `gorm:"index" json:"-"`
	Name        string         `gorm:"uniqueIndex;size:255;not null" json:"name"`
	Description string         `gorm:"size:1000" json:"description"`
	Type        string         `gorm:"size:50;not null" json:"type"` // access, network, data, system
	Status      string         `gorm:"size:50;not null;default:'active'" json:"status"`
	Enabled     bool           `json:"enabled"`
	Rules       string         `gorm:"type:text" json:"rules"` // JSON格式的规则
	Details     string         `gorm:"size:1000" json:"details"`
}