package services

import (
	"errors"
	"metaclouds-backend/config"
	"metaclouds-backend/models"
	"metaclouds-backend/pkg/logger"
	"time"
)

type ResourceService struct {
	db     *models.MemoryStore
	config *config.Config
}

func NewResourceService(db interface{}, config *config.Config) *ResourceService {
	memoryStore, err := models.GetDBStore(db, "ResourceService")
	if err != nil {
		logger.ErrorWithCtx(nil, "Failed to initialize ResourceService", err)
		return nil
	}
	return &ResourceService{
		db:     memoryStore,
		config: config,
	}
}

func (s *ResourceService) GetResources() ([]models.Resource, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var resources []models.Resource
	for _, resource := range s.db.Resources {
		resources = append(resources, *resource)
	}
	return resources, nil
}

func (s *ResourceService) GetResource(id uint) (*models.Resource, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	resource, exists := s.db.Resources[id]
	if !exists {
		return nil, errors.New("resource not found")
	}
	return resource, nil
}

type UpdateResourceRequest struct {
	Status      string  `json:"status"`
	Total       int     `json:"total" binding:"gte=0"`
	Used        int     `json:"used" binding:"gte=0"`
	Available   int     `json:"available" binding:"gte=0"`
	Utilization float64 `json:"utilization" binding:"gte=0"`
	Details     string  `json:"details"`
}

func (s *ResourceService) UpdateResource(id uint, req UpdateResourceRequest) (*models.Resource, error) {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	resource, exists := s.db.Resources[id]
	if !exists {
		return nil, errors.New("resource not found")
	}

	if req.Status != "" {
		resource.Status = req.Status
	}
	if req.Total > 0 {
		resource.Total = req.Total
	}
	if req.Used >= 0 {
		resource.Used = req.Used
	}
	if req.Available >= 0 {
		resource.Available = req.Available
	}
	if req.Utilization >= 0 {
		resource.Utilization = req.Utilization
	}
	if req.Details != "" {
		resource.Details = req.Details
	}
	resource.UpdatedAt = time.Now()

	return resource, nil
}
