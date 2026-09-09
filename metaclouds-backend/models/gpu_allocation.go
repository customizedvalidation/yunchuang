package models

import (
	"time"

	"gorm.io/gorm"
)

// GPUAllocation GPU 细粒度分配记录
type GPUAllocation struct {
	ID         uint           `gorm:"primaryKey" json:"id"`
	CreatedAt  time.Time      `json:"created_at"`
	UpdatedAt  time.Time      `json:"updated_at"`
	DeletedAt  gorm.DeletedAt `gorm:"index" json:"-"`
	DeviceID   uint           `gorm:"index" json:"device_id"`
	Device     *GPUDevice     `gorm:"foreignKey:DeviceID" json:"device,omitempty"`
	JobID      uint           `gorm:"index" json:"job_id"`
	Job        *Job           `gorm:"foreignKey:JobID" json:"job,omitempty"`
	TenantID   uint           `gorm:"index" json:"tenant_id"`
	UserID     uint           `gorm:"index" json:"user_id"`
	Fraction   float64        `json:"fraction"`   // 1.0, 0.5, 0.25
	MemoryGB   int            `json:"memory_gb"`
	MIGProfile string         `gorm:"size:50" json:"mig_profile"`
	Status     string         `gorm:"size:50;default:'active'" json:"status"` // active, released, failed
	StartedAt  *time.Time     `json:"started_at"`
	EndedAt    *time.Time     `json:"ended_at"`
}
