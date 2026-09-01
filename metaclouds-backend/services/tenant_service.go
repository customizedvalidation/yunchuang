package services

import (
	"errors"
	"metaclouds-backend/config"
	"metaclouds-backend/models"
	"metaclouds-backend/pkg/logger"
	"time"
)

type TenantService struct {
	db     *models.MemoryStore
	config *config.Config
}

func NewTenantService(db interface{}, config *config.Config) *TenantService {
	memoryStore, err := models.GetDBStore(db, "TenantService")
	if err != nil {
		logger.ErrorWithCtx(nil, "Failed to initialize TenantService", err)
		return nil
	}
	return &TenantService{
		db:     memoryStore,
		config: config,
	}
}

type CreateTenantRequest struct {
	Name         string `json:"name" binding:"required"`
	Description  string `json:"description"`
	GPUQuota     int    `json:"gpu_quota" binding:"gte=0"`
	CPUQuota     int    `json:"cpu_quota" binding:"gte=0"`
	MemoryQuota  int    `json:"memory_quota" binding:"gte=0"`
	StorageQuota int    `json:"storage_quota" binding:"gte=0"`
}

type UpdateTenantRequest struct {
	Name         string `json:"name"`
	Description  string `json:"description"`
	Status       string `json:"status"`
	GPUQuota     int    `json:"gpu_quota" binding:"gte=0"`
	CPUQuota     int    `json:"cpu_quota" binding:"gte=0"`
	MemoryQuota  int    `json:"memory_quota" binding:"gte=0"`
	StorageQuota int    `json:"storage_quota" binding:"gte=0"`
}

func (s *TenantService) GetTenants() ([]models.Tenant, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var tenants []models.Tenant
	for _, tenant := range s.db.Tenants {
		tenants = append(tenants, *tenant)
	}
	return tenants, nil
}

func (s *TenantService) GetTenant(id uint) (*models.Tenant, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	tenant, exists := s.db.Tenants[id]
	if !exists {
		return nil, errors.New("tenant not found")
	}
	return tenant, nil
}

func (s *TenantService) CreateTenant(req CreateTenantRequest) (*models.Tenant, error) {
	s.db.Mu.RLock()
	for _, t := range s.db.Tenants {
		if t.Name == req.Name {
			s.db.Mu.RUnlock()
			return nil, errors.New("tenant name already exists")
		}
	}
	s.db.Mu.RUnlock()

	s.db.Mu.Lock()
	tenant := &models.Tenant{
		ID:           s.db.TenantSeq,
		Name:         req.Name,
		Description:  req.Description,
		Status:       "active",
		GPUQuota:     req.GPUQuota,
		CPUQuota:     req.CPUQuota,
		MemoryQuota:  req.MemoryQuota,
		StorageQuota: req.StorageQuota,
		CreatedAt:    time.Now(),
		UpdatedAt:    time.Now(),
	}
	s.db.Tenants[s.db.TenantSeq] = tenant
	s.db.TenantSeq++
	s.db.Mu.Unlock()

	return tenant, nil
}

func (s *TenantService) UpdateTenant(id uint, req UpdateTenantRequest) (*models.Tenant, error) {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	tenant, exists := s.db.Tenants[id]
	if !exists {
		return nil, errors.New("tenant not found")
	}

	if req.Name != "" && req.Name != tenant.Name {
		for _, t := range s.db.Tenants {
			if t.Name == req.Name && t.ID != id {
				return nil, errors.New("tenant name already exists")
			}
		}
		tenant.Name = req.Name
	}
	if req.Description != "" {
		tenant.Description = req.Description
	}
	if req.Status != "" {
		tenant.Status = req.Status
	}
	if req.GPUQuota > 0 {
		tenant.GPUQuota = req.GPUQuota
	}
	if req.CPUQuota > 0 {
		tenant.CPUQuota = req.CPUQuota
	}
	if req.MemoryQuota > 0 {
		tenant.MemoryQuota = req.MemoryQuota
	}
	if req.StorageQuota > 0 {
		tenant.StorageQuota = req.StorageQuota
	}
	tenant.UpdatedAt = time.Now()

	return tenant, nil
}

func (s *TenantService) DeleteTenant(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.Tenants[id]; !exists {
		return errors.New("tenant not found")
	}

	delete(s.db.Tenants, id)
	return nil
}
