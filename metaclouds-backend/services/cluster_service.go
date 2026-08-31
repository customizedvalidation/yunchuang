package services

import (
	"errors"
	"metaclouds-backend/config"
	"metaclouds-backend/models"
	"metaclouds-backend/pkg/logger"
	"time"
)

type ClusterService struct {
	db     *models.MemoryStore
	config *config.Config
}

func NewClusterService(db interface{}, config *config.Config) *ClusterService {
	memoryStore, err := models.GetDBStore(db, "ClusterService")
	if err != nil {
		logger.ErrorWithCtx(nil, "Failed to initialize ClusterService", err)
		return nil
	}
	return &ClusterService{
		db:     memoryStore,
		config: config,
	}
}

type CreateClusterRequest struct {
	Name        string `json:"name" binding:"required"`
	Description string `json:"description"`
	Nodes       int    `json:"nodes"`
	GPUs        int    `json:"gpus"`
	CPUs        int    `json:"cpus"`
	Memory      int    `json:"memory"`
	Storage     int    `json:"storage"`
	NetworkType string `json:"network_type"`
	Location    string `json:"location"`
}

type UpdateClusterRequest struct {
	Name        string `json:"name"`
	Description string `json:"description"`
	Status      string `json:"status"`
	Nodes       int    `json:"nodes"`
	GPUs        int    `json:"gpus"`
	CPUs        int    `json:"cpus"`
	Memory      int    `json:"memory"`
	Storage     int    `json:"storage"`
	NetworkType string `json:"network_type"`
	Location    string `json:"location"`
}

func (s *ClusterService) GetClusters() ([]models.Cluster, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var clusters []models.Cluster
	for _, cluster := range s.db.Clusters {
		clusters = append(clusters, *cluster)
	}
	return clusters, nil
}

func (s *ClusterService) GetCluster(id uint) (*models.Cluster, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	cluster, exists := s.db.Clusters[id]
	if !exists {
		return nil, errors.New("cluster not found")
	}
	return cluster, nil
}

func (s *ClusterService) CreateCluster(req CreateClusterRequest) (*models.Cluster, error) {
	s.db.Mu.RLock()
	for _, c := range s.db.Clusters {
		if c.Name == req.Name {
			s.db.Mu.RUnlock()
			return nil, errors.New("cluster name already exists")
		}
	}
	s.db.Mu.RUnlock()

	s.db.Mu.Lock()
	cluster := &models.Cluster{
		ID:          s.db.ClusterSeq,
		Name:        req.Name,
		Description: req.Description,
		Status:      "active",
		Nodes:       req.Nodes,
		GPUs:        req.GPUs,
		CPUs:        req.CPUs,
		Memory:      req.Memory,
		Storage:     req.Storage,
		NetworkType: req.NetworkType,
		Location:    req.Location,
		CreatedAt:   time.Now(),
		UpdatedAt:   time.Now(),
	}
	s.db.Clusters[s.db.ClusterSeq] = cluster
	s.db.ClusterSeq++
	s.db.Mu.Unlock()

	return cluster, nil
}

func (s *ClusterService) UpdateCluster(id uint, req UpdateClusterRequest) (*models.Cluster, error) {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	cluster, exists := s.db.Clusters[id]
	if !exists {
		return nil, errors.New("cluster not found")
	}

	if req.Name != "" && req.Name != cluster.Name {
		for _, c := range s.db.Clusters {
			if c.Name == req.Name && c.ID != id {
				return nil, errors.New("cluster name already exists")
			}
		}
		cluster.Name = req.Name
	}
	if req.Description != "" {
		cluster.Description = req.Description
	}
	if req.Status != "" {
		cluster.Status = req.Status
	}
	if req.Nodes > 0 {
		cluster.Nodes = req.Nodes
	}
	if req.GPUs >= 0 {
		cluster.GPUs = req.GPUs
	}
	if req.CPUs >= 0 {
		cluster.CPUs = req.CPUs
	}
	if req.Memory >= 0 {
		cluster.Memory = req.Memory
	}
	if req.Storage >= 0 {
		cluster.Storage = req.Storage
	}
	if req.NetworkType != "" {
		cluster.NetworkType = req.NetworkType
	}
	if req.Location != "" {
		cluster.Location = req.Location
	}
	cluster.UpdatedAt = time.Now()

	return cluster, nil
}

func (s *ClusterService) DeleteCluster(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.Clusters[id]; !exists {
		return errors.New("cluster not found")
	}

	delete(s.db.Clusters, id)
	return nil
}
