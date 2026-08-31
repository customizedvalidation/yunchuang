package services

import (
	"context"
	"fmt"
	"math/rand"
	"metaclouds-backend/config"
	"metaclouds-backend/models"
	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/logger"
	"sync"
	"time"
)

type K8SService struct {
	db          *models.MemoryStore
	config      *config.Config
	allocated   map[uint][]uint
	allocatedMu sync.RWMutex
}

func NewK8SService(db interface{}, cfg *config.Config) *K8SService {
	memoryStore, err := models.GetDBStore(db, "K8SService")
	if err != nil {
		logger.ErrorWithCtx(nil, "Failed to initialize K8SService", err)
		return nil
	}
	return &K8SService{
		db:        memoryStore,
		config:    cfg,
		allocated: make(map[uint][]uint),
	}
}

type SubmitJobRequest struct {
	JobID uint `json:"job_id"`
}

type JobStatusResponse struct {
	JobID   uint   `json:"job_id"`
	Status  string `json:"status"`
	Message string `json:"message"`
	Pods    []Pod  `json:"pods,omitempty"`
}

type Pod struct {
	Name   string `json:"name"`
	Status string `json:"status"`
	Node   string `json:"node"`
	GPUs   int    `json:"gpus"`
}

func (s *K8SService) SubmitJob(req SubmitJobRequest) (*JobStatusResponse, error) {
	s.db.Mu.Lock()

	job, exists := s.db.Jobs[req.JobID]
	if !exists {
		s.db.Mu.Unlock()
		return nil, errors.NotFound("job not found")
	}

	if job.Status == "running" {
		s.db.Mu.Unlock()
		return &JobStatusResponse{
			JobID:   job.ID,
			Status:  job.Status,
			Message: fmt.Sprintf("Job %s is already running", job.Name),
		}, nil
	}

	clusterID := job.ClusterID
	gpuCount := job.GPUs
	jobName := job.Name

	var gpuIDs []uint
	var err error

	// 先收集所有可用 GPU 的候选 ID，再统一扣减。原先是边遍历边扣减，
	// 一旦候选不足会在返回错误前已扣掉一部分 GPU，且从未回滚，造成 GPU
	// 被悄悄泄漏（永远不可用）。
	var candidates []uint
	for id, resource := range s.db.Resources {
		if resource.ClusterID == clusterID && resource.Type == "gpu" && resource.Available > 0 {
			candidates = append(candidates, id)
		}
	}
	if len(candidates) < gpuCount {
		s.db.Mu.Unlock()
		err = fmt.Errorf("not enough GPU resources available: requested %d, got %d", gpuCount, len(candidates))
		logger.ErrorWithCtx(context.Background(), "SubmitJob failed - insufficient resources", err, "cluster_id", clusterID, "requested", gpuCount, "available", len(candidates))
		return nil, err
	}

	gpuIDs = candidates[:gpuCount]
	for _, id := range gpuIDs {
		s.db.Resources[id].Available--
		s.db.Resources[id].Used++
		s.db.Resources[id].Utilization = float64(s.db.Resources[id].Used) / float64(s.db.Resources[id].Total) * 100
		if s.db.Resources[id].Utilization > 90 {
			s.db.Resources[id].Status = "high_utilization"
		}
	}

	s.allocatedMu.Lock()
	s.allocated[req.JobID] = gpuIDs
	s.allocatedMu.Unlock()

	job.Status = "running"
	job.UpdatedAt = time.Now()
	s.db.Mu.Unlock()

	go s.simulateJobProgress(req.JobID)

	gpuAllocatedCounter.WithLabelValues(fmt.Sprintf("%d", clusterID)).Add(float64(gpuCount))
	logger.InfoWithCtx(context.Background(), "Job submitted to K8S",
		"job_id", req.JobID,
		"job_name", jobName,
		"gpu_count", gpuCount,
		"gpu_ids", gpuIDs)

	return &JobStatusResponse{
		JobID:   req.JobID,
		Status:  "running",
		Message: fmt.Sprintf("Job %s submitted to K8S successfully with %d GPUs", jobName, gpuCount),
		Pods: []Pod{
			{
				Name:   fmt.Sprintf("job-pod-%d", req.JobID),
				Status: "Running",
				Node:   "gpu-node-1",
				GPUs:   gpuCount,
			},
		},
	}, nil
}

func (s *K8SService) GetJobStatus(jobID uint) (*JobStatusResponse, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	job, exists := s.db.Jobs[jobID]
	if !exists {
		return nil, errors.NotFound("job not found")
	}

	return &JobStatusResponse{
		JobID:   job.ID,
		Status:  job.Status,
		Message: fmt.Sprintf("Job %s is currently %s (Progress: %d%%)", job.Name, job.Status, job.Progress),
		Pods: []Pod{
			{
				Name:   fmt.Sprintf("job-pod-%d", job.ID),
				Status: job.Status,
				Node:   "gpu-node-1",
				GPUs:   job.GPUs,
			},
		},
	}, nil
}

func (s *K8SService) CancelJob(jobID uint) (*JobStatusResponse, error) {
	s.db.Mu.Lock()

	job, exists := s.db.Jobs[jobID]
	if !exists {
		s.db.Mu.Unlock()
		return nil, errors.NotFound("job not found")
	}

	if job.Status != "running" && job.Status != "pending" {
		s.db.Mu.Unlock()
		return nil, errors.BadRequest("only running or pending jobs can be cancelled")
	}

	job.Status = "cancelled"
	job.UpdatedAt = time.Now()
	clusterID := job.ClusterID
	gpuCount := job.GPUs
	jobName := job.Name

	s.allocatedMu.Lock()
	gpuIDs, exists := s.allocated[jobID]
	if exists {
		delete(s.allocated, jobID)
		s.allocatedMu.Unlock()

		for _, gpuID := range gpuIDs {
			if resource, ok := s.db.Resources[gpuID]; ok {
				if resource.Used > 0 {
					resource.Used--
					resource.Available++
					resource.Utilization = float64(resource.Used) / float64(resource.Total) * 100
					if resource.Utilization < 90 && resource.Status == "high_utilization" {
						resource.Status = "available"
					}
					resource.UpdatedAt = time.Now()
				}
			}
		}

		gpuReleasedCounter.WithLabelValues(fmt.Sprintf("%d", clusterID)).Add(float64(len(gpuIDs)))
		logger.InfoWithCtx(context.Background(), "GPUs released", "job_id", jobID, "cluster_id", clusterID, "gpu_count", len(gpuIDs), "gpu_ids", gpuIDs)
	} else {
		s.allocatedMu.Unlock()
	}

	s.db.Mu.Unlock()

	logger.InfoWithCtx(context.Background(), "Job cancelled",
		"job_id", jobID,
		"job_name", jobName,
		"gpu_count", gpuCount)

	return &JobStatusResponse{
		JobID:   jobID,
		Status:  "cancelled",
		Message: fmt.Sprintf("Job %s cancelled successfully", jobName),
	}, nil
}

type GPUResource struct {
	ID          uint    `json:"id"`
	Name        string  `json:"gpuName"`
	Type        string  `json:"type"`
	Status      string  `json:"status"`
	Total       int     `json:"total"`
	Used        int     `json:"used"`
	Available   int     `json:"available"`
	Utilization float64 `json:"utilization"`
	Details     string  `json:"details"`
	AllocatedTo uint    `json:"allocated_to,omitempty"`
}

func (s *K8SService) GetGPUResources() ([]GPUResource, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var resources []GPUResource
	for _, resource := range s.db.Resources {
		if resource.Type == "gpu" {
			gpuResource := GPUResource{
				ID:          resource.ID,
				Name:        resource.Name,
				Type:        resource.Type,
				Status:      resource.Status,
				Total:       resource.Total,
				Used:        resource.Used,
				Available:   resource.Available,
				Utilization: resource.Utilization,
				Details:     resource.Details,
			}
			resources = append(resources, gpuResource)
		}
	}

	if len(resources) == 0 {
		resources = []GPUResource{
			{
				ID:          1,
				Name:        "NVIDIA-A100-0",
				Type:        "gpu",
				Status:      "available",
				Total:       1,
				Used:        0,
				Available:   1,
				Utilization: 0,
				Details:     "NVIDIA A100 80GB",
			},
		}
	}

	return resources, nil
}

func (s *K8SService) allocateGPUs(clusterID uint, count int) ([]uint, error) {
	if count == 0 {
		logger.DebugWithCtx(context.Background(), "allocateGPUs called with zero count, returning empty", "cluster_id", clusterID)
		return []uint{}, nil
	}

	logger.DebugWithCtx(context.Background(), "allocateGPUs starting", "cluster_id", clusterID, "requested_count", count)

	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	// 先收集所有可用 GPU 的候选 ID，再统一扣减。原先是边遍历边扣减，
	// 一旦候选不足会在返回错误前已扣掉一部分 GPU，且从未回滚，造成 GPU
	// 被悄悄泄漏（永远不可用）。
	var candidates []uint
	for id, resource := range s.db.Resources {
		if resource.ClusterID == clusterID && resource.Type == "gpu" && resource.Available > 0 {
			candidates = append(candidates, id)
		}
	}
	logger.DebugWithCtx(context.Background(), "allocateGPUs found available GPUs", "cluster_id", clusterID, "available_count", len(candidates), "available_ids", candidates)

	if len(candidates) < count {
		err := fmt.Errorf("not enough GPU resources available: requested %d, got %d", count, len(candidates))
		logger.ErrorWithCtx(context.Background(), "allocateGPUs failed - insufficient resources", err, "cluster_id", clusterID, "requested", count, "available", len(candidates))
		return nil, err
	}

	allocated := candidates[:count]
	for _, id := range allocated {
		resource := s.db.Resources[id]
		resource.Available--
		resource.Used++
		resource.Utilization = float64(resource.Used) / float64(resource.Total) * 100
		if resource.Utilization > 90 {
			resource.Status = "high_utilization"
			logger.DebugWithCtx(context.Background(), "allocateGPUs GPU utilization exceeded 90%", "gpu_id", id, "utilization", resource.Utilization)
		}
		resource.UpdatedAt = time.Now()
		logger.DebugWithCtx(context.Background(), "allocateGPUs allocated GPU", "gpu_id", id, "cluster_id", clusterID, "remaining_available", resource.Available)
	}

	logger.InfoWithCtx(context.Background(), "allocateGPUs completed successfully", "cluster_id", clusterID, "requested", count, "allocated", len(allocated), "gpu_ids", allocated)
	return allocated, nil
}

func (s *K8SService) releaseGPUs(jobID uint, clusterID uint) {
	logger.DebugWithCtx(context.Background(), "releaseGPUs starting", "job_id", jobID, "cluster_id", clusterID)

	s.allocatedMu.Lock()
	gpuIDs, exists := s.allocated[jobID]
	if !exists {
		s.allocatedMu.Unlock()
		logger.WarnWithCtx(context.Background(), "releaseGPUs job not found in allocated map", "job_id", jobID, "cluster_id", clusterID)
		return
	}
	count := len(gpuIDs)
	logger.DebugWithCtx(context.Background(), "releaseGPUs found allocated GPUs", "job_id", jobID, "cluster_id", clusterID, "gpu_count", count, "gpu_ids", gpuIDs)
	delete(s.allocated, jobID)
	s.allocatedMu.Unlock()

	logger.InfoWithCtx(context.Background(), "releaseGPUs releasing GPUs", "job_id", jobID, "cluster_id", clusterID, "gpu_count", count, "gpu_ids", gpuIDs)
	gpuReleasedCounter.WithLabelValues(fmt.Sprintf("%d", clusterID)).Add(float64(count))

	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	successCount := 0
	failedCount := 0
	for _, gpuID := range gpuIDs {
		if resource, ok := s.db.Resources[gpuID]; ok {
			if resource.Used > 0 {
				oldUsed := resource.Used
				oldAvailable := resource.Available
				resource.Used--
				resource.Available++
				resource.Utilization = float64(resource.Used) / float64(resource.Total) * 100
				if resource.Utilization < 90 && resource.Status == "high_utilization" {
					resource.Status = "available"
					logger.DebugWithCtx(context.Background(), "releaseGPUs GPU status changed to available", "gpu_id", gpuID, "utilization", resource.Utilization)
				}
				resource.UpdatedAt = time.Now()
				successCount++
				logger.DebugWithCtx(context.Background(), "releaseGPUs released GPU successfully",
					"gpu_id", gpuID,
					"cluster_id", clusterID,
					"job_id", jobID,
					"used_before", oldUsed, "used_after", resource.Used,
					"available_before", oldAvailable, "available_after", resource.Available)
			} else {
				logger.WarnWithCtx(context.Background(), "releaseGPUs GPU already has zero used count", "gpu_id", gpuID, "job_id", jobID)
				failedCount++
			}
		} else {
			err := fmt.Errorf("GPU resource %d not found", gpuID)
			logger.ErrorWithCtx(context.Background(), "releaseGPUs GPU resource not found", err, "gpu_id", gpuID, "job_id", jobID)
			failedCount++
		}
	}

	logger.InfoWithCtx(context.Background(), "releaseGPUs completed",
		"job_id", jobID,
		"cluster_id", clusterID,
		"total_requested", count,
		"success_count", successCount,
		"failed_count", failedCount)
}

func (s *K8SService) simulateJobProgress(jobID uint) {
	ticker := time.NewTicker(5 * time.Second)
	defer ticker.Stop()

	firstTick := make(chan struct{}, 1)
	firstTick <- struct{}{}

	for {
		select {
		case <-firstTick:
		case <-ticker.C:
		}

		s.db.Mu.Lock()
		currentJob, exists := s.db.Jobs[jobID]
		if !exists || currentJob.Status != "running" {
			s.db.Mu.Unlock()
			return
		}

		increase := rand.Intn(10) + 5
		currentJob.Progress += increase
		if currentJob.Progress >= 100 {
			currentJob.Progress = 100
			currentJob.Status = "completed"
			clusterID := currentJob.ClusterID
			jobName := currentJob.Name
			s.db.Mu.Unlock()
			s.releaseGPUs(jobID, clusterID)
			logger.InfoWithCtx(context.Background(), "Job completed",
				"job_id", jobID,
				"job_name", jobName)
			return
		}
		currentJob.UpdatedAt = time.Now()
		s.db.Mu.Unlock()
	}
}

type ClusterStatus struct {
	ID          uint    `json:"id"`
	Name        string  `json:"name"`
	Status      string  `json:"status"`
	Nodes       int     `json:"nodes"`
	GPUsTotal   int     `json:"gpus_total"`
	GPUsUsed    int     `json:"gpus_used"`
	GPUsFree    int     `json:"gpus_free"`
	CPUUsage    float64 `json:"cpu_usage"`
	MemoryUsage float64 `json:"memory_usage"`
}

func (s *K8SService) GetClusterStatus(clusterID uint) (*ClusterStatus, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	cluster, exists := s.db.Clusters[clusterID]
	if !exists {
		return nil, errors.NotFound("cluster not found")
	}

	var totalGPUs, usedGPUs int
	for _, resource := range s.db.Resources {
		if resource.ClusterID == clusterID && resource.Type == "gpu" {
			totalGPUs += resource.Total
			usedGPUs += resource.Used
		}
	}

	return &ClusterStatus{
		ID:          cluster.ID,
		Name:        cluster.Name,
		Status:      cluster.Status,
		Nodes:       cluster.Nodes,
		GPUsTotal:   totalGPUs,
		GPUsUsed:    usedGPUs,
		GPUsFree:    totalGPUs - usedGPUs,
		CPUUsage:    float64(cluster.CPUs) * 0.5,
		MemoryUsage: float64(cluster.Memory) * 0.6,
	}, nil
}
