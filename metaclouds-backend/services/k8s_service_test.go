package services

import (
	"sync"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
)

func setupTestK8SService(t *testing.T, gpuCount int) (*K8SService, *models.MemoryStore, *config.Config) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	db.Mu.Lock()
	for i := uint(1); i <= uint(gpuCount); i++ {
		db.Resources[i] = &models.Resource{
			ID:          i,
			ClusterID:   1,
			Name:        "NVIDIA-A100",
			Type:        "gpu",
			Status:      "available",
			Total:       1,
			Used:        0,
			Available:   1,
			Utilization: 0,
		}
	}
	db.ResourceSeq = uint(gpuCount) + 1

	job := &models.Job{
		ID:        1,
		Name:      "test-job",
		Status:    "pending",
		GPUs:      2,
		ClusterID: 1,
	}
	db.Jobs[1] = job
	db.JobSeq = 2
	db.Mu.Unlock()

	service := NewK8SService(db, cfg)
	return service, db, cfg
}

func TestK8SService_AllocateGPUs_Success(t *testing.T) {
	service, db, _ := setupTestK8SService(t, 10)

	gpuIDs, err := service.allocateGPUs(1, 2)

	assert.NoError(t, err)
	assert.Len(t, gpuIDs, 2)

	db.Mu.RLock()
	for _, id := range gpuIDs {
		resource := db.Resources[id]
		assert.Equal(t, 1, resource.Used)
		assert.Equal(t, 0, resource.Available)
	}
	db.Mu.RUnlock()
}

func TestK8SService_AllocateGPUs_Insufficient(t *testing.T) {
	service, _, _ := setupTestK8SService(t, 10)

	_, err := service.allocateGPUs(1, 100)

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "not enough GPU resources available")
}

func TestK8SService_AllocateGPUs_ZeroCount(t *testing.T) {
	service, _, _ := setupTestK8SService(t, 10)

	gpuIDs, err := service.allocateGPUs(1, 0)

	assert.NoError(t, err)
	assert.Empty(t, gpuIDs)
}

func TestK8SService_ReleaseGPUs_Success(t *testing.T) {
	service, db, _ := setupTestK8SService(t, 10)

	service.allocatedMu.Lock()
	service.allocated[1] = []uint{1, 2}
	service.allocatedMu.Unlock()

	db.Mu.Lock()
	db.Resources[1].Used = 1
	db.Resources[1].Available = 0
	db.Resources[2].Used = 1
	db.Resources[2].Available = 0
	db.Mu.Unlock()

	service.releaseGPUs(1, 1)

	db.Mu.RLock()
	assert.Equal(t, 0, db.Resources[1].Used)
	assert.Equal(t, 1, db.Resources[1].Available)
	assert.Equal(t, 0, db.Resources[2].Used)
	assert.Equal(t, 1, db.Resources[2].Available)
	db.Mu.RUnlock()

	service.allocatedMu.RLock()
	_, exists := service.allocated[1]
	service.allocatedMu.RUnlock()
	assert.False(t, exists)
}

func TestK8SService_ReleaseGPUs_NotFound(t *testing.T) {
	service, _, _ := setupTestK8SService(t, 10)

	assert.NotPanics(t, func() {
		service.releaseGPUs(999, 1)
	})
}

func TestK8SService_SubmitJob_Success(t *testing.T) {
	service, db, _ := setupTestK8SService(t, 10)

	req := SubmitJobRequest{JobID: 1}
	resp, err := service.SubmitJob(req)

	assert.NoError(t, err)
	assert.NotNil(t, resp)
	assert.Equal(t, "running", resp.Status)

	db.Mu.RLock()
	assert.Equal(t, "running", db.Jobs[1].Status)
	db.Mu.RUnlock()
}

func TestK8SService_SubmitJob_NotFound(t *testing.T) {
	service, _, _ := setupTestK8SService(t, 10)

	req := SubmitJobRequest{JobID: 999}
	resp, err := service.SubmitJob(req)

	assert.Error(t, err)
	assert.Nil(t, resp)
	assert.Contains(t, err.Error(), "job not found")
}

func TestK8SService_SubmitJob_AlreadyRunning(t *testing.T) {
	service, db, _ := setupTestK8SService(t, 10)

	db.Mu.Lock()
	db.Jobs[1].Status = "running"
	db.Mu.Unlock()

	req := SubmitJobRequest{JobID: 1}
	resp, err := service.SubmitJob(req)

	assert.NoError(t, err)
	assert.NotNil(t, resp)
	assert.Equal(t, "running", resp.Status)
	assert.Contains(t, resp.Message, "already running")
}

func TestK8SService_CancelJob_Success(t *testing.T) {
	service, db, _ := setupTestK8SService(t, 10)

	db.Mu.Lock()
	db.Jobs[1].Status = "running"
	db.Resources[1].Used = 1
	db.Resources[1].Available = 0
	db.Mu.Unlock()

	service.allocatedMu.Lock()
	service.allocated[1] = []uint{1}
	service.allocatedMu.Unlock()

	resp, err := service.CancelJob(1)

	assert.NoError(t, err)
	assert.NotNil(t, resp)
	assert.Equal(t, "cancelled", resp.Status)

	db.Mu.RLock()
	assert.Equal(t, "cancelled", db.Jobs[1].Status)
	assert.Equal(t, 0, db.Resources[1].Used)
	assert.Equal(t, 1, db.Resources[1].Available)
	db.Mu.RUnlock()
}

func TestK8SService_CancelJob_NotFound(t *testing.T) {
	service, _, _ := setupTestK8SService(t, 10)

	resp, err := service.CancelJob(999)

	assert.Error(t, err)
	assert.Nil(t, resp)
	assert.Contains(t, err.Error(), "job not found")
}

func TestK8SService_CancelJob_InvalidStatus(t *testing.T) {
	service, db, _ := setupTestK8SService(t, 10)

	db.Mu.Lock()
	db.Jobs[1].Status = "completed"
	db.Mu.Unlock()

	resp, err := service.CancelJob(1)

	assert.Error(t, err)
	assert.Nil(t, resp)
	assert.Contains(t, err.Error(), "only running or pending jobs can be cancelled")
}

func TestK8SService_ConcurrentSubmitJobs(t *testing.T) {
	service, db, _ := setupTestK8SService(t, 10)

	db.Mu.Lock()
	for i := uint(2); i <= 5; i++ {
		db.Jobs[i] = &models.Job{
			ID:        i,
			Name:      "test-job",
			Status:    "pending",
			GPUs:      2,
			ClusterID: 1,
		}
	}
	db.JobSeq = 6
	db.Mu.Unlock()

	var wg sync.WaitGroup
	errors := make(chan error, 5)

	for i := uint(1); i <= 5; i++ {
		wg.Add(1)
		go func(jobID uint) {
			defer wg.Done()
			req := SubmitJobRequest{JobID: jobID}
			_, err := service.SubmitJob(req)
			if err != nil {
				errors <- err
			}
		}(i)
	}

	wg.Wait()
	close(errors)

	var errs []error
	for err := range errors {
		errs = append(errs, err)
	}

	assert.Len(t, errs, 0, "Expected no errors in concurrent submissions")

	db.Mu.RLock()
	usedCount := 0
	for _, resource := range db.Resources {
		usedCount += resource.Used
	}
	db.Mu.RUnlock()

	assert.LessOrEqual(t, usedCount, 10, "Total used GPUs should not exceed available")
}

func TestK8SService_ConcurrentSubmitAndCancel(t *testing.T) {
	service, db, _ := setupTestK8SService(t, 10)

	db.Mu.Lock()
	db.Jobs[1].GPUs = 1
	for i := uint(2); i <= 10; i++ {
		db.Jobs[i] = &models.Job{
			ID:        i,
			Name:      "test-job",
			Status:    "pending",
			GPUs:      1,
			ClusterID: 1,
		}
	}
	db.JobSeq = 11
	db.Mu.Unlock()

	var wg sync.WaitGroup

	for i := uint(1); i <= 10; i++ {
		wg.Add(1)
		go func(jobID uint) {
			defer wg.Done()
			req := SubmitJobRequest{JobID: jobID}
			service.SubmitJob(req)
		}(i)
	}

	wg.Wait()

	for i := uint(1); i <= 10; i++ {
		wg.Add(1)
		go func(jobID uint) {
			defer wg.Done()
			service.CancelJob(jobID)
		}(i)
	}

	wg.Wait()

	db.Mu.RLock()
	totalUsed := 0
	totalAvailable := 0
	for _, resource := range db.Resources {
		totalUsed += resource.Used
		totalAvailable += resource.Available
	}
	db.Mu.RUnlock()

	assert.Equal(t, 0, totalUsed, "All GPUs should be released")
	assert.Equal(t, 40, totalAvailable, "All 40 GPUs should be available")
}

func TestK8SService_NoDeadlock(t *testing.T) {
	service, db, _ := setupTestK8SService(t, 10)

	db.Mu.Lock()
	db.Jobs[1].Status = "running"
	db.Resources[1].Used = 1
	db.Resources[1].Available = 0
	db.Mu.Unlock()

	service.allocatedMu.Lock()
	service.allocated[1] = []uint{1}
	service.allocatedMu.Unlock()

	done := make(chan struct{})
	go func() {
		service.releaseGPUs(1, 1)
		close(done)
	}()

	select {
	case <-done:
	case <-time.After(5 * time.Second):
		t.Fatal("Deadlock detected: releaseGPUs did not complete within timeout")
	}
}

func TestK8SService_SimulateJobProgress_NoDeadlock(t *testing.T) {
	service, db, _ := setupTestK8SService(t, 10)

	db.Mu.Lock()
	db.Jobs[1].Status = "running"
	db.Jobs[1].Progress = 95
	db.Mu.Unlock()

	done := make(chan struct{})
	go func() {
		service.simulateJobProgress(1)
		close(done)
	}()

	select {
	case <-done:
	case <-time.After(30 * time.Second):
		t.Fatal("Deadlock detected: simulateJobProgress did not complete within timeout")
	}
}

func TestK8SService_ReleaseGPUs_Idempotent(t *testing.T) {
	service, db, _ := setupTestK8SService(t, 10)

	db.Mu.Lock()
	db.Resources[1].Used = 1
	db.Resources[1].Available = 0
	db.Mu.Unlock()

	service.allocatedMu.Lock()
	service.allocated[1] = []uint{1}
	service.allocatedMu.Unlock()

	service.releaseGPUs(1, 1)

	assert.NotPanics(t, func() {
		service.releaseGPUs(1, 1)
	})

	db.Mu.RLock()
	assert.Equal(t, 0, db.Resources[1].Used)
	assert.Equal(t, 1, db.Resources[1].Available)
	db.Mu.RUnlock()
}
