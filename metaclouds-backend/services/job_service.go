package services

import (
	"context"
	"fmt"
	"metaclouds-backend/config"
	"metaclouds-backend/models"
	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/logger"
	"time"
)

type JobService struct {
	db         *models.MemoryStore
	config     *config.Config
	k8sService *K8SService
}

func NewJobService(db interface{}, config *config.Config, k8sService *K8SService) *JobService {
	memoryStore, err := models.GetDBStore(db, "JobService")
	if err != nil {
		logger.ErrorWithCtx(nil, "Failed to initialize JobService", err)
		return nil
	}
	return &JobService{
		db:         memoryStore,
		config:     config,
		k8sService: k8sService,
	}
}

type CreateJobRequest struct {
	Name        string `json:"name" binding:"required"`
	Description string `json:"description"`
	Type        string `json:"type"`
	Priority    int    `json:"priority"`
	GPUs        int    `json:"gpus"`
	CPUs        int    `json:"cpus"`
	Memory      int    `json:"memory"`
	Duration    int    `json:"duration"`
	ClusterID   uint   `json:"cluster_id"`
	TenantID    uint   `json:"tenant_id"`
	UserID      uint   `json:"user_id"`
}

type UpdateJobRequest struct {
	Name        string `json:"name"`
	Description string `json:"description"`
	Status      string `json:"status"`
	Priority    *int   `json:"priority"`
	Progress    int    `json:"progress"`
	OutputPath  string `json:"output_path"`
	ErrorMsg    string `json:"error_msg"`
}

// GetJobs 返回全部作业。
//
// 仅限内部调用（如调度器）。面向 HTTP 的读取必须走 GetJobsVisibleTo，
// 否则会跨租户泄露数据。
func (s *JobService) GetJobs() ([]models.Job, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	jobs := make([]models.Job, 0, len(s.db.Jobs))
	for _, job := range s.db.Jobs {
		jobs = append(jobs, *job)
	}
	return jobs, nil
}

// GetJobsVisibleTo 按租户范围返回作业。
// 管理员（isAdmin=true）可以看到全部租户的数据，普通用户只能看到本租户的作业。
func (s *JobService) GetJobsVisibleTo(tenantID uint, isAdmin bool) ([]models.Job, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	if isAdmin {
		jobs := make([]models.Job, 0, len(s.db.Jobs))
		for _, job := range s.db.Jobs {
			jobs = append(jobs, *job)
		}
		return jobs, nil
	}

	scoped := s.db.JobsByTenant[tenantID]
	jobs := make([]models.Job, 0, len(scoped))
	for _, job := range scoped {
		jobs = append(jobs, *job)
	}
	return jobs, nil
}

// GetJob 按 ID 返回作业副本。
//
// 返回副本而非内部指针：调用方可能在锁外读取该对象，直接暴露指针会造成数据竞争。
func (s *JobService) GetJob(id uint) (*models.Job, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	job, exists := s.db.Jobs[id]
	if !exists {
		return nil, errors.NotFound("job not found")
	}
	copied := *job
	return &copied, nil
}

// GetJobVisibleTo 在租户范围内按 ID 返回作业，用于防止跨租户越权访问。
func (s *JobService) GetJobVisibleTo(id, tenantID uint, isAdmin bool) (*models.Job, error) {
	job, err := s.GetJob(id)
	if err != nil {
		return nil, err
	}
	if !isAdmin && job.TenantID != tenantID {
		// 对越权访问返回 404 而不是 403，避免泄露资源是否存在。
		return nil, errors.NotFound("job not found")
	}
	return job, nil
}

func (s *JobService) CreateJob(req CreateJobRequest) (*models.Job, error) {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if req.TenantID > 0 {
		tenant, exists := s.db.Tenants[req.TenantID]
		if !exists {
			return nil, errors.NotFound("tenant not found")
		}

		if tenant.Status != "active" {
			return nil, errors.BadRequest("tenant is not active")
		}

		if req.GPUs > tenant.GPUQuota {
			return nil, errors.BadRequest(fmt.Sprintf("GPU quota exceeded: requested %d, available %d", req.GPUs, tenant.GPUQuota))
		}

		if req.CPUs > tenant.CPUQuota {
			return nil, errors.BadRequest(fmt.Sprintf("CPU quota exceeded: requested %d, available %d", req.CPUs, tenant.CPUQuota))
		}

		if req.Memory > tenant.MemoryQuota {
			return nil, errors.BadRequest(fmt.Sprintf("Memory quota exceeded: requested %d GB, available %d GB", req.Memory, tenant.MemoryQuota))
		}
	}

	priority := req.Priority
	if priority < 0 || priority > 3 {
		priority = 0
	}

	job := &models.Job{
		ID:          s.db.JobSeq,
		Name:        req.Name,
		Description: req.Description,
		Type:        req.Type,
		Status:      "pending",
		Priority:    priority,
		GPUs:        req.GPUs,
		CPUs:        req.CPUs,
		Memory:      req.Memory,
		Duration:    req.Duration,
		ClusterID:   req.ClusterID,
		TenantID:    req.TenantID,
		UserID:      req.UserID,
		Progress:    0,
		CreatedAt:   time.Now(),
		UpdatedAt:   time.Now(),
	}
	s.db.AddJobWithIndex(job)
	s.db.JobSeq++

	logger.InfoWithCtx(context.Background(), "Job created",
		"job_id", job.ID,
		"job_name", job.Name,
		"tenant_id", job.TenantID,
		"user_id", job.UserID,
		"gpus", req.GPUs,
		"cpus", req.CPUs,
		"memory", req.Memory)

	copied := *job
	return &copied, nil
}

// CreateJobForUser 以调用者身份创建作业。
//
// TenantID 与 UserID 一律取自 JWT 声明而非请求体，否则调用方可以伪造
// 这两个字段，把作业挂到其他租户名下（既污染配额统计，也造成越权可见）。
// 管理员可以通过请求体显式指定目标租户。
func (s *JobService) CreateJobForUser(req CreateJobRequest, tenantID, userID uint, isAdmin bool) (*models.Job, error) {
	if isAdmin && req.TenantID > 0 {
		tenantID = req.TenantID
	}
	req.TenantID = tenantID
	req.UserID = userID
	return s.CreateJob(req)
}

func (s *JobService) UpdateJob(id uint, req UpdateJobRequest) (*models.Job, error) {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	job, exists := s.db.Jobs[id]
	if !exists {
		return nil, errors.NotFound("job not found")
	}

	if req.Name != "" {
		job.Name = req.Name
	}
	if req.Description != "" {
		job.Description = req.Description
	}
	if req.Status != "" && req.Status != job.Status {
		s.db.UpdateJobStatus(id, req.Status)
	}
	if req.Priority != nil && *req.Priority >= 0 && *req.Priority <= 3 && *req.Priority != job.Priority {
		oldPriority := job.Priority
		s.db.UpdateJobPriority(id, *req.Priority)
		select {
		case s.db.PriorityChanged <- id:
			logger.DebugWithCtx(nil, "Priority change notification sent", "job_id", id, "old_priority", oldPriority, "new_priority", *req.Priority)
		default:
			logger.WarnWithCtx(nil, "Priority change channel full, dropping notification", "job_id", id, "old_priority", oldPriority, "new_priority", *req.Priority)
		}
	}
	if req.Progress >= 0 {
		job.Progress = req.Progress
	}
	if req.OutputPath != "" {
		job.OutputPath = req.OutputPath
	}
	if req.ErrorMsg != "" {
		job.ErrorMsg = req.ErrorMsg
	}
	job.UpdatedAt = time.Now()

	copied := *job
	return &copied, nil
}

// UpdateJobForTenant 在租户范围内更新作业。
// 越权访问返回 404，避免泄露作业是否属于其他租户。
func (s *JobService) UpdateJobForTenant(id, tenantID uint, isAdmin bool, req UpdateJobRequest) (*models.Job, error) {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	job, exists := s.db.Jobs[id]
	if !exists {
		return nil, errors.NotFound("job not found")
	}
	if !isAdmin && job.TenantID != tenantID {
		return nil, errors.NotFound("job not found")
	}
	return s.applyUpdateLocked(id, job, req)
}

// DeleteJobForTenant 在租户范围内删除作业。
func (s *JobService) DeleteJobForTenant(id, tenantID uint, isAdmin bool) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	job, exists := s.db.Jobs[id]
	if !exists {
		return errors.NotFound("job not found")
	}
	if !isAdmin && job.TenantID != tenantID {
		return errors.NotFound("job not found")
	}
	s.db.RemoveJobFromIndex(id)
	return nil
}

// CancelJobForTenant 在租户范围内取消作业。
func (s *JobService) CancelJobForTenant(id, tenantID uint, isAdmin bool) (*models.Job, error) {
	s.db.Mu.Lock()

	job, exists := s.db.Jobs[id]
	if !exists {
		s.db.Mu.Unlock()
		return nil, errors.NotFound("job not found")
	}
	if !isAdmin && job.TenantID != tenantID {
		s.db.Mu.Unlock()
		return nil, errors.NotFound("job not found")
	}

	if job.Status != "running" && job.Status != "pending" {
		s.db.Mu.Unlock()
		return nil, errors.BadRequest("only running or pending jobs can be cancelled")
	}

	clusterID := job.ClusterID

	// 必须经过 UpdateJobStatus 修改状态，否则 JobsByStatus 索引会停留在
	// 旧状态，导致按状态查询持续返回已取消的作业。
	s.db.UpdateJobStatus(id, "cancelled")
	job.UpdatedAt = time.Now()
	s.db.Mu.Unlock()

	if s.k8sService != nil {
		s.k8sService.releaseGPUs(id, clusterID)
	}

	copied := *job
	return &copied, nil
}

// applyUpdateLocked 在持有写锁的前提下应用更新，返回作业副本。
func (s *JobService) applyUpdateLocked(id uint, job *models.Job, req UpdateJobRequest) (*models.Job, error) {
	if req.Name != "" {
		job.Name = req.Name
	}
	if req.Description != "" {
		job.Description = req.Description
	}
	if req.Status != "" && req.Status != job.Status {
		s.db.UpdateJobStatus(id, req.Status)
	}
	if req.Priority != nil && *req.Priority >= 0 && *req.Priority <= 3 && *req.Priority != job.Priority {
		oldPriority := job.Priority
		s.db.UpdateJobPriority(id, *req.Priority)
		select {
		case s.db.PriorityChanged <- id:
			logger.DebugWithCtx(nil, "Priority change notification sent", "job_id", id, "old_priority", oldPriority, "new_priority", *req.Priority)
		default:
			logger.WarnWithCtx(nil, "Priority change channel full, dropping notification", "job_id", id, "old_priority", oldPriority, "new_priority", *req.Priority)
		}
	}
	if req.Progress >= 0 {
		job.Progress = req.Progress
	}
	if req.OutputPath != "" {
		job.OutputPath = req.OutputPath
	}
	if req.ErrorMsg != "" {
		job.ErrorMsg = req.ErrorMsg
	}
	job.UpdatedAt = time.Now()

	copied := *job
	return &copied, nil
}

func (s *JobService) DeleteJob(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.Jobs[id]; !exists {
		return errors.NotFound("job not found")
	}
	s.db.RemoveJobFromIndex(id)
	return nil
}

func (s *JobService) GetJobsByStatus(status string) ([]models.Job, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var jobs []models.Job
	if jobsByStatus, exists := s.db.JobsByStatus[status]; exists {
		for _, job := range jobsByStatus {
			jobs = append(jobs, *job)
		}
	}
	return jobs, nil
}

func (s *JobService) GetJobsByPriority(priority int) ([]models.Job, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var jobs []models.Job
	if jobsByPriority, exists := s.db.JobsByPriority[priority]; exists {
		for _, job := range jobsByPriority {
			jobs = append(jobs, *job)
		}
	}
	return jobs, nil
}

func (s *JobService) GetJobsByTenant(tenantID uint) ([]models.Job, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var jobs []models.Job
	if jobsByTenant, exists := s.db.JobsByTenant[tenantID]; exists {
		for _, job := range jobsByTenant {
			jobs = append(jobs, *job)
		}
	}
	return jobs, nil
}

func (s *JobService) GetJobsByCluster(clusterID uint) ([]models.Job, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var jobs []models.Job
	for _, job := range s.db.Jobs {
		if job.ClusterID == clusterID {
			jobs = append(jobs, *job)
		}
	}
	return jobs, nil
}

func (s *ResourceService) GetResourcesByStatus(status string) ([]models.Resource, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var resources []models.Resource
	if resourcesByStatus, exists := s.db.ResourcesByStatus[status]; exists {
		for _, resource := range resourcesByStatus {
			resources = append(resources, *resource)
		}
	}
	return resources, nil
}

func (s *ResourceService) GetResourcesByCluster(clusterID uint) ([]models.Resource, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var resources []models.Resource
	if resourcesByCluster, exists := s.db.ResourcesByCluster[clusterID]; exists {
		for _, resource := range resourcesByCluster {
			resources = append(resources, *resource)
		}
	}
	return resources, nil
}

func (s *ResourceService) GetResourcesByType(resourceType string) ([]models.Resource, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var resources []models.Resource
	if resourcesByType, exists := s.db.ResourcesByType[resourceType]; exists {
		for _, resource := range resourcesByType {
			resources = append(resources, *resource)
		}
	}
	return resources, nil
}

// CancelJob 取消作业，不做租户校验。仅供调度器等内部调用方使用；
// HTTP 入口必须调用 CancelJobForTenant。
func (s *JobService) CancelJob(id uint) (*models.Job, error) {
	return s.CancelJobForTenant(id, 0, true)
}
