package services

import (
	"context"
	"time"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
	mcerrors "metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/logger"
)

// PartitionService 分区管理服务
type PartitionService struct {
	db     *models.MemoryStore
	config *config.Config
}

func NewPartitionService(db interface{}, config *config.Config) *PartitionService {
	memoryStore, err := models.GetDBStore(db, "PartitionService")
	if err != nil {
		logger.ErrorWithCtx(context.Background(), "Failed to initialize PartitionService", err)
		return nil
	}
	return &PartitionService{
		db:     memoryStore,
		config: config,
	}
}

// --- Partition CRUD ---

func (s *PartitionService) ListPartitions(clusterID uint, status string) ([]models.Partition, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var partitions []models.Partition
	for _, p := range s.db.Partitions {
		if clusterID > 0 && p.ClusterID != clusterID {
			continue
		}
		if status != "" && p.Status != status {
			continue
		}
		partitions = append(partitions, *p)
	}
	return partitions, nil
}

func (s *PartitionService) GetPartition(id uint) (*models.Partition, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	partition, exists := s.db.Partitions[id]
	if !exists {
		return nil, mcerrors.NotFound("partition not found")
	}
	return partition, nil
}

func (s *PartitionService) CreatePartition(partition *models.Partition) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	// 检查同集群下名称唯一
	for _, p := range s.db.Partitions {
		if p.ClusterID == partition.ClusterID && p.Name == partition.Name {
			return mcerrors.Conflict("partition name already exists in this cluster")
		}
	}

	partition.ID = s.db.PartitionSeq
	partition.CreatedAt = time.Now()
	partition.UpdatedAt = time.Now()
	if partition.Status == "" {
		partition.Status = "active"
	}
	if partition.SchedulerType == "" {
		partition.SchedulerType = "k8s_native"
	}
	s.db.Partitions[s.db.PartitionSeq] = partition
	s.db.PartitionSeq++
	return nil
}

func (s *PartitionService) UpdatePartition(partition *models.Partition) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	existing, exists := s.db.Partitions[partition.ID]
	if !exists {
		return mcerrors.NotFound("partition not found")
	}

	if partition.Name != "" {
		existing.Name = partition.Name
	}
	if partition.Description != "" {
		existing.Description = partition.Description
	}
	if partition.Priority >= 0 {
		existing.Priority = partition.Priority
	}
	if partition.MaxRuntimeMinutes >= 0 {
		existing.MaxRuntimeMinutes = partition.MaxRuntimeMinutes
	}
	if partition.NodeCount >= 0 {
		existing.NodeCount = partition.NodeCount
	}
	if partition.CPULimit >= 0 {
		existing.CPULimit = partition.CPULimit
	}
	if partition.GPUCount >= 0 {
		existing.GPUCount = partition.GPUCount
	}
	if partition.GPUVendor != "" {
		existing.GPUVendor = partition.GPUVendor
	}
	if partition.Status != "" {
		existing.Status = partition.Status
	}
	existing.AllowSharing = partition.AllowSharing
	if partition.DefaultQoS != "" {
		existing.DefaultQoS = partition.DefaultQoS
	}
	if partition.SchedulerType != "" {
		existing.SchedulerType = partition.SchedulerType
	}
	if partition.Nodes != "" {
		existing.Nodes = partition.Nodes
	}
	if partition.Taints != "" {
		existing.Taints = partition.Taints
	}
	if partition.Labels != "" {
		existing.Labels = partition.Labels
	}
	existing.UpdatedAt = time.Now()
	return nil
}

func (s *PartitionService) DeletePartition(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.Partitions[id]; !exists {
		return mcerrors.NotFound("partition not found")
	}
	delete(s.db.Partitions, id)

	// 级联删除分区权限
	for permID, perm := range s.db.PartitionPermissions {
		if perm.PartitionID == id {
			delete(s.db.PartitionPermissions, permID)
		}
	}
	return nil
}

func (s *PartitionService) UpdatePartitionPriority(id uint, priority int) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	existing, exists := s.db.Partitions[id]
	if !exists {
		return mcerrors.NotFound("partition not found")
	}
	existing.Priority = priority
	existing.UpdatedAt = time.Now()
	return nil
}

func (s *PartitionService) UpdatePartitionMaxRuntime(id uint, maxRuntime int) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	existing, exists := s.db.Partitions[id]
	if !exists {
		return mcerrors.NotFound("partition not found")
	}
	existing.MaxRuntimeMinutes = maxRuntime
	existing.UpdatedAt = time.Now()
	return nil
}

// --- Partition Permissions ---

func (s *PartitionService) SetPartitionPermission(partitionID uint, principalType string, principalID uint, accessLevel string) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.Partitions[partitionID]; !exists {
		return mcerrors.NotFound("partition not found")
	}

	// 检查是否已存在相同 principal 的权限，存在则更新
	for _, perm := range s.db.PartitionPermissions {
		if perm.PartitionID == partitionID && perm.PrincipalType == principalType && perm.PrincipalID == principalID {
			perm.AccessLevel = accessLevel
			perm.UpdatedAt = time.Now()
			return nil
		}
	}

	perm := &models.PartitionPermission{
		ID:            s.db.PartitionPermissionSeq,
		PartitionID:   partitionID,
		PrincipalType: principalType,
		PrincipalID:   principalID,
		AccessLevel:   accessLevel,
		CreatedAt:     time.Now(),
		UpdatedAt:     time.Now(),
	}
	s.db.PartitionPermissions[s.db.PartitionPermissionSeq] = perm
	s.db.PartitionPermissionSeq++
	return nil
}

func (s *PartitionService) RemovePartitionPermission(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.PartitionPermissions[id]; !exists {
		return mcerrors.NotFound("partition permission not found")
	}
	delete(s.db.PartitionPermissions, id)
	return nil
}

func (s *PartitionService) GetPartitionPermissions(partitionID uint) ([]models.PartitionPermission, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var permissions []models.PartitionPermission
	for _, p := range s.db.PartitionPermissions {
		if p.PartitionID == partitionID {
			permissions = append(permissions, *p)
		}
	}
	return permissions, nil
}

// accessLevelRank 权限等级排序：view < submit < admin
func accessLevelRank(level string) int {
	switch level {
	case "admin":
		return 3
	case "submit":
		return 2
	case "view":
		return 1
	default:
		return 0
	}
}

func (s *PartitionService) CheckPartitionAccess(userID uint, partitionID uint, requiredLevel string) (bool, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	requiredRank := accessLevelRank(requiredLevel)
	for _, perm := range s.db.PartitionPermissions {
		if perm.PartitionID == partitionID && perm.PrincipalType == "user" && perm.PrincipalID == userID {
			return accessLevelRank(perm.AccessLevel) >= requiredRank, nil
		}
	}
	// 未找到权限记录，默认无访问
	return false, nil
}
