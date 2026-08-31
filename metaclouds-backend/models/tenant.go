package models

import (
	"time"

	"gorm.io/gorm"
)

// Tenant 租户模型
type Tenant struct {
	ID          uint           `gorm:"primaryKey" json:"id"`
	CreatedAt   time.Time      `json:"created_at"`
	UpdatedAt   time.Time      `json:"updated_at"`
	DeletedAt   gorm.DeletedAt `gorm:"index" json:"-"`
	Name        string         `gorm:"uniqueIndex;size:255;not null" json:"name"`
	Description string         `gorm:"size:1000" json:"description"`
	Status      string         `gorm:"size:50;not null;default:'active'" json:"status"`
	GPUQuota    int            `json:"gpu_quota"`
	CPUQuota    int            `json:"cpu_quota"`
	MemoryQuota int            `json:"memory_quota"` // 内存（GB）
	StorageQuota int           `json:"storage_quota"` // 存储（TB）
	Users       []User         `gorm:"foreignKey:TenantID" json:"users,omitempty"`
	Jobs        []Job          `gorm:"foreignKey:TenantID" json:"jobs,omitempty"`
}