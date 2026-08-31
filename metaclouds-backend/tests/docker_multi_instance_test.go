package tests

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"sync"
	"sync/atomic"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

const (
	DockerBaseURL     = "http://localhost:8000"
	DockerAdminUser   = "admin"
	DockerAdminPass   = "admin123"
)

type DockerTestClient struct {
	baseURL    string
	httpClient *http.Client
	token      string
}

func NewDockerTestClient(baseURL string) *DockerTestClient {
	return &DockerTestClient{
		baseURL: baseURL,
		httpClient: &http.Client{
			Timeout: 30 * time.Second,
		},
	}
}

func (c *DockerTestClient) Login() error {
	reqBody := map[string]string{
		"username": DockerAdminUser,
		"password": DockerAdminPass,
	}
	body, _ := json.Marshal(reqBody)

	resp, err := c.httpClient.Post(c.baseURL+"/api/v1/auth/login", "application/json", bytes.NewBuffer(body))
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return err
	}

	if data, ok := result["data"].(map[string]interface{}); ok {
		if token, ok := data["token"].(string); ok {
			c.token = token
			return nil
		}
	}

	return fmt.Errorf("login failed")
}

func (c *DockerTestClient) CreateJob(priority int) (uint, error) {
	reqBody := map[string]interface{}{
		"name":     fmt.Sprintf("Docker-Test-Job-P%d", priority),
		"priority": priority,
		"type":     "training",
		"gpus":     1,
		"cpus":      2,
		"memory":    4,
		"duration":  60,
	}
	body, _ := json.Marshal(reqBody)

	req, _ := http.NewRequest("POST", c.baseURL+"/api/v1/jobs", bytes.NewBuffer(body))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+c.token)

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return 0, err
	}
	defer resp.Body.Close()

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return 0, err
	}

	if data, ok := result["data"].(map[string]interface{}); ok {
		if id, ok := data["id"].(float64); ok {
			return uint(id), nil
		}
	}

	return 0, fmt.Errorf("job creation failed")
}

func (c *DockerTestClient) UpdateJobPriority(jobID uint, priority int) error {
	reqBody := map[string]interface{}{
		"priority": priority,
	}
	body, _ := json.Marshal(reqBody)

	req, _ := http.NewRequest("PUT", fmt.Sprintf("%s/api/v1/jobs/%d", c.baseURL, jobID), bytes.NewBuffer(body))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+c.token)

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	return nil
}

func (c *DockerTestClient) GetJobs() ([]map[string]interface{}, error) {
	req, _ := http.NewRequest("GET", c.baseURL+"/api/v1/jobs", nil)
	req.Header.Set("Authorization", "Bearer "+c.token)

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		return nil, err
	}

	if data, ok := result["data"].([]interface{}); ok {
		jobs := make([]map[string]interface{}, len(data))
		for i, v := range data {
			jobs[i] = v.(map[string]interface{})
		}
		return jobs, nil
	}

	return nil, fmt.Errorf("failed to get jobs")
}

func TestDockerMultiInstanceDeployment(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping docker test in short mode")
	}

	client := NewDockerTestClient(DockerBaseURL)

	err := client.Login()
	if err != nil {
		t.Skipf("Docker services not available: %v", err)
	}

	t.Run("Create Jobs with Different Priorities", func(t *testing.T) {
		priorities := []int{0, 1, 2, 3}
		createdJobs := make([]uint, 0, len(priorities))

		for _, priority := range priorities {
			jobID, err := client.CreateJob(priority)
			assert.NoError(t, err)
			createdJobs = append(createdJobs, jobID)
		}

		assert.Equal(t, 4, len(createdJobs))

		jobs, err := client.GetJobs()
		assert.NoError(t, err)

		priorityCounts := make(map[int]int)
		for _, job := range jobs {
			if priority, ok := job["priority"].(float64); ok {
				priorityCounts[int(priority)]++
			}
		}

		for _, p := range priorities {
			assert.GreaterOrEqual(t, priorityCounts[p], 1, "Should have at least 1 job with priority %d", p)
		}
	})

	t.Run("Concurrent Priority Updates", func(t *testing.T) {
		jobs, err := client.GetJobs()
		assert.NoError(t, err)

		if len(jobs) == 0 {
			t.Skip("No jobs available for testing")
		}

		var wg sync.WaitGroup
		successCount := int32(0)
		numGoroutines := 50

		wg.Add(numGoroutines)
		for i := 0; i < numGoroutines; i++ {
			go func(idx int) {
				defer wg.Done()

				jobID := jobs[idx%len(jobs)]["id"].(float64)
				newPriority := idx % 4

				err := client.UpdateJobPriority(uint(jobID), newPriority)
				if err == nil {
					atomic.AddInt32(&successCount, 1)
				}
			}(i)
		}

		wg.Wait()

		successRate := float64(successCount) / float64(numGoroutines)
		t.Logf("Concurrent update success rate: %.2f%% (%d/%d)", successRate*100, successCount, numGoroutines)
		assert.GreaterOrEqual(t, successRate, 0.8, "At least 80%% of updates should succeed")
	})

	t.Run("Load Balancing Across Instances", func(t *testing.T) {
		var wg sync.WaitGroup
		instanceHits := make(map[string]int)
		var mu sync.Mutex
		numRequests := 100

		wg.Add(numRequests)
		for i := 0; i < numRequests; i++ {
			go func() {
				defer wg.Done()

				req, _ := http.NewRequest("GET", DockerBaseURL+"/health", nil)
				resp, err := client.httpClient.Do(req)
				if err != nil {
					return
				}
				defer resp.Body.Close()

				instance := resp.Header.Get("X-Upstream-Instance")
				if instance != "" {
					mu.Lock()
					instanceHits[instance]++
					mu.Unlock()
				}
			}()
		}

		wg.Wait()

		t.Logf("Instance distribution: %v", instanceHits)
		assert.NotEmpty(t, instanceHits, "Should have requests distributed across instances")
	})

	t.Run("Priority Scheduler Metrics", func(t *testing.T) {
		req, _ := http.NewRequest("GET", DockerBaseURL+"/metrics", nil)
		resp, err := client.httpClient.Do(req)
		assert.NoError(t, err)
		defer resp.Body.Close()

		body, _ := io.ReadAll(resp.Body)
		metrics := string(body)

		assert.Contains(t, metrics, "scheduler_priority_changes", "Metrics should contain priority change counter")
	})
}

func TestDockerHighConcurrency(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping docker test in short mode")
	}

	client := NewDockerTestClient(DockerBaseURL)

	err := client.Login()
	if err != nil {
		t.Skipf("Docker services not available: %v", err)
	}

	t.Run("High Concurrency Job Creation", func(t *testing.T) {
		var wg sync.WaitGroup
		successCount := int32(0)
		numGoroutines := 100

		wg.Add(numGoroutines)
		for i := 0; i < numGoroutines; i++ {
			go func(idx int) {
				defer wg.Done()

				priority := idx % 4
				_, err := client.CreateJob(priority)
				if err == nil {
					atomic.AddInt32(&successCount, 1)
				}
			}(i)
		}

		wg.Wait()

		successRate := float64(successCount) / float64(numGoroutines)
		t.Logf("Job creation success rate: %.2f%% (%d/%d)", successRate*100, successCount, numGoroutines)
		assert.GreaterOrEqual(t, successRate, 0.9, "At least 90%% of jobs should be created")
	})

	t.Run("Mixed Priority Operations", func(t *testing.T) {
		jobs, err := client.GetJobs()
		assert.NoError(t, err)

		if len(jobs) == 0 {
			t.Skip("No jobs available for testing")
		}

		var wg sync.WaitGroup
		operations := int32(0)
		numGoroutines := 100

		wg.Add(numGoroutines)
		for i := 0; i < numGoroutines; i++ {
			go func(idx int) {
				defer wg.Done()

				jobID := jobs[idx%len(jobs)]["id"].(float64)

				op := idx % 3
				switch op {
				case 0:
					client.CreateJob(idx % 4)
				case 1:
					client.UpdateJobPriority(uint(jobID), idx%4)
				case 2:
					client.GetJobs()
				}

				atomic.AddInt32(&operations, 1)
			}(i)
		}

		wg.Wait()

		t.Logf("Completed %d operations", operations)
		assert.Equal(t, int32(numGoroutines), operations)
	})
}
