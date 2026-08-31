package models

import (
	"time"

	"gorm.io/gorm"
)

// Alert 告警模型
type Alert struct {
	ID          uint           `gorm:"primaryKey" json:"id"`
	CreatedAt   time.Time      `json:"created_at"`
	UpdatedAt   time.Time      `json:"updated_at"`
	DeletedAt   gorm.DeletedAt `gorm:"index" json:"-"`
	ClusterID   *uint          `json:"cluster_id"`
	Cluster     *Cluster       `gorm:"foreignKey:ClusterID" json:"cluster,omitempty"`
	ResourceID  *uint          `json:"resource_id"`
	Resource    *Resource      `gorm:"foreignKey:ResourceID" json:"resource,omitempty"`
	JobID       *uint          `json:"job_id"`
	Job         *Job           `gorm:"foreignKey:JobID" json:"job,omitempty"`
	Type        string         `gorm:"size:50;not null" json:"type"` // system, resource, job, security
	Level       string         `gorm:"size:50;not null" json:"level"` // info, warning, error, critical
	Message     string         `gorm:"size:1000;not null" json:"message"`
	Status      string         `gorm:"size:50;not null;default:'active'" json:"status"` // active, resolved, ignored
	ResolvedAt  *time.Time     `json:"resolved_at"`
	Details     string         `gorm:"size:1000" json:"details"`
}