package models

import (
	"time"
)

// PartitionPermission 分区权限模型
type PartitionPermission struct {
	ID            uint       `gorm:"primaryKey" json:"id"`
	CreatedAt     time.Time  `json:"created_at"`
	UpdatedAt     time.Time  `json:"updated_at"`
	PartitionID   uint       `gorm:"index;not null" json:"partition_id"`
	Partition     *Partition `gorm:"foreignKey:PartitionID" json:"partition,omitempty"`
	PrincipalType string     `gorm:"size:20;not null" json:"principal_type"`               // user, group
	PrincipalID   uint       `gorm:"not null" json:"principal_id"`
	AccessLevel   string     `gorm:"size:20;not null;default:'view'" json:"access_level"` // view, submit, admin
}
