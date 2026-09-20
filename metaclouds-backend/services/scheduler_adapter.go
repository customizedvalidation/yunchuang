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

// SchedulerAdapter 外部调度器适配器接口
type SchedulerAdapter interface {
	Type() string
	SubmitJob(job *models.Job) (schedulerJobID string, err error)
	CancelJob(schedulerJobID string) error
	GetJobStatus(schedulerJobID string) (status string, err error)
	GetQueueInfo() ([]QueueInfo, error)
	GetNodeInfo() ([]NodeInfo, error)
	SyncJobs() ([]JobSyncResult, error)
	HealthCheck() (bool, error)
}

// QueueInfo 队列信息
type QueueInfo struct {
	Name           string `json:"name"`
	State          string `json:"state"`
	TotalNodes     int    `json:"total_nodes"`
	AllocatedNodes int    `json:"allocated_nodes"`
	TotalGPUs      int    `json:"total_gpus"`
	AllocatedGPUs  int    `json:"allocated_gpus"`
	PendingJobs    int    `json:"pending_jobs"`
	RunningJobs    int    `json:"running_jobs"`
}

// NodeInfo 节点信息
type NodeInfo struct {
	Name       string   `json:"name"`
	State      string   `json:"state"`
	Partitions []string `json:"partitions"`
	CPUs       int      `json:"cpus"`
	GPUs       int      `json:"gpus"`
	Memory     int      `json:"memory"`
	GRES       string   `json:"gres"`
}

// JobSyncResult 作业同步结果
type JobSyncResult struct {
	SchedulerJobID string    `json:"scheduler_job_id"`
	Status         string    `json:"status"`
	StartTime      time.Time `json:"start_time"`
	EndTime        time.Time `json:"end_time"`
	ErrorMsg       string    `json:"error_msg"`
}

// SchedulerService 调度器集成管理服务
type SchedulerService struct {
	db     *models.MemoryStore
	config *config.Config
}

func NewSchedulerService(db interface{}, config *config.Config) *SchedulerService {
	memoryStore, err := models.GetDBStore(db, "SchedulerService")
	if err != nil {
		logger.ErrorWithCtx(context.Background(), "Failed to initialize SchedulerService", err)
		return nil
	}
	return &SchedulerService{
		db:     memoryStore,
		config: config,
	}
}

// NewSchedulerAdapter 工厂函数：根据集成配置创建对应调度器适配器
func NewSchedulerAdapter(integration *models.SchedulerIntegration) (SchedulerAdapter, error) {
	switch integration.Type {
	case "slurm":
		return &SlurmAdapter{integration: integration}, nil
	case "k8s_native":
		return &K8sNativeAdapter{integration: integration}, nil
	case "lsf", "sge":
		// 返回通用模拟适配器
		return &GenericSchedulerAdapter{integration: integration}, nil
	default:
		return nil, mcerrors.BadRequest(fmt.Sprintf("unsupported scheduler type: %s", integration.Type))
	}
}

// --- SchedulerIntegration CRUD ---

func (s *SchedulerService) ListSchedulerIntegrations() ([]models.SchedulerIntegration, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var integrations []models.SchedulerIntegration
	for _, i := range s.db.SchedulerIntegrations {
		integrations = append(integrations, *i)
	}
	return integrations, nil
}

func (s *SchedulerService) GetSchedulerIntegration(id uint) (*models.SchedulerIntegration, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	integration, exists := s.db.SchedulerIntegrations[id]
	if !exists {
		return nil, mcerrors.NotFound("scheduler integration not found")
	}
	return integration, nil
}

func (s *SchedulerService) CreateSchedulerIntegration(integration *models.SchedulerIntegration) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	for _, i := range s.db.SchedulerIntegrations {
		if i.Name == integration.Name {
			return mcerrors.Conflict("scheduler integration name already exists")
		}
	}

	integration.ID = s.db.SchedulerIntegrationSeq
	integration.CreatedAt = time.Now()
	integration.UpdatedAt = time.Now()
	if integration.Status == "" {
		integration.Status = "inactive"
	}
	if integration.AuthType == "" {
		integration.AuthType = "none"
	}
	s.db.SchedulerIntegrations[s.db.SchedulerIntegrationSeq] = integration
	s.db.SchedulerIntegrationSeq++
	return nil
}

func (s *SchedulerService) UpdateSchedulerIntegration(integration *models.SchedulerIntegration) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	existing, exists := s.db.SchedulerIntegrations[integration.ID]
	if !exists {
		return mcerrors.NotFound("scheduler integration not found")
	}

	if integration.Name != "" {
		existing.Name = integration.Name
	}
	if integration.Type != "" {
		existing.Type = integration.Type
	}
	if integration.Endpoint != "" {
		existing.Endpoint = integration.Endpoint
	}
	if integration.AuthType != "" {
		existing.AuthType = integration.AuthType
	}
	if integration.AuthConfig != "" {
		existing.AuthConfig = integration.AuthConfig
	}
	if integration.Version != "" {
		existing.Version = integration.Version
	}
	if integration.Status != "" {
		existing.Status = integration.Status
	}
	if integration.DefaultPartition != "" {
		existing.DefaultPartition = integration.DefaultPartition
	}
	if integration.DefaultQoS != "" {
		existing.DefaultQoS = integration.DefaultQoS
	}
	if integration.MaxNodes >= 0 {
		existing.MaxNodes = integration.MaxNodes
	}
	if integration.MaxJobs >= 0 {
		existing.MaxJobs = integration.MaxJobs
	}
	if integration.Details != "" {
		existing.Details = integration.Details
	}
	existing.UpdatedAt = time.Now()
	return nil
}

func (s *SchedulerService) DeleteSchedulerIntegration(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.SchedulerIntegrations[id]; !exists {
		return mcerrors.NotFound("scheduler integration not found")
	}
	delete(s.db.SchedulerIntegrations, id)
	return nil
}

func (s *SchedulerService) GetQueues(id uint) ([]QueueInfo, error) {
	integration, err := s.GetSchedulerIntegration(id)
	if err != nil {
		return nil, err
	}
	adapter, err := NewSchedulerAdapter(integration)
	if err != nil {
		return nil, err
	}
	return adapter.GetQueueInfo()
}

func (s *SchedulerService) GetNodes(id uint) ([]NodeInfo, error) {
	integration, err := s.GetSchedulerIntegration(id)
	if err != nil {
		return nil, err
	}
	adapter, err := NewSchedulerAdapter(integration)
	if err != nil {
		return nil, err
	}
	return adapter.GetNodeInfo()
}

func (s *SchedulerService) SyncJobs(id uint) ([]JobSyncResult, error) {
	integration, err := s.GetSchedulerIntegration(id)
	if err != nil {
		return nil, err
	}
	adapter, err := NewSchedulerAdapter(integration)
	if err != nil {
		return nil, err
	}

	results, err := adapter.SyncJobs()
	if err != nil {
		return nil, err
	}

	// 更新 last_sync_at
	s.db.Mu.Lock()
	now := time.Now()
	integration.LastSyncAt = &now
	integration.UpdatedAt = now
	s.db.Mu.Unlock()

	return results, nil
}

func (s *SchedulerService) HealthCheck(id uint) (bool, error) {
	integration, err := s.GetSchedulerIntegration(id)
	if err != nil {
		return false, err
	}
	adapter, err := NewSchedulerAdapter(integration)
	if err != nil {
		return false, err
	}
	return adapter.HealthCheck()
}

// --- SlurmAdapter 实现（配置驱动的模拟结果） ---

type SlurmAdapter struct {
	integration *models.SchedulerIntegration
}

func (a *SlurmAdapter) Type() string { return "slurm" }

func (a *SlurmAdapter) SubmitJob(job *models.Job) (string, error) {
	// 模拟 sbatch 提交，返回配置驱动的 job ID
	schedulerJobID := fmt.Sprintf("slurm-%d-%d", a.integration.ID, job.ID)
	logger.InfoWithCtx(context.Background(), "SlurmAdapter: simulated job submit",
		"scheduler_job_id", schedulerJobID,
		"job_name", job.Name,
		"partition", a.integration.DefaultPartition)
	return schedulerJobID, nil
}

func (a *SlurmAdapter) CancelJob(schedulerJobID string) error {
	logger.InfoWithCtx(context.Background(), "SlurmAdapter: simulated job cancel", "scheduler_job_id", schedulerJobID)
	return nil
}

func (a *SlurmAdapter) GetJobStatus(schedulerJobID string) (string, error) {
	return "running", nil
}

func (a *SlurmAdapter) GetQueueInfo() ([]QueueInfo, error) {
	return []QueueInfo{
		{
			Name:           a.integration.DefaultPartition,
			State:          "up",
			TotalNodes:     a.integration.MaxNodes,
			AllocatedNodes: a.integration.MaxNodes / 2,
			TotalGPUs:      a.integration.MaxNodes * 8,
			AllocatedGPUs:  a.integration.MaxNodes * 4,
			PendingJobs:    3,
			RunningJobs:    12,
		},
	}, nil
}

func (a *SlurmAdapter) GetNodeInfo() ([]NodeInfo, error) {
	nodes := make([]NodeInfo, 0)
	for i := 0; i < a.integration.MaxNodes && i < 5; i++ {
		nodes = append(nodes, NodeInfo{
			Name:       fmt.Sprintf("node%03d", i+1),
			State:      "idle",
			Partitions: []string{a.integration.DefaultPartition},
			CPUs:       64,
			GPUs:       8,
			Memory:     512,
			GRES:       "gpu:8",
		})
	}
	return nodes, nil
}

func (a *SlurmAdapter) SyncJobs() ([]JobSyncResult, error) {
	return []JobSyncResult{
		{
			SchedulerJobID: fmt.Sprintf("slurm-%d-1", a.integration.ID),
			Status:         "running",
			StartTime:      time.Now().Add(-time.Hour),
		},
	}, nil
}

func (a *SlurmAdapter) HealthCheck() (bool, error) {
	return a.integration.Status == "active", nil
}

// --- K8sNativeAdapter 实现 ---

type K8sNativeAdapter struct {
	integration *models.SchedulerIntegration
}

func (a *K8sNativeAdapter) Type() string { return "k8s_native" }

func (a *K8sNativeAdapter) SubmitJob(job *models.Job) (string, error) {
	return fmt.Sprintf("k8s-%d", job.ID), nil
}

func (a *K8sNativeAdapter) CancelJob(schedulerJobID string) error {
	return nil
}

func (a *K8sNativeAdapter) GetJobStatus(schedulerJobID string) (string, error) {
	return "running", nil
}

func (a *K8sNativeAdapter) GetQueueInfo() ([]QueueInfo, error) {
	return []QueueInfo{
		{
			Name:           "default",
			State:          "up",
			TotalNodes:     a.integration.MaxNodes,
			AllocatedNodes: 0,
			TotalGPUs:      0,
			AllocatedGPUs:  0,
			PendingJobs:    0,
			RunningJobs:    0,
		},
	}, nil
}

func (a *K8sNativeAdapter) GetNodeInfo() ([]NodeInfo, error) {
	return []NodeInfo{}, nil
}

func (a *K8sNativeAdapter) SyncJobs() ([]JobSyncResult, error) {
	return []JobSyncResult{}, nil
}

func (a *K8sNativeAdapter) HealthCheck() (bool, error) {
	return true, nil
}

// --- GenericSchedulerAdapter（LSF/SGE 通用模拟） ---

type GenericSchedulerAdapter struct {
	integration *models.SchedulerIntegration
}

func (a *GenericSchedulerAdapter) Type() string { return a.integration.Type }

func (a *GenericSchedulerAdapter) SubmitJob(job *models.Job) (string, error) {
	return fmt.Sprintf("%s-%d", a.integration.Type, job.ID), nil
}

func (a *GenericSchedulerAdapter) CancelJob(schedulerJobID string) error { return nil }

func (a *GenericSchedulerAdapter) GetJobStatus(schedulerJobID string) (string, error) {
	return "running", nil
}

func (a *GenericSchedulerAdapter) GetQueueInfo() ([]QueueInfo, error) {
	return []QueueInfo{}, nil
}

func (a *GenericSchedulerAdapter) GetNodeInfo() ([]NodeInfo, error) {
	return []NodeInfo{}, nil
}

func (a *GenericSchedulerAdapter) SyncJobs() ([]JobSyncResult, error) {
	return []JobSyncResult{}, nil
}

func (a *GenericSchedulerAdapter) HealthCheck() (bool, error) {
	return a.integration.Status == "active", nil
}
