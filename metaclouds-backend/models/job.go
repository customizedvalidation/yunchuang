package models

import (
	"time"

	"gorm.io/gorm"
)

// Job 作业模型
type Job struct {
	ID          uint           `gorm:"primaryKey" json:"id"`
	CreatedAt   time.Time      `json:"created_at"`
	UpdatedAt   time.Time      `json:"updated_at"`
	DeletedAt   gorm.DeletedAt `gorm:"index" json:"-"`
	ClusterID   uint           `gorm:"index" json:"cluster_id"`
	Cluster     *Cluster       `gorm:"foreignKey:ClusterID" json:"cluster,omitempty"`
	TenantID    uint           `gorm:"index" json:"tenant_id"`
	Tenant      *Tenant        `gorm:"foreignKey:TenantID" json:"tenant,omitempty"`
	UserID      uint           `gorm:"index" json:"user_id"`
	User        *User          `gorm:"foreignKey:UserID" json:"user,omitempty"`
	Name        string         `gorm:"size:255;not null" json:"name"`
	Description string         `gorm:"size:1000" json:"description"`
	Status      string         `gorm:"size:50;not null;default:'pending';index" json:"status"` // pending, running, completed, failed, cancelled
	Type        string         `gorm:"size:50;not null;index" json:"type"`                     // training, inference, batch
	Priority    int            `gorm:"default:0;index" json:"priority"`                        // 优先级: 0=low, 1=medium, 2=high, 3=critical
	GPUs        int            `json:"gpus"`
	CPUs        int            `json:"cpus"`
	Memory      int            `json:"memory"`   // 内存（GB）
	Duration    int            `json:"duration"` // 预计运行时间（分钟）
	StartTime   *time.Time     `json:"start_time"`
	EndTime     *time.Time     `json:"end_time"`
	Progress    int            `json:"progress"` // 进度（0-100）
	OutputPath  string         `gorm:"size:1000" json:"output_path"`
	ErrorMsg    string         `gorm:"size:1000" json:"error_msg"`
}
