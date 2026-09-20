package services

import (
	"context"
	"fmt"
	"time"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
	mcerrors "metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/logger"
)

// QuotaService 多维度资源配额服务
type QuotaService struct {
	db     *models.MemoryStore
	config *config.Config
}

func NewQuotaService(db interface{}, config *config.Config) *QuotaService {
	memoryStore, err := models.GetDBStore(db, "QuotaService")
	if err != nil {
		logger.ErrorWithCtx(context.Background(), "Failed to initialize QuotaService", err)
		return nil
	}
	return &QuotaService{
		db:     memoryStore,
		config: config,
	}
}

// --- ResourceQuota CRUD ---

func (s *QuotaService) GetQuotas(scopeType string, scopeID uint) ([]models.ResourceQuota, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var quotas []models.ResourceQuota
	for _, q := range s.db.ResourceQuotas {
		if scopeType != "" && q.ScopeType != scopeType {
			continue
		}
		if scopeID > 0 && q.ScopeID != scopeID {
			continue
		}
		quotas = append(quotas, *q)
	}
	return quotas, nil
}

func (s *QuotaService) GetQuota(id uint) (*models.ResourceQuota, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	quota, exists := s.db.ResourceQuotas[id]
	if !exists {
		return nil, mcerrors.NotFound("resource quota not found")
	}
	return quota, nil
}

func (s *QuotaService) CreateQuota(quota *models.ResourceQuota) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	// 检查同 scope + resource_type 是否已存在
	for _, q := range s.db.ResourceQuotas {
		if q.ScopeType == quota.ScopeType && q.ScopeID == quota.ScopeID && q.ResourceType == quota.ResourceType {
			return mcerrors.Conflict("quota already exists for this scope and resource type")
		}
	}

	quota.ID = s.db.ResourceQuotaSeq
	quota.CreatedAt = time.Now()
	quota.UpdatedAt = time.Now()
	if quota.Status == "" {
		quota.Status = "active"
	}
	s.db.ResourceQuotas[s.db.ResourceQuotaSeq] = quota
	s.db.ResourceQuotaSeq++
	return nil
}

func (s *QuotaService) UpdateQuota(quota *models.ResourceQuota) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	existing, exists := s.db.ResourceQuotas[quota.ID]
	if !exists {
		return mcerrors.NotFound("resource quota not found")
	}

	if quota.Limit >= 0 {
		existing.Limit = quota.Limit
	}
	if quota.GPUFractionLimit >= 0 {
		existing.GPUFractionLimit = quota.GPUFractionLimit
	}
	if quota.GPUMemoryLimitGB >= 0 {
		existing.GPUMemoryLimitGB = quota.GPUMemoryLimitGB
	}
	if quota.MaxPodCPU >= 0 {
		existing.MaxPodCPU = quota.MaxPodCPU
	}
	if quota.MinPodCPU >= 0 {
		existing.MinPodCPU = quota.MinPodCPU
	}
	if quota.MaxPodMemory >= 0 {
		existing.MaxPodMemory = quota.MaxPodMemory
	}
	if quota.MinPodMemory >= 0 {
		existing.MinPodMemory = quota.MinPodMemory
	}
	if quota.MaxPodGPU >= 0 {
		existing.MaxPodGPU = quota.MaxPodGPU
	}
	if quota.MinPodGPU >= 0 {
		existing.MinPodGPU = quota.MinPodGPU
	}
	if quota.Status != "" {
		existing.Status = quota.Status
	}
	existing.UpdatedAt = time.Now()
	return nil
}

func (s *QuotaService) DeleteQuota(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.ResourceQuotas[id]; !exists {
		return mcerrors.NotFound("resource quota not found")
	}
	delete(s.db.ResourceQuotas, id)
	return nil
}

// --- Quota Operations ---

func (s *QuotaService) findQuota(scopeType string, scopeID uint, resourceType string) *models.ResourceQuota {
	for _, q := range s.db.ResourceQuotas {
		if q.ScopeType == scopeType && q.ScopeID == scopeID && q.ResourceType == resourceType && q.Status == "active" {
			return q
		}
	}
	return nil
}

func (s *QuotaService) CheckQuota(scopeType string, scopeID uint, resourceType string, requested int, gpuFraction float64) (bool, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	quota := s.findQuota(scopeType, scopeID, resourceType)
	if quota == nil {
		// 无配额限制视为允许
		return true, nil
	}

	// 检查整数资源限制
	if quota.Limit > 0 && quota.Used+requested > quota.Limit {
		return false, nil
	}

	// 检查 GPU 分数限制
	if resourceType == "gpu" && quota.GPUFractionLimit > 0 {
		var usedFraction float64
		for _, q := range s.db.ResourceQuotas {
			if q.ScopeType == scopeType && q.ScopeID == scopeID && q.ResourceType == "gpu" {
				usedFraction += float64(q.Used)
			}
		}
		if usedFraction+gpuFraction > quota.GPUFractionLimit {
			return false, nil
		}
	}

	return true, nil
}

func (s *QuotaService) ConsumeQuota(scopeType string, scopeID uint, resourceType string, amount int, gpuFraction float64) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	quota := s.findQuota(scopeType, scopeID, resourceType)
	if quota == nil {
		return nil
	}

	if quota.Limit > 0 && quota.Used+amount > quota.Limit {
		return mcerrors.New(mcerrors.ErrForbidden,
			fmt.Sprintf("quota exceeded: used=%d, requested=%d, limit=%d", quota.Used, amount, quota.Limit))
	}

	quota.Used += amount
	if resourceType == "gpu" && gpuFraction > 0 {
		quota.GPUMemoryUsedGB += int(gpuFraction * 100) // 简化存储
	}
	quota.UpdatedAt = time.Now()
	return nil
}

func (s *QuotaService) ReleaseQuota(scopeType string, scopeID uint, resourceType string, amount int, gpuFraction float64) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	quota := s.findQuota(scopeType, scopeID, resourceType)
	if quota == nil {
		return nil
	}

	quota.Used -= amount
	if quota.Used < 0 {
		quota.Used = 0
	}
	if resourceType == "gpu" && gpuFraction > 0 {
		quota.GPUMemoryUsedGB -= int(gpuFraction * 100)
		if quota.GPUMemoryUsedGB < 0 {
			quota.GPUMemoryUsedGB = 0
		}
	}
	quota.UpdatedAt = time.Now()
	return nil
}

func (s *QuotaService) GetQuotaUsageSummary(scopeType string, scopeID uint) (map[string]interface{}, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	summary := make(map[string]interface{})
	for _, q := range s.db.ResourceQuotas {
		if scopeType != "" && q.ScopeType != scopeType {
			continue
		}
		if scopeID > 0 && q.ScopeID != scopeID {
			continue
		}
		key := fmt.Sprintf("%s_%s", q.ScopeType, q.ResourceType)
		usagePercent := 0.0
		if q.Limit > 0 {
			usagePercent = float64(q.Used) / float64(q.Limit) * 100
		}
		summary[key] = map[string]interface{}{
			"limit":         q.Limit,
			"used":          q.Used,
			"usage_percent": fmt.Sprintf("%.2f", usagePercent),
			"gpu_fraction_limit": q.GPUFractionLimit,
			"gpu_memory_limit_gb": q.GPUMemoryLimitGB,
			"gpu_memory_used_gb": q.GPUMemoryUsedGB,
		}
	}
	return summary, nil
}
