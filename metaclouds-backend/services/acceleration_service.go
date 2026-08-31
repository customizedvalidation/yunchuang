package services

import (
	"errors"
	"metaclouds-backend/config"
	"metaclouds-backend/models"
	"metaclouds-backend/pkg/logger"
	"time"
)

type AccelerationService struct {
	db     *models.MemoryStore
	config *config.Config
}

func NewAccelerationService(db interface{}, config *config.Config) *AccelerationService {
	memoryStore, err := models.GetDBStore(db, "AccelerationService")
	if err != nil {
		logger.ErrorWithCtx(nil, "Failed to initialize AccelerationService", err)
		return nil
	}
	return &AccelerationService{
		db:     memoryStore,
		config: config,
	}
}

type CreateAccelerationSuiteRequest struct {
	Name        string `json:"name" binding:"required"`
	Description string `json:"description"`
	Type        string `json:"type"`
	Version     string `json:"version"`
	Enabled     bool   `json:"enabled"`
	Details     string `json:"details"`
}

type UpdateAccelerationSuiteRequest struct {
	Name        string `json:"name"`
	Description string `json:"description"`
	Status      string `json:"status"`
	Enabled     *bool  `json:"enabled"`
	Details     string `json:"details"`
}

func (s *AccelerationService) GetAccelerationSuites() ([]models.AccelerationSuite, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var suites []models.AccelerationSuite
	for _, suite := range s.db.AccelerationSuites {
		suites = append(suites, *suite)
	}
	return suites, nil
}

func (s *AccelerationService) GetAccelerationSuite(id uint) (*models.AccelerationSuite, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	suite, exists := s.db.AccelerationSuites[id]
	if !exists {
		return nil, errors.New("acceleration suite not found")
	}
	return suite, nil
}

func (s *AccelerationService) CreateAccelerationSuite(req CreateAccelerationSuiteRequest) (*models.AccelerationSuite, error) {
	s.db.Mu.RLock()
	for _, a := range s.db.AccelerationSuites {
		if a.Name == req.Name {
			s.db.Mu.RUnlock()
			return nil, errors.New("acceleration suite name already exists")
		}
	}
	s.db.Mu.RUnlock()

	s.db.Mu.Lock()
	suite := &models.AccelerationSuite{
		ID:          s.db.AccelerationSuiteSeq,
		Name:        req.Name,
		Description: req.Description,
		Type:        req.Type,
		Version:     req.Version,
		Status:      "active",
		Enabled:     req.Enabled,
		Details:     req.Details,
		CreatedAt:   time.Now(),
		UpdatedAt:   time.Now(),
	}
	s.db.AccelerationSuites[s.db.AccelerationSuiteSeq] = suite
	s.db.AccelerationSuiteSeq++
	s.db.Mu.Unlock()

	return suite, nil
}

func (s *AccelerationService) UpdateAccelerationSuite(id uint, req UpdateAccelerationSuiteRequest) (*models.AccelerationSuite, error) {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	suite, exists := s.db.AccelerationSuites[id]
	if !exists {
		return nil, errors.New("acceleration suite not found")
	}

	if req.Name != "" && req.Name != suite.Name {
		for _, a := range s.db.AccelerationSuites {
			if a.Name == req.Name && a.ID != id {
				return nil, errors.New("acceleration suite name already exists")
			}
		}
		suite.Name = req.Name
	}
	if req.Description != "" {
		suite.Description = req.Description
	}
	if req.Status != "" {
		suite.Status = req.Status
	}
	if req.Enabled != nil {
		suite.Enabled = *req.Enabled
	}
	if req.Details != "" {
		suite.Details = req.Details
	}
	suite.UpdatedAt = time.Now()

	return suite, nil
}

func (s *AccelerationService) DeleteAccelerationSuite(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.AccelerationSuites[id]; !exists {
		return errors.New("acceleration suite not found")
	}

	delete(s.db.AccelerationSuites, id)
	return nil
}
