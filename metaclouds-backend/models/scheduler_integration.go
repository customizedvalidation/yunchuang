package models

import (
	"time"

	"gorm.io/gorm"
)

// SchedulerIntegration 外部调度器集成模型
type SchedulerIntegration struct {
	ID               uint           `gorm:"primaryKey" json:"id"`
	CreatedAt        time.Time      `json:"created_at"`
	UpdatedAt        time.Time      `json:"updated_at"`
	DeletedAt        gorm.DeletedAt `gorm:"index" json:"-"`
	Name             string         `gorm:"size:100;not null;unique" json:"name"`
	Type             string         `gorm:"size:20;not null" json:"type"` // slurm, lsf, sge, k8s_native
	Endpoint         string         `gorm:"size:500" json:"endpoint"`
	AuthType         string         `gorm:"size:20;default:'none'" json:"auth_type"` // none, ssh, token, mTLS
	AuthConfig       string         `gorm:"size:2000" json:"auth_config"`            // JSON: credentials/keys
	Version          string         `gorm:"size:50" json:"version"`
	Status           string         `gorm:"size:20;default:'inactive'" json:"status"` // active, inactive, error
	DefaultPartition string         `gorm:"size:100" json:"default_partition"`
	DefaultQoS       string         `gorm:"size:50" json:"default_qos"`
	MaxNodes         int            `gorm:"default:0" json:"max_nodes"`
	MaxJobs          int            `gorm:"default:0" json:"max_jobs"`
	LastSyncAt       *time.Time     `json:"last_sync_at"`
	Details          string         `gorm:"size:2000" json:"details"`
}
