package services

import (
	"errors"
	"metaclouds-backend/config"
	"metaclouds-backend/models"
	"metaclouds-backend/pkg/logger"
	"time"
)

type MonitoringService struct {
	db     *models.MemoryStore
	config *config.Config
}

func NewMonitoringService(db interface{}, config *config.Config) *MonitoringService {
	memoryStore, err := models.GetDBStore(db, "MonitoringService")
	if err != nil {
		logger.ErrorWithCtx(nil, "Failed to initialize MonitoringService", err)
		return nil
	}
	return &MonitoringService{
		db:     memoryStore,
		config: config,
	}
}

func (s *MonitoringService) GetMetrics() (map[string]interface{}, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var totalGPUs, usedGPUs, availableGPUs int
	var gpuUtilization float64
	for _, resource := range s.db.Resources {
		if resource.Type == "gpu" {
			totalGPUs += resource.Total
			usedGPUs += resource.Used
			availableGPUs += resource.Available
			gpuUtilization += resource.Utilization
		}
	}
	if totalGPUs > 0 {
		gpuUtilization = float64(usedGPUs) / float64(totalGPUs) * 100
	}

	var runningJobs, pendingJobs, completedJobs, failedJobs int
	for _, job := range s.db.Jobs {
		switch job.Status {
		case "running":
			runningJobs++
		case "pending":
			pendingJobs++
		case "completed":
			completedJobs++
		case "failed":
			failedJobs++
		}
	}

	metrics := map[string]interface{}{
		"cpu": map[string]interface{}{
			"usage":     65.5,
			"cores":     32,
			"available": 11,
		},
		"memory": map[string]interface{}{
			"usage":     72.3,
			"total":     128,
			"available": 35,
		},
		"gpu": map[string]interface{}{
			"usage":     gpuUtilization,
			"count":     totalGPUs,
			"used":      usedGPUs,
			"available": availableGPUs,
		},
		"network": map[string]interface{}{
			"in":   1024,
			"out":  512,
			"loss": 0.1,
		},
		"storage": map[string]interface{}{
			"usage":     60.2,
			"total":     1000,
			"available": 398,
		},
		"jobs": map[string]interface{}{
			"running":   runningJobs,
			"pending":   pendingJobs,
			"completed": completedJobs,
			"failed":    failedJobs,
			"total":     len(s.db.Jobs),
		},
	}

	return metrics, nil
}

func (s *MonitoringService) GetAlerts() ([]models.Alert, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var alerts []models.Alert
	for _, alert := range s.db.Alerts {
		alerts = append(alerts, *alert)
	}
	return alerts, nil
}

func (s *MonitoringService) ResolveAlert(id uint) (*models.Alert, error) {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	alert, exists := s.db.Alerts[id]
	if !exists {
		return nil, errors.New("alert not found")
	}

	alert.Status = "resolved"
	alert.UpdatedAt = time.Now()

	return alert, nil
}
