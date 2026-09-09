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

	// P0/P1 差距对齐新增字段
	PartitionID               uint       `gorm:"index" json:"partition_id"`
	Partition                 *Partition `gorm:"foreignKey:PartitionID" json:"partition,omitempty"`
	SchedulerType             string     `gorm:"size:20;default:'k8s_native'" json:"scheduler_type"`
	SchedulerJobID            string     `gorm:"size:100" json:"scheduler_job_id"`
	GPUFraction               float64    `gorm:"default:1.0" json:"gpu_fraction"` // 1.0, 0.5, 0.25
	GPUMemoryGB               int        `gorm:"default:0" json:"gpu_memory_gb"`
	GPUVendor                 string     `gorm:"size:50" json:"gpu_vendor"`
	QoS                       string     `gorm:"size:50" json:"qos"`
	NodesRequested            int        `gorm:"default:1" json:"nodes_requested"`
	NodeSelector              string     `gorm:"size:1000" json:"node_selector"` // JSON map
	Affinity                  string     `gorm:"size:2000" json:"affinity"`      // JSON
	Tolerations               string     `gorm:"size:1000" json:"tolerations"`   // JSON
	ElasticEnabled            bool       `gorm:"default:false" json:"elastic_enabled"`
	MinGPUs                   int        `gorm:"default:0" json:"min_gpus"`
	MaxGPUs                   int        `gorm:"default:0" json:"max_gpus"`
	ScalingPolicy             string     `gorm:"size:500" json:"scaling_policy"` // JSON
	CheckpointEnabled         bool       `gorm:"default:false" json:"checkpoint_enabled"`
	CheckpointIntervalMinutes int        `gorm:"default:0" json:"checkpoint_interval_minutes"`
	MaxRetries                int        `gorm:"default:0" json:"max_retries"`
	RetryCount                int        `gorm:"default:0" json:"retry_count"`
	FaultToleranceLevel       string     `gorm:"size:20;default:'none'" json:"fault_tolerance_level"` // none, node, rack, switch
	TopologyAffinity          string     `gorm:"size:20" json:"topology_affinity"`                    // node, rack, switch, cluster
	TopologyAntiAffinity      bool       `gorm:"default:false" json:"topology_anti_affinity"`
	NetworkRequirement        string     `gorm:"size:20" json:"network_requirement"` // rdma, ethernet, any
}
