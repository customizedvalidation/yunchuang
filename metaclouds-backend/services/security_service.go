package services

import (
	"errors"
	"metaclouds-backend/config"
	"metaclouds-backend/models"
	"metaclouds-backend/pkg/logger"
	"time"
)

type SecurityService struct {
	db     *models.MemoryStore
	config *config.Config
}

func NewSecurityService(db interface{}, config *config.Config) *SecurityService {
	memoryStore, err := models.GetDBStore(db, "SecurityService")
	if err != nil {
		logger.ErrorWithCtx(nil, "Failed to initialize SecurityService", err)
		return nil
	}
	return &SecurityService{
		db:     memoryStore,
		config: config,
	}
}

type CreateSecurityPolicyRequest struct {
	Name        string `json:"name" binding:"required"`
	Description string `json:"description"`
	Type        string `json:"type"`
	Enabled     bool   `json:"enabled"`
	Rules       string `json:"rules"`
	Details     string `json:"details"`
}

type UpdateSecurityPolicyRequest struct {
	Name        string `json:"name"`
	Description string `json:"description"`
	Status      string `json:"status"`
	Enabled     *bool  `json:"enabled"`
	Rules       string `json:"rules"`
	Details     string `json:"details"`
}

func (s *SecurityService) GetSecurityPolicies() ([]models.SecurityPolicy, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var policies []models.SecurityPolicy
	for _, policy := range s.db.SecurityPolicies {
		policies = append(policies, *policy)
	}
	return policies, nil
}

func (s *SecurityService) GetSecurityPolicy(id uint) (*models.SecurityPolicy, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	policy, exists := s.db.SecurityPolicies[id]
	if !exists {
		return nil, errors.New("security policy not found")
	}
	return policy, nil
}

func (s *SecurityService) CreateSecurityPolicy(req CreateSecurityPolicyRequest) (*models.SecurityPolicy, error) {
	s.db.Mu.RLock()
	for _, p := range s.db.SecurityPolicies {
		if p.Name == req.Name {
			s.db.Mu.RUnlock()
			return nil, errors.New("security policy name already exists")
		}
	}
	s.db.Mu.RUnlock()

	s.db.Mu.Lock()
	policy := &models.SecurityPolicy{
		ID:          s.db.SecurityPolicySeq,
		Name:        req.Name,
		Description: req.Description,
		Type:        req.Type,
		Status:      "active",
		Enabled:     req.Enabled,
		Rules:       req.Rules,
		Details:     req.Details,
		CreatedAt:   time.Now(),
		UpdatedAt:   time.Now(),
	}
	s.db.SecurityPolicies[s.db.SecurityPolicySeq] = policy
	s.db.SecurityPolicySeq++
	s.db.Mu.Unlock()

	return policy, nil
}

func (s *SecurityService) UpdateSecurityPolicy(id uint, req UpdateSecurityPolicyRequest) (*models.SecurityPolicy, error) {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	policy, exists := s.db.SecurityPolicies[id]
	if !exists {
		return nil, errors.New("security policy not found")
	}

	if req.Name != "" && req.Name != policy.Name {
		for _, p := range s.db.SecurityPolicies {
			if p.Name == req.Name && p.ID != id {
				return nil, errors.New("security policy name already exists")
			}
		}
		policy.Name = req.Name
	}
	if req.Description != "" {
		policy.Description = req.Description
	}
	if req.Status != "" {
		policy.Status = req.Status
	}
	if req.Enabled != nil {
		policy.Enabled = *req.Enabled
	}
	if req.Rules != "" {
		policy.Rules = req.Rules
	}
	if req.Details != "" {
		policy.Details = req.Details
	}
	policy.UpdatedAt = time.Now()

	return policy, nil
}

func (s *SecurityService) DeleteSecurityPolicy(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.SecurityPolicies[id]; !exists {
		return errors.New("security policy not found")
	}

	delete(s.db.SecurityPolicies, id)
	return nil
}
