package services

import (
	"fmt"
	"time"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
	"metaclouds-backend/pkg/logger"
)

type AlertRule struct {
	ID          uint
	Name        string
	Description string
	Type        string
	Threshold   float64
	Operator    string
	Level       string
	Enabled     bool
	Notify      bool
}

type AlertNotification struct {
	ID        uint
	AlertID   uint
	Level     string
	Message   string
	Timestamp time.Time
	Sent      bool
}

type AlertService struct {
	config      *config.Config
	rules       []AlertRule
	notifications []AlertNotification
	nextID      uint
}

func NewAlertService(cfg *config.Config) *AlertService {
	s := &AlertService{
		config:      cfg,
		rules:       make([]AlertRule, 0),
		notifications: make([]AlertNotification, 0),
		nextID:      1,
	}
	s.initDefaultRules()
	return s
}

func (s *AlertService) initDefaultRules() {
	s.rules = []AlertRule{
		{ID: 1, Name: "GPU High Utilization", Description: "Trigger when GPU utilization exceeds 90%", Type: "gpu", Threshold: 90, Operator: ">", Level: "warning", Enabled: true, Notify: true},
		{ID: 2, Name: "CPU High Utilization", Description: "Trigger when CPU utilization exceeds 85%", Type: "cpu", Threshold: 85, Operator: ">", Level: "warning", Enabled: true, Notify: true},
		{ID: 3, Name: "Memory High Utilization", Description: "Trigger when memory utilization exceeds 80%", Type: "memory", Threshold: 80, Operator: ">", Level: "warning", Enabled: true, Notify: true},
		{ID: 4, Name: "Storage Low", Description: "Trigger when available storage is below 10%", Type: "storage", Threshold: 10, Operator: "<", Level: "critical", Enabled: true, Notify: true},
		{ID: 5, Name: "Job Queue Length", Description: "Trigger when pending jobs exceed 50", Type: "jobs_pending", Threshold: 50, Operator: ">", Level: "warning", Enabled: true, Notify: true},
		{ID: 6, Name: "GPU Unavailable", Description: "Trigger when no GPUs are available", Type: "gpu_available", Threshold: 0, Operator: "==", Level: "critical", Enabled: true, Notify: true},
	}
}

func (s *AlertService) EvaluateMetrics(metrics map[string]interface{}) []models.Alert {
	var newAlerts []models.Alert

	for _, rule := range s.rules {
		if !rule.Enabled {
			continue
		}

		var currentValue float64
		var found bool

		switch rule.Type {
		case "gpu":
			if gpu, ok := metrics["gpu"].(map[string]interface{}); ok {
				if usage, ok := gpu["usage"].(float64); ok {
					currentValue = usage
					found = true
				}
			}
		case "cpu":
			if cpu, ok := metrics["cpu"].(map[string]interface{}); ok {
				if usage, ok := cpu["usage"].(float64); ok {
					currentValue = usage
					found = true
				}
			}
		case "memory":
			if memory, ok := metrics["memory"].(map[string]interface{}); ok {
				if usage, ok := memory["usage"].(float64); ok {
					currentValue = usage
					found = true
				}
			}
		case "storage":
			if storage, ok := metrics["storage"].(map[string]interface{}); ok {
				if available, ok := storage["available"].(float64); ok {
					currentValue = available
					found = true
				}
			}
		case "jobs_pending":
			if jobs, ok := metrics["jobs"].(map[string]interface{}); ok {
				if pending, ok := jobs["pending"].(int); ok {
					currentValue = float64(pending)
					found = true
				}
			}
		case "gpu_available":
			if gpu, ok := metrics["gpu"].(map[string]interface{}); ok {
				if available, ok := gpu["available"].(int); ok {
					currentValue = float64(available)
					found = true
				}
			}
		}

		if !found {
			continue
		}

		if s.evaluateCondition(currentValue, rule.Threshold, rule.Operator) {
			alert := models.Alert{
				ID:        s.nextID,
				Type:      rule.Type,
				Level:     rule.Level,
				Status:    "active",
				Message:   fmt.Sprintf("%s: Current value %.2f %s threshold %.2f", rule.Name, currentValue, rule.Operator, rule.Threshold),
				Details:   fmt.Sprintf(`{"rule_id": %d, "current_value": %.2f, "threshold": %.2f}`, rule.ID, currentValue, rule.Threshold),
				CreatedAt: time.Now(),
				UpdatedAt: time.Now(),
			}
			newAlerts = append(newAlerts, alert)
			s.nextID++

			if rule.Notify {
				s.sendNotification(alert)
			}

			logger.WarnWithCtx(nil, "Alert triggered", "rule", rule.Name, "level", rule.Level, "value", currentValue)
		}
	}

	return newAlerts
}

func (s *AlertService) evaluateCondition(current, threshold float64, operator string) bool {
	switch operator {
	case ">":
		return current > threshold
	case ">=":
		return current >= threshold
	case "<":
		return current < threshold
	case "<=":
		return current <= threshold
	case "==":
		return current == threshold
	case "!=":
		return current != threshold
	default:
		return false
	}
}

func (s *AlertService) sendNotification(alert models.Alert) {
	notification := AlertNotification{
		ID:        uint(len(s.notifications) + 1),
		AlertID:   alert.ID,
		Level:     alert.Level,
		Message:   alert.Message,
		Timestamp: time.Now(),
		Sent:      false,
	}

	s.notifications = append(s.notifications, notification)

	if s.config.AlertEnabled {
		s.dispatchNotification(notification)
	}
}

func (s *AlertService) dispatchNotification(notification AlertNotification) {
	if notification.Level == "critical" {
		logger.ErrorWithCtx(nil, "Critical alert notification", nil, "message", notification.Message)
	} else {
		logger.WarnWithCtx(nil, "Alert notification", "message", notification.Message)
	}
}

func (s *AlertService) GetRules() []AlertRule {
	return s.rules
}

func (s *AlertService) GetNotifications() []AlertNotification {
	return s.notifications
}

func (s *AlertService) UpdateRule(id uint, rule AlertRule) error {
	for i, r := range s.rules {
		if r.ID == id {
			s.rules[i] = rule
			return nil
		}
	}
	return fmt.Errorf("rule not found")
}

func (s *AlertService) EnableRule(id uint) error {
	for i, r := range s.rules {
		if r.ID == id {
			s.rules[i].Enabled = true
			return nil
		}
	}
	return fmt.Errorf("rule not found")
}

func (s *AlertService) DisableRule(id uint) error {
	for i, r := range s.rules {
		if r.ID == id {
			s.rules[i].Enabled = false
			return nil
		}
	}
	return fmt.Errorf("rule not found")
}