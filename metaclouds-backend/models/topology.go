package models

import (
	"time"
)

// NodeTopology 节点拓扑感知模型（P1）
type NodeTopology struct {
	ID                   uint      `gorm:"primaryKey" json:"id"`
	CreatedAt            time.Time `json:"created_at"`
	UpdatedAt            time.Time `json:"updated_at"`
	ClusterID            uint      `gorm:"index" json:"cluster_id"`
	NodeName             string    `gorm:"size:255;not null" json:"node_name"`
	RackID               string    `gorm:"size:100" json:"rack_id"`
	SwitchID             string    `gorm:"size:100" json:"switch_id"`
	PodID                string    `gorm:"size:100" json:"pod_id"` // 可用区/Pod
	NUMANodes            string    `gorm:"size:1000" json:"numa_nodes"` // JSON: NUMA topology
	GPUTopology          string    `gorm:"size:2000" json:"gpu_topology"` // JSON: NVLink connection matrix
	NetworkType          string    `gorm:"size:50" json:"network_type"` // roce, ib, ethernet
	NetworkBandwidthGbps int       `gorm:"default:0" json:"network_bandwidth_gbps"`
	RDMAEnabled          bool      `gorm:"default:false" json:"rdma_enabled"`
	FaultDomain          string    `gorm:"size:100" json:"fault_domain"`
}
