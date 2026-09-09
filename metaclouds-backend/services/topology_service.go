package services

import (
	"time"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
	mcerrors "metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/logger"
)

// TopologyService 拓扑感知服务
type TopologyService struct {
	db     *models.MemoryStore
	config *config.Config
}

func NewTopologyService(db interface{}, config *config.Config) *TopologyService {
	memoryStore, err := models.GetDBStore(db, "TopologyService")
	if err != nil {
		logger.ErrorWithCtx(nil, "Failed to initialize TopologyService", err)
		return nil
	}
	return &TopologyService{
		db:     memoryStore,
		config: config,
	}
}

// --- NodeTopology CRUD ---

func (s *TopologyService) ListNodeTopologies(clusterID uint) ([]models.NodeTopology, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var topologies []models.NodeTopology
	for _, t := range s.db.NodeTopologies {
		if clusterID > 0 && t.ClusterID != clusterID {
			continue
		}
		topologies = append(topologies, *t)
	}
	return topologies, nil
}

func (s *TopologyService) GetNodeTopology(id uint) (*models.NodeTopology, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	topo, exists := s.db.NodeTopologies[id]
	if !exists {
		return nil, mcerrors.NotFound("node topology not found")
	}
	return topo, nil
}

func (s *TopologyService) GetNodeTopologyByName(nodeName string) (*models.NodeTopology, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	for _, t := range s.db.NodeTopologies {
		if t.NodeName == nodeName {
			return t, nil
		}
	}
	return nil, mcerrors.NotFound("node topology not found")
}

func (s *TopologyService) CreateNodeTopology(topo *models.NodeTopology) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	// 检查节点名唯一
	for _, t := range s.db.NodeTopologies {
		if t.NodeName == topo.NodeName {
			return mcerrors.Conflict("node topology already exists for this node")
		}
	}

	topo.ID = s.db.NodeTopologySeq
	topo.CreatedAt = time.Now()
	topo.UpdatedAt = time.Now()
	s.db.NodeTopologies[s.db.NodeTopologySeq] = topo
	s.db.NodeTopologySeq++
	return nil
}

func (s *TopologyService) UpdateNodeTopology(topo *models.NodeTopology) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	existing, exists := s.db.NodeTopologies[topo.ID]
	if !exists {
		return mcerrors.NotFound("node topology not found")
	}

	if topo.NodeName != "" {
		existing.NodeName = topo.NodeName
	}
	if topo.RackID != "" {
		existing.RackID = topo.RackID
	}
	if topo.SwitchID != "" {
		existing.SwitchID = topo.SwitchID
	}
	if topo.PodID != "" {
		existing.PodID = topo.PodID
	}
	if topo.NUMANodes != "" {
		existing.NUMANodes = topo.NUMANodes
	}
	if topo.GPUTopology != "" {
		existing.GPUTopology = topo.GPUTopology
	}
	if topo.NetworkType != "" {
		existing.NetworkType = topo.NetworkType
	}
	if topo.NetworkBandwidthGbps >= 0 {
		existing.NetworkBandwidthGbps = topo.NetworkBandwidthGbps
	}
	existing.RDMAEnabled = topo.RDMAEnabled
	if topo.FaultDomain != "" {
		existing.FaultDomain = topo.FaultDomain
	}
	existing.UpdatedAt = time.Now()
	return nil
}

func (s *TopologyService) DeleteNodeTopology(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.NodeTopologies[id]; !exists {
		return mcerrors.NotFound("node topology not found")
	}
	delete(s.db.NodeTopologies, id)
	return nil
}

// CalculateTopologyScore 计算候选节点的拓扑评分
// 评分基于：同机架加分、同交换机加分、同 Pod 加分、NVLink 直连加分、RDMA 加分
func (s *TopologyService) CalculateTopologyScore(job *models.Job, candidateNodes []string) (map[string]float64, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	scores := make(map[string]float64)

	// 构建节点拓扑映射
	topoMap := make(map[string]*models.NodeTopology)
	for _, t := range s.db.NodeTopologies {
		topoMap[t.NodeName] = t
	}

	// 获取作业已分配节点（用于亲和性计算）
	var allocatedTopos []*models.NodeTopology
	for _, alloc := range s.db.GPUAllocations {
		if alloc.JobID == job.ID && alloc.Status == "active" {
			if device, ok := s.db.GPUDevices[alloc.DeviceID]; ok {
				if t, ok := topoMap[device.NodeName]; ok {
					allocatedTopos = append(allocatedTopos, t)
				}
			}
		}
	}

	for _, nodeName := range candidateNodes {
		score := 0.0
		topo, exists := topoMap[nodeName]
		if !exists {
			scores[nodeName] = 0.0
			continue
		}

		// 基础分
		score += 1.0

		// RDMA 加分
		if topo.RDMAEnabled {
			score += 2.0
		}

		// 网络带宽加分
		if topo.NetworkBandwidthGbps >= 100 {
			score += 1.0
		}

		// 与已分配节点的亲和性评分
		for _, allocated := range allocatedTopos {
			if allocated.NodeName == nodeName {
				score += 10.0 // 同节点最高亲和
			}
			if allocated.RackID != "" && allocated.RackID == topo.RackID {
				score += 5.0 // 同机架
			}
			if allocated.SwitchID != "" && allocated.SwitchID == topo.SwitchID {
				score += 3.0 // 同交换机
			}
			if allocated.PodID != "" && allocated.PodID == topo.PodID {
				score += 1.0 // 同 Pod
			}
		}

		// 反亲和性：如果作业要求拓扑反亲和，同故障域减分
		if job.TopologyAntiAffinity {
			for _, allocated := range allocatedTopos {
				if allocated.FaultDomain != "" && allocated.FaultDomain == topo.FaultDomain {
					score -= 8.0
				}
			}
		}

		// 网络需求匹配
		if job.NetworkRequirement == "rdma" && !topo.RDMAEnabled {
			score -= 5.0
		}

		scores[nodeName] = score
	}

	return scores, nil
}
