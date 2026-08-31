package services

import (
	"sync/atomic"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
)

func TestPriorityScheduling_NotificationOnChange(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	jobService := NewJobService(db, cfg, nil)

	job := &models.Job{
		ID:       1,
		Name:     "Priority Test Job",
		Status:   "pending",
		Priority: 1,
	}

	db.Mu.Lock()
	db.Jobs[1] = job
	db.JobSeq = 2
	db.Mu.Unlock()

	priority := 3
	req := UpdateJobRequest{
		Priority: &priority,
	}

	_, err := jobService.UpdateJob(1, req)
	assert.NoError(t, err)

	select {
	case jobID := <-db.PriorityChanged:
		assert.Equal(t, uint(1), jobID)
	case <-time.After(1 * time.Second):
		t.Error("Timeout waiting for priority change notification")
	}
}

func TestPriorityScheduling_NoNotificationOnSamePriority(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	jobService := NewJobService(db, cfg, nil)

	job := &models.Job{
		ID:       1,
		Name:     "Priority Test Job",
		Status:   "pending",
		Priority: 2,
	}

	db.Mu.Lock()
	db.Jobs[1] = job
	db.JobSeq = 2
	db.Mu.Unlock()

	priority := 2
	req := UpdateJobRequest{
		Priority: &priority,
	}

	_, err := jobService.UpdateJob(1, req)
	assert.NoError(t, err)

	select {
	case <-db.PriorityChanged:
		t.Error("Unexpected priority change notification")
	case <-time.After(100 * time.Millisecond):
	}
}

func TestPriorityScheduling_QueueOrdering(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	jobService := NewJobService(db, cfg, nil)

	jobs := []*models.Job{
		{ID: 1, Name: "Low Priority", Status: "pending", Priority: 0, GPUs: 1},
		{ID: 2, Name: "Medium Priority", Status: "pending", Priority: 1, GPUs: 1},
		{ID: 3, Name: "High Priority", Status: "pending", Priority: 2, GPUs: 1},
		{ID: 4, Name: "Critical Priority", Status: "pending", Priority: 3, GPUs: 1},
	}

	db.Mu.Lock()
	for _, job := range jobs {
		db.Jobs[job.ID] = job
	}
	db.JobSeq = 5
	db.Mu.Unlock()

	allJobs, err := jobService.GetJobs()
	assert.NoError(t, err)
	assert.Len(t, allJobs, 4)

	sortedJobs := make([]models.Job, len(allJobs))
	copy(sortedJobs, allJobs)

	for i := 0; i < len(sortedJobs)-1; i++ {
		for j := 0; j < len(sortedJobs)-i-1; j++ {
			if sortedJobs[j].Priority < sortedJobs[j+1].Priority {
				sortedJobs[j], sortedJobs[j+1] = sortedJobs[j+1], sortedJobs[j]
			}
		}
	}

	assert.Equal(t, 3, sortedJobs[0].Priority)
	assert.Equal(t, "Critical Priority", sortedJobs[0].Name)
	assert.Equal(t, 0, sortedJobs[len(sortedJobs)-1].Priority)
	assert.Equal(t, "Low Priority", sortedJobs[len(sortedJobs)-1].Name)
}

func TestPriorityScheduling_MixedStatusJobs(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	jobService := NewJobService(db, cfg, nil)

	jobs := []*models.Job{
		{ID: 1, Name: "Low Running", Status: "running", Priority: 0, GPUs: 1},
		{ID: 2, Name: "High Pending", Status: "pending", Priority: 3, GPUs: 1},
		{ID: 3, Name: "Medium Pending", Status: "pending", Priority: 1, GPUs: 1},
		{ID: 4, Name: "Critical Completed", Status: "completed", Priority: 3, GPUs: 1},
	}

	db.Mu.Lock()
	for _, job := range jobs {
		db.Jobs[job.ID] = job
	}
	db.JobSeq = 5
	db.Mu.Unlock()

	pendingJobs := []models.Job{}
	allJobs, _ := jobService.GetJobs()
	for _, job := range allJobs {
		if job.Status == "pending" {
			pendingJobs = append(pendingJobs, job)
		}
	}

	assert.Len(t, pendingJobs, 2)

	for i := 0; i < len(pendingJobs)-1; i++ {
		for j := 0; j < len(pendingJobs)-i-1; j++ {
			if pendingJobs[j].Priority < pendingJobs[j+1].Priority {
				pendingJobs[j], pendingJobs[j+1] = pendingJobs[j+1], pendingJobs[j]
			}
		}
	}

	assert.Equal(t, 3, pendingJobs[0].Priority)
	assert.Equal(t, "High Pending", pendingJobs[0].Name)
}

func TestPriorityScheduling_NotificationWithInvalidPriority(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	jobService := NewJobService(db, cfg, nil)

	job := &models.Job{
		ID:       1,
		Name:     "Test Job",
		Status:   "pending",
		Priority: 1,
	}

	db.Mu.Lock()
	db.Jobs[1] = job
	db.JobSeq = 2
	db.Mu.Unlock()

	priority := 5
	req := UpdateJobRequest{
		Priority: &priority,
	}

	_, err := jobService.UpdateJob(1, req)
	assert.NoError(t, err)

	select {
	case <-db.PriorityChanged:
		t.Error("Unexpected priority change notification for invalid priority")
	case <-time.After(100 * time.Millisecond):
	}

	updatedJob, _ := jobService.GetJob(1)
	assert.Equal(t, 1, updatedJob.Priority)
}

func TestPriorityScheduling_ConcurrentPriorityChanges(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	jobService := NewJobService(db, cfg, nil)

	jobs := make([]*models.Job, 10)
	for i := 0; i < 10; i++ {
		jobs[i] = &models.Job{
			ID:       uint(i + 1),
			Name:     "Job",
			Status:   "pending",
			Priority: 0,
			GPUs:     1,
		}
	}

	db.Mu.Lock()
	for _, job := range jobs {
		db.Jobs[job.ID] = job
	}
	db.JobSeq = 11
	db.Mu.Unlock()

	notificationCount := atomic.Int64{}
	done := make(chan struct{})

	go func() {
		for {
			select {
			case <-db.PriorityChanged:
				notificationCount.Add(1)
			case <-done:
				return
			}
		}
	}()

	for i := 0; i < 10; i++ {
		priority := ((i % 3) + 1)
		req := UpdateJobRequest{
			Priority: &priority,
		}
		jobService.UpdateJob(uint(i+1), req)
	}

	time.Sleep(100 * time.Millisecond)
	close(done)

	assert.Equal(t, int64(10), notificationCount.Load())
}

func TestPriorityScheduling_ChannelBuffer(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()
	jobService := NewJobService(db, cfg, nil)

	jobs := make([]*models.Job, 80)
	for i := 0; i < 80; i++ {
		jobs[i] = &models.Job{
			ID:       uint(i + 1),
			Name:     "Job",
			Status:   "pending",
			Priority: 0,
			GPUs:     1,
		}
	}

	db.Mu.Lock()
	for _, job := range jobs {
		db.Jobs[job.ID] = job
	}
	db.JobSeq = 81
	db.Mu.Unlock()

	successCount := 0
	for i := 0; i < 80; i++ {
		priority := 3
		req := UpdateJobRequest{
			Priority: &priority,
		}
		_, err := jobService.UpdateJob(uint(i+1), req)
		if err == nil {
			successCount++
		}
	}

	assert.Equal(t, 80, successCount)

	notificationCount := 0
	for {
		select {
		case <-db.PriorityChanged:
			notificationCount++
		default:
			goto done
		}
	}
done:
	assert.Equal(t, 80, notificationCount)
}
