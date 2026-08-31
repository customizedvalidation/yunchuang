package tests

import (
	"sync"
	"sync/atomic"
	"testing"

	"github.com/stretchr/testify/assert"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
	"metaclouds-backend/services"
)

func TestHighConcurrencyPriorityTaskCreation(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
		JWTExpirationHours: 24,
	}

	db := models.MustNewMemoryStore()
	k8sService := services.NewK8SService(db, cfg)
	jobService := services.NewJobService(db, cfg, k8sService)

	numGoroutines := 100
	var wg sync.WaitGroup
	wg.Add(numGoroutines)

	successCount := 0
	var mu sync.Mutex

	for i := 0; i < numGoroutines; i++ {
		go func(idx int) {
			defer wg.Done()

			priority := idx % 4
			req := services.CreateJobRequest{
				Name:        "Concurrent Task",
				Description: "High concurrency test task",
				Type:        "training",
				Priority:    priority,
				GPUs:        1,
				CPUs:        1,
				Memory:      4,
				Duration:    60,
				ClusterID:   1,
				TenantID:    1,
				UserID:      1,
			}

			_, err := jobService.CreateJob(req)
			if err == nil {
				mu.Lock()
				successCount++
				mu.Unlock()
			}
		}(i)
	}

	wg.Wait()

	assert.Equal(t, numGoroutines, successCount, "All concurrent task creation should succeed")

	allJobs, err := jobService.GetJobs()
	assert.NoError(t, err)
	assert.Equal(t, numGoroutines+2, len(allJobs), "Total jobs should include initial data plus new tasks")

	priorityCounts := make(map[int]int)
	for _, job := range allJobs {
		priorityCounts[job.Priority]++
	}

	assert.GreaterOrEqual(t, priorityCounts[0], 20, "Should have at least 20 low priority tasks")
	assert.GreaterOrEqual(t, priorityCounts[1], 20, "Should have at least 20 medium priority tasks")
	assert.GreaterOrEqual(t, priorityCounts[2], 20, "Should have at least 20 high priority tasks")
	assert.GreaterOrEqual(t, priorityCounts[3], 20, "Should have at least 20 critical priority tasks")
}

func TestHighConcurrencyPriorityUpdates(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
		JWTExpirationHours: 24,
	}

	db := models.MustNewMemoryStore()
	k8sService := services.NewK8SService(db, cfg)
	jobService := services.NewJobService(db, cfg, k8sService)

	numInitialJobs := 50
	jobIDs := make([]uint, 0, numInitialJobs)
	for i := 0; i < numInitialJobs; i++ {
		req := services.CreateJobRequest{
			Name:        "Initial Task",
			Type:        "training",
			Priority:    0,
			GPUs:        1,
			CPUs:        1,
			Memory:      4,
			Duration:    60,
			ClusterID:   1,
			TenantID:    1,
			UserID:      1,
		}
		job, _ := jobService.CreateJob(req)
		jobIDs = append(jobIDs, job.ID)
	}

	numUpdates := 100
	var wg sync.WaitGroup
	wg.Add(numUpdates)

	successCount := 0
	var mu sync.Mutex

	for i := 0; i < numUpdates; i++ {
		go func(idx int) {
			defer wg.Done()

			jobID := jobIDs[idx%numInitialJobs]
			newPriority := idx % 4

			req := services.UpdateJobRequest{
				Priority: &newPriority,
			}

			_, err := jobService.UpdateJob(jobID, req)
			if err == nil {
				mu.Lock()
				successCount++
				mu.Unlock()
			}
		}(i)
	}

	wg.Wait()

	assert.Equal(t, numUpdates, successCount, "All concurrent priority updates should succeed")

	allJobs, err := jobService.GetJobs()
	assert.NoError(t, err)

	priorityDistribution := make(map[int]int)
	for _, job := range allJobs {
		priorityDistribution[job.Priority]++
	}

	t.Logf("Priority distribution after concurrent updates: %v", priorityDistribution)

	assert.True(t, len(priorityDistribution) > 1, "Should have multiple priority levels after updates")
}

func TestPriorityChangeChannelConcurrency(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
		JWTExpirationHours: 24,
	}

	db := models.MustNewMemoryStore()
	k8sService := services.NewK8SService(db, cfg)
	jobService := services.NewJobService(db, cfg, k8sService)

	var wg sync.WaitGroup

	numJobs := 30
	for i := 0; i < numJobs; i++ {
		req := services.CreateJobRequest{
			Name:        "Priority Change Test Task",
			Type:        "training",
			Priority:    0,
			GPUs:        1,
			CPUs:        1,
			Memory:      4,
			Duration:    60,
			ClusterID:   1,
			TenantID:    1,
			UserID:      1,
		}
		jobService.CreateJob(req)
	}

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

	numUpdates := 30
	wg.Add(numUpdates)

	for i := 0; i < numUpdates; i++ {
		go func(idx int) {
			defer wg.Done()

			jobID := uint(idx + 1)
			newPriority := (idx % 3) + 1

			req := services.UpdateJobRequest{
				Priority: &newPriority,
			}

			jobService.UpdateJob(jobID, req)
		}(i)
	}

	wg.Wait()

	close(done)

	assert.Greater(t, notificationCount.Load(), int64(0), "Should receive priority change notifications")
	t.Logf("Total priority change notifications received: %d", notificationCount.Load())
}

func TestMixedConcurrentOperations(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
		JWTExpirationHours: 24,
	}

	db := models.MustNewMemoryStore()
	k8sService := services.NewK8SService(db, cfg)
	jobService := services.NewJobService(db, cfg, k8sService)

	var wg sync.WaitGroup

	numInitialJobs := 10
	for i := 0; i < numInitialJobs; i++ {
		req := services.CreateJobRequest{
			Name:        "Initial Mixed Op Task",
			Type:        "training",
			Priority:    0,
			GPUs:        1,
			CPUs:        1,
			Memory:      4,
			Duration:    60,
			ClusterID:   1,
			TenantID:    1,
			UserID:      1,
		}
		jobService.CreateJob(req)
	}

	numGoroutines := 50
	successCount := 0
	var mu sync.Mutex

	wg.Add(numGoroutines)

	for i := 0; i < numGoroutines; i++ {
		go func(idx int) {
			defer wg.Done()

			opType := idx % 2
			switch opType {
			case 0:
				req := services.CreateJobRequest{
					Name:        "Mixed Op Task",
					Type:        "training",
					Priority:    idx % 4,
					GPUs:        1,
					CPUs:        1,
					Memory:      4,
					Duration:    60,
					ClusterID:   1,
					TenantID:    1,
					UserID:      1,
				}
				_, err := jobService.CreateJob(req)
				if err == nil {
					mu.Lock()
					successCount++
					mu.Unlock()
				}
			case 1:
				allJobs, err := jobService.GetJobs()
				if err == nil && len(allJobs) > 0 {
					jobID := allJobs[idx%len(allJobs)].ID
					newPriority := idx % 4
					req := services.UpdateJobRequest{
						Priority: &newPriority,
					}
					_, err := jobService.UpdateJob(jobID, req)
					if err == nil {
						mu.Lock()
						successCount++
						mu.Unlock()
					}
				}
			}
		}(i)
	}

	wg.Wait()

	assert.GreaterOrEqual(t, successCount, 20, "Most operations should succeed")
	t.Logf("Successful concurrent operations: %d out of %d", successCount, numGoroutines)
}
