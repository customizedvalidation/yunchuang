package services

import (
	"testing"
	"time"

	"github.com/stretchr/testify/assert"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
)

func clearTestData(db *models.MemoryStore) {
	db.Mu.Lock()
	db.Resources = make(map[uint]*models.Resource)
	db.Jobs = make(map[uint]*models.Job)
	db.Alerts = make(map[uint]*models.Alert)
	db.ResourceSeq = 1
	db.JobSeq = 1
	db.AlertSeq = 1
	db.Mu.Unlock()
}

func TestMonitoringService_GetMetrics_WithData(t *testing.T) {
	cfg := &config.Config{}

	db := models.MustNewMemoryStore()
	clearTestData(db)

	db.Mu.Lock()
	db.Resources[1] = &models.Resource{
		ID:          1,
		Type:        "gpu",
		Total:       4,
		Used:        2,
		Available:   2,
		Utilization: 50.0,
	}
	db.Resources[2] = &models.Resource{
		ID:          2,
		Type:        "gpu",
		Total:       2,
		Used:        1,
		Available:   1,
		Utilization: 50.0,
	}
	db.Jobs[1] = &models.Job{
		ID:     1,
		Status: "running",
	}
	db.Jobs[2] = &models.Job{
		ID:     2,
		Status: "pending",
	}
	db.Jobs[3] = &models.Job{
		ID:     3,
		Status: "completed",
	}
	db.Mu.Unlock()

	service := NewMonitoringService(db, cfg)

	metrics, err := service.GetMetrics()

	assert.NoError(t, err)
	assert.NotNil(t, metrics)

	cpu, ok := metrics["cpu"].(map[string]interface{})
	assert.True(t, ok)
	assert.Equal(t, 65.5, cpu["usage"])

	memory, ok := metrics["memory"].(map[string]interface{})
	assert.True(t, ok)
	assert.Equal(t, 72.3, memory["usage"])

	gpu, ok := metrics["gpu"].(map[string]interface{})
	assert.True(t, ok)
	assert.Equal(t, 6, gpu["count"])
	assert.Equal(t, 3, gpu["used"])
	assert.Equal(t, 3, gpu["available"])
	assert.InDelta(t, 50.0, gpu["usage"], 0.01)

	jobs, ok := metrics["jobs"].(map[string]interface{})
	assert.True(t, ok)
	assert.Equal(t, 1, jobs["running"])
	assert.Equal(t, 1, jobs["pending"])
	assert.Equal(t, 1, jobs["completed"])
	assert.Equal(t, 3, jobs["total"])
}

func TestMonitoringService_GetMetrics_EmptyData(t *testing.T) {
	cfg := &config.Config{}

	db := models.MustNewMemoryStore()
	clearTestData(db)

	service := NewMonitoringService(db, cfg)

	metrics, err := service.GetMetrics()

	assert.NoError(t, err)
	assert.NotNil(t, metrics)

	gpu, ok := metrics["gpu"].(map[string]interface{})
	assert.True(t, ok)
	assert.Zero(t, gpu["count"])
	assert.Zero(t, gpu["used"])
	assert.Zero(t, gpu["usage"])

	jobs, ok := metrics["jobs"].(map[string]interface{})
	assert.True(t, ok)
	assert.Zero(t, jobs["total"])
}

func TestMonitoringService_GetMetrics_NoGPUData(t *testing.T) {
	cfg := &config.Config{}

	db := models.MustNewMemoryStore()
	clearTestData(db)

	db.Mu.Lock()
	db.Resources[1] = &models.Resource{
		ID:          1,
		Type:        "cpu",
		Total:       8,
		Used:        4,
		Available:   4,
		Utilization: 50.0,
	}
	db.Mu.Unlock()

	service := NewMonitoringService(db, cfg)

	metrics, err := service.GetMetrics()

	assert.NoError(t, err)
	assert.NotNil(t, metrics)

	gpu, ok := metrics["gpu"].(map[string]interface{})
	assert.True(t, ok)
	assert.Zero(t, gpu["count"])
	assert.Zero(t, gpu["usage"])
}

func TestMonitoringService_GetMetrics_MixedResources(t *testing.T) {
	cfg := &config.Config{}

	db := models.MustNewMemoryStore()
	clearTestData(db)

	db.Mu.Lock()
	db.Resources[1] = &models.Resource{
		ID:          1,
		Type:        "gpu",
		Total:       10,
		Used:        7,
		Available:   3,
		Utilization: 70.0,
	}
	db.Resources[2] = &models.Resource{
		ID:          2,
		Type:        "cpu",
		Total:       32,
		Used:        16,
		Available:   16,
		Utilization: 50.0,
	}
	db.Resources[3] = &models.Resource{
		ID:          3,
		Type:        "gpu",
		Total:       5,
		Used:        2,
		Available:   3,
		Utilization: 40.0,
	}
	db.Mu.Unlock()

	service := NewMonitoringService(db, cfg)

	metrics, err := service.GetMetrics()

	assert.NoError(t, err)
	assert.NotNil(t, metrics)

	gpu, ok := metrics["gpu"].(map[string]interface{})
	assert.True(t, ok)
	assert.Equal(t, 15, gpu["count"])
	assert.Equal(t, 9, gpu["used"])
	assert.InDelta(t, 60.0, gpu["usage"], 0.01)
}

func TestMonitoringService_GetAlerts_WithAlerts(t *testing.T) {
	cfg := &config.Config{}

	db := models.MustNewMemoryStore()
	clearTestData(db)

	now := time.Now()
	db.Mu.Lock()
	db.Alerts[1] = &models.Alert{
		ID:        1,
		Type:      "system",
		Level:     "warning",
		Message:   "High CPU usage",
		Status:    "active",
		CreatedAt: now,
		UpdatedAt: now,
	}
	db.Alerts[2] = &models.Alert{
		ID:        2,
		Type:      "resource",
		Level:     "error",
		Message:   "GPU failure detected",
		Status:    "active",
		CreatedAt: now,
		UpdatedAt: now,
	}
	db.Mu.Unlock()

	service := NewMonitoringService(db, cfg)

	alerts, err := service.GetAlerts()

	assert.NoError(t, err)
	assert.Len(t, alerts, 2)

	var foundAlert1, foundAlert2 bool
	for _, alert := range alerts {
		if alert.ID == 1 {
			foundAlert1 = true
			assert.Equal(t, "system", alert.Type)
			assert.Equal(t, "warning", alert.Level)
			assert.Equal(t, "High CPU usage", alert.Message)
			assert.Equal(t, "active", alert.Status)
		}
		if alert.ID == 2 {
			foundAlert2 = true
			assert.Equal(t, "resource", alert.Type)
			assert.Equal(t, "error", alert.Level)
			assert.Equal(t, "GPU failure detected", alert.Message)
			assert.Equal(t, "active", alert.Status)
		}
	}
	assert.True(t, foundAlert1, "Alert with ID 1 not found")
	assert.True(t, foundAlert2, "Alert with ID 2 not found")
}

func TestMonitoringService_GetAlerts_Empty(t *testing.T) {
	cfg := &config.Config{}

	db := models.MustNewMemoryStore()
	clearTestData(db)

	service := NewMonitoringService(db, cfg)

	alerts, err := service.GetAlerts()

	assert.NoError(t, err)
	assert.Empty(t, alerts)
}

func TestMonitoringService_GetAlerts_DefaultData(t *testing.T) {
	cfg := &config.Config{}

	db := models.MustNewMemoryStore()

	service := NewMonitoringService(db, cfg)

	alerts, err := service.GetAlerts()

	assert.NoError(t, err)
	assert.Len(t, alerts, 1)
	assert.Equal(t, "resource", alerts[0].Type)
	assert.Equal(t, "warning", alerts[0].Level)
	assert.Equal(t, "GPU utilization exceeds 90%", alerts[0].Message)
}

func TestMonitoringService_ResolveAlert_Success(t *testing.T) {
	cfg := &config.Config{}

	db := models.MustNewMemoryStore()
	clearTestData(db)

	now := time.Now()
	db.Mu.Lock()
	db.Alerts[1] = &models.Alert{
		ID:        1,
		Type:      "system",
		Level:     "warning",
		Message:   "High CPU usage",
		Status:    "active",
		CreatedAt: now,
		UpdatedAt: now,
	}
	db.Mu.Unlock()

	service := NewMonitoringService(db, cfg)

	alert, err := service.ResolveAlert(1)

	assert.NoError(t, err)
	assert.NotNil(t, alert)
	assert.Equal(t, uint(1), alert.ID)
	assert.Equal(t, "resolved", alert.Status)
	assert.True(t, alert.UpdatedAt.After(now) || alert.UpdatedAt.Equal(now))
}

func TestMonitoringService_ResolveAlert_NotFound(t *testing.T) {
	cfg := &config.Config{}

	db := models.MustNewMemoryStore()
	clearTestData(db)

	service := NewMonitoringService(db, cfg)

	alert, err := service.ResolveAlert(999)

	assert.Error(t, err)
	assert.Nil(t, alert)
	assert.Contains(t, err.Error(), "alert not found")
}

func TestMonitoringService_ResolveAlert_AlreadyResolved(t *testing.T) {
	cfg := &config.Config{}

	db := models.MustNewMemoryStore()
	clearTestData(db)

	now := time.Now()
	db.Mu.Lock()
	db.Alerts[1] = &models.Alert{
		ID:        1,
		Type:      "system",
		Level:     "info",
		Message:   "Test alert",
		Status:    "resolved",
		CreatedAt: now,
		UpdatedAt: now,
	}
	db.Mu.Unlock()

	service := NewMonitoringService(db, cfg)

	alert, err := service.ResolveAlert(1)

	assert.NoError(t, err)
	assert.NotNil(t, alert)
	assert.Equal(t, "resolved", alert.Status)
}

func TestMonitoringService_New_NilDB(t *testing.T) {
	cfg := &config.Config{}

	service := NewMonitoringService(nil, cfg)

	assert.NotNil(t, service)
	assert.NotNil(t, service.db)
}

func TestMonitoringService_New_WithDB(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	service := NewMonitoringService(db, cfg)

	assert.NotNil(t, service)
	assert.Equal(t, db, service.db)
}

func TestMonitoringService_New_InvalidDBType(t *testing.T) {
	cfg := &config.Config{}

	invalidDB := "not a memory store"

	service := NewMonitoringService(invalidDB, cfg)

	assert.Nil(t, service)
}

func TestMonitoringService_New_NilConfig(t *testing.T) {
	db := models.MustNewMemoryStore()

	service := NewMonitoringService(db, nil)

	assert.NotNil(t, service)
	assert.Equal(t, db, service.db)
	assert.Nil(t, service.config)
}

func TestMonitoringService_New_BothNil(t *testing.T) {
	service := NewMonitoringService(nil, nil)

	assert.NotNil(t, service)
	assert.NotNil(t, service.db)
	assert.Nil(t, service.config)
}

func TestMonitoringService_GetMetrics_NetworkStorage(t *testing.T) {
	cfg := &config.Config{}

	db := models.MustNewMemoryStore()
	clearTestData(db)

	service := NewMonitoringService(db, cfg)

	metrics, err := service.GetMetrics()

	assert.NoError(t, err)
	assert.NotNil(t, metrics)

	network, ok := metrics["network"].(map[string]interface{})
	assert.True(t, ok)
	assert.Equal(t, 1024, network["in"])
	assert.Equal(t, 512, network["out"])
	assert.Equal(t, 0.1, network["loss"])

	storage, ok := metrics["storage"].(map[string]interface{})
	assert.True(t, ok)
	assert.Equal(t, 60.2, storage["usage"])
	assert.Equal(t, 1000, storage["total"])
	assert.Equal(t, 398, storage["available"])
}

func TestMonitoringService_GetMetrics_AllJobStatus(t *testing.T) {
	cfg := &config.Config{}

	db := models.MustNewMemoryStore()
	clearTestData(db)

	db.Mu.Lock()
	db.Jobs[1] = &models.Job{ID: 1, Status: "running"}
	db.Jobs[2] = &models.Job{ID: 2, Status: "running"}
	db.Jobs[3] = &models.Job{ID: 3, Status: "pending"}
	db.Jobs[4] = &models.Job{ID: 4, Status: "completed"}
	db.Jobs[5] = &models.Job{ID: 5, Status: "failed"}
	db.Jobs[6] = &models.Job{ID: 6, Status: "cancelled"}
	db.Mu.Unlock()

	service := NewMonitoringService(db, cfg)

	metrics, err := service.GetMetrics()

	assert.NoError(t, err)

	jobs, ok := metrics["jobs"].(map[string]interface{})
	assert.True(t, ok)
	assert.Equal(t, 2, jobs["running"])
	assert.Equal(t, 1, jobs["pending"])
	assert.Equal(t, 1, jobs["completed"])
	assert.Equal(t, 6, jobs["total"])
}
