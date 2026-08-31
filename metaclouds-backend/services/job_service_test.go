package services

import (
	"testing"

	"github.com/stretchr/testify/assert"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
)

func TestJobService_CreateJob_WithPriority(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	service := NewJobService(db, cfg, nil)

	t.Run("Create job with critical priority (3)", func(t *testing.T) {
		req := CreateJobRequest{
			Name:        "Critical Job",
			Description: "Job with highest priority",
			Type:        "training",
			Priority:    3,
			GPUs:        1,
			CPUs:        2,
			Memory:      8,
			Duration:    60,
			ClusterID:   1,
			TenantID:    1,
			UserID:      1,
		}

		job, err := service.CreateJob(req)

		assert.NoError(t, err)
		assert.NotNil(t, job)
		assert.Equal(t, 3, job.Priority)
		assert.Equal(t, "Critical Job", job.Name)
		assert.Equal(t, "pending", job.Status)
	})

	t.Run("Create job with high priority (2)", func(t *testing.T) {
		req := CreateJobRequest{
			Name:      "High Priority Job",
			Type:      "inference",
			Priority:  2,
			GPUs:      1,
			CPUs:      1,
			Memory:    4,
			Duration:  30,
			ClusterID: 1,
			TenantID:  1,
			UserID:    1,
		}

		job, err := service.CreateJob(req)

		assert.NoError(t, err)
		assert.NotNil(t, job)
		assert.Equal(t, 2, job.Priority)
	})

	t.Run("Create job with medium priority (1)", func(t *testing.T) {
		req := CreateJobRequest{
			Name:      "Medium Priority Job",
			Type:      "batch",
			Priority:  1,
			GPUs:      0,
			CPUs:      4,
			Memory:    16,
			Duration:  120,
			ClusterID: 1,
			TenantID:  1,
			UserID:    1,
		}

		job, err := service.CreateJob(req)

		assert.NoError(t, err)
		assert.NotNil(t, job)
		assert.Equal(t, 1, job.Priority)
	})

	t.Run("Create job with low priority (0)", func(t *testing.T) {
		req := CreateJobRequest{
			Name:      "Low Priority Job",
			Type:      "training",
			Priority:  0,
			GPUs:      2,
			CPUs:      4,
			Memory:    16,
			Duration:  60,
			ClusterID: 1,
			TenantID:  1,
			UserID:    1,
		}

		job, err := service.CreateJob(req)

		assert.NoError(t, err)
		assert.NotNil(t, job)
		assert.Equal(t, 0, job.Priority)
	})
}

func TestJobService_CreateJob_DefaultPriority(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	service := NewJobService(db, cfg, nil)

	req := CreateJobRequest{
		Name:        "Default Priority Job",
		Description: "Job without priority specified",
		Type:        "training",
		GPUs:        1,
		CPUs:        2,
		Memory:      8,
		Duration:    60,
		ClusterID:   1,
		TenantID:    1,
		UserID:      1,
	}

	job, err := service.CreateJob(req)

	assert.NoError(t, err)
	assert.NotNil(t, job)
	assert.Equal(t, 0, job.Priority)
}

func TestJobService_CreateJob_InvalidPriority(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	service := NewJobService(db, cfg, nil)

	testCases := []struct {
		name     string
		priority int
		expected int
	}{
		{"Negative priority", -1, 0},
		{"Priority exceeds max (4)", 4, 0},
		{"Priority exceeds max (10)", 10, 0},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			req := CreateJobRequest{
				Name:      "Invalid Priority Job",
				Type:      "training",
				Priority:  tc.priority,
				GPUs:      1,
				CPUs:      2,
				Memory:    8,
				Duration:  60,
				ClusterID: 1,
				TenantID:  1,
				UserID:    1,
			}

			job, err := service.CreateJob(req)

			assert.NoError(t, err)
			assert.NotNil(t, job)
			assert.Equal(t, tc.expected, job.Priority)
		})
	}
}

func TestJobService_GetJob(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	job := &models.Job{
		ID:          1,
		Name:        "Test Job",
		Description: "Test description",
		Status:      "pending",
		Type:        "training",
		Priority:    2,
		GPUs:        1,
		CPUs:        2,
		Memory:      8,
		Duration:    60,
		ClusterID:   1,
		TenantID:    1,
		UserID:      1,
	}

	db.Mu.Lock()
	db.Jobs[1] = job
	db.Mu.Unlock()

	service := NewJobService(db, cfg, nil)

	result, err := service.GetJob(1)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, "Test Job", result.Name)
	assert.Equal(t, 2, result.Priority)
}

func TestJobService_GetJob_NotFound(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	service := NewJobService(db, cfg, nil)

	result, err := service.GetJob(999)

	assert.Error(t, err)
	assert.Nil(t, result)
	assert.Contains(t, err.Error(), "job not found")
}

func TestJobService_GetJobs(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	job1 := &models.Job{ID: 1, Name: "Job 1", Priority: 3, Status: "running"}
	job2 := &models.Job{ID: 2, Name: "Job 2", Priority: 1, Status: "pending"}
	job3 := &models.Job{ID: 3, Name: "Job 3", Priority: 0, Status: "completed"}

	db.Mu.Lock()
	db.Jobs[1] = job1
	db.Jobs[2] = job2
	db.Jobs[3] = job3
	db.Mu.Unlock()

	service := NewJobService(db, cfg, nil)

	jobs, err := service.GetJobs()

	assert.NoError(t, err)
	assert.Len(t, jobs, 3)
}

func TestJobService_UpdateJob_PriorityChange(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	job := &models.Job{
		ID:       1,
		Name:     "Original Job",
		Status:   "pending",
		Priority: 1,
	}

	db.Mu.Lock()
	db.Jobs[1] = job
	db.JobSeq = 2
	db.Mu.Unlock()

	service := NewJobService(db, cfg, nil)

	priority := 3
	req := UpdateJobRequest{
		Name:     "Updated Job",
		Priority: &priority,
	}

	result, err := service.UpdateJob(1, req)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, "Updated Job", result.Name)
	assert.Equal(t, 3, result.Priority)
}

func TestJobService_UpdateJob_NoPriorityChange(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	job := &models.Job{
		ID:       1,
		Name:     "Job",
		Status:   "pending",
		Priority: 2,
	}

	db.Mu.Lock()
	db.Jobs[1] = job
	db.JobSeq = 2
	db.Mu.Unlock()

	service := NewJobService(db, cfg, nil)

	req := UpdateJobRequest{
		Name: "Updated Job",
	}

	result, err := service.UpdateJob(1, req)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, "Updated Job", result.Name)
	assert.Equal(t, 2, result.Priority)
}

func TestJobService_UpdateJob_InvalidPriority(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	job := &models.Job{
		ID:       1,
		Name:     "Job",
		Status:   "pending",
		Priority: 1,
	}

	db.Mu.Lock()
	db.Jobs[1] = job
	db.JobSeq = 2
	db.Mu.Unlock()

	service := NewJobService(db, cfg, nil)

	priority := 5
	req := UpdateJobRequest{
		Priority: &priority,
	}

	result, err := service.UpdateJob(1, req)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, 1, result.Priority)
}

func TestJobService_UpdateJob_PriorityChange_FromLowToHigh(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	job := &models.Job{
		ID:       1,
		Name:     "Low Priority Job",
		Status:   "pending",
		Priority: 0,
	}

	db.Mu.Lock()
	db.Jobs[1] = job
	db.JobSeq = 2
	db.Mu.Unlock()

	service := NewJobService(db, cfg, nil)

	priority := 3
	req := UpdateJobRequest{
		Priority: &priority,
	}

	result, err := service.UpdateJob(1, req)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, 3, result.Priority)
}

func TestJobService_UpdateJob_PriorityChange_FromHighToLow(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	job := &models.Job{
		ID:       1,
		Name:     "High Priority Job",
		Status:   "running",
		Priority: 3,
	}

	db.Mu.Lock()
	db.Jobs[1] = job
	db.JobSeq = 2
	db.Mu.Unlock()

	service := NewJobService(db, cfg, nil)

	priority := 0
	req := UpdateJobRequest{
		Priority: &priority,
	}

	result, err := service.UpdateJob(1, req)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, 0, result.Priority)
}

func TestJobService_UpdateJob_PriorityChange_SameValue(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	job := &models.Job{
		ID:       1,
		Name:     "Job",
		Status:   "pending",
		Priority: 2,
	}

	db.Mu.Lock()
	db.Jobs[1] = job
	db.JobSeq = 2
	db.Mu.Unlock()

	service := NewJobService(db, cfg, nil)

	priority := 2
	req := UpdateJobRequest{
		Priority: &priority,
	}

	result, err := service.UpdateJob(1, req)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, 2, result.Priority)
}

func TestJobService_UpdateJob_PriorityChange_BoundaryValues(t *testing.T) {
	cfg := &config.Config{}

	testCases := []struct {
		name        string
		initial     int
		newPriority int
		expected    int
	}{
		{"Increase to max priority", 2, 3, 3},
		{"Decrease to min priority", 1, 0, 0},
		{"Stay at min priority", 0, 0, 0},
		{"Stay at max priority", 3, 3, 3},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			db := models.MustNewMemoryStore()
			job := &models.Job{
				ID:       1,
				Name:     tc.name,
				Status:   "pending",
				Priority: tc.initial,
			}

			db.Mu.Lock()
			db.Jobs[1] = job
			db.JobSeq = 2
			db.Mu.Unlock()

			service := NewJobService(db, cfg, nil)

			newPriority := tc.newPriority
			req := UpdateJobRequest{
				Priority: &newPriority,
			}

			result, err := service.UpdateJob(1, req)

			assert.NoError(t, err)
			assert.NotNil(t, result)
			assert.Equal(t, tc.expected, result.Priority)
		})
	}
}

func TestJobService_UpdateJob_PriorityChange_WithNegativeValue(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	job := &models.Job{
		ID:       1,
		Name:     "Job",
		Status:   "pending",
		Priority: 2,
	}

	db.Mu.Lock()
	db.Jobs[1] = job
	db.JobSeq = 2
	db.Mu.Unlock()

	service := NewJobService(db, cfg, nil)

	priority := -1
	req := UpdateJobRequest{
		Priority: &priority,
	}

	result, err := service.UpdateJob(1, req)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, 2, result.Priority)
}

func TestJobService_UpdateJob_NotFound(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	service := NewJobService(db, cfg, nil)

	req := UpdateJobRequest{
		Name: "Update Non-existent",
	}

	result, err := service.UpdateJob(999, req)

	assert.Error(t, err)
	assert.Nil(t, result)
	assert.Contains(t, err.Error(), "job not found")
}

func TestJobService_DeleteJob(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	job := &models.Job{
		ID:       1,
		Name:     "To Delete",
		Priority: 2,
	}

	db.Mu.Lock()
	db.Jobs[1] = job
	db.Mu.Unlock()

	service := NewJobService(db, cfg, nil)

	err := service.DeleteJob(1)

	assert.NoError(t, err)

	db.Mu.RLock()
	_, exists := db.Jobs[1]
	db.Mu.RUnlock()
	assert.False(t, exists)
}

func TestJobService_DeleteJob_NotFound(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	service := NewJobService(db, cfg, nil)

	err := service.DeleteJob(999)

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "job not found")
}
