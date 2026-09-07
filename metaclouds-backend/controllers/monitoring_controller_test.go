package controllers

import (
	"errors"
	"net/http"
	"testing"

	"github.com/gin-gonic/gin"
	"github.com/stretchr/testify/assert"

	"metaclouds-backend/models"
)

func TestMonitoringController_GetMetrics(t *testing.T) {
	tests := []struct {
		name           string
		setupMock      func(m *mockMonitoringService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "获取监控指标成功",
			setupMock: func(m *mockMonitoringService) {
				m.getMetricsFunc = func() (map[string]interface{}, error) {
					return map[string]interface{}{
						"cpu":    map[string]interface{}{"usage": 65.5},
						"memory": map[string]interface{}{"usage": 72.3},
						"gpu":    map[string]interface{}{"usage": 45.0, "count": 8},
						"jobs":   map[string]interface{}{"running": 3, "pending": 1},
					}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "空指标",
			setupMock: func(m *mockMonitoringService) {
				m.getMetricsFunc = func() (map[string]interface{}, error) {
					return map[string]interface{}{}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "服务层错误",
			setupMock: func(m *mockMonitoringService) {
				m.getMetricsFunc = func() (map[string]interface{}, error) {
					return nil, errors.New("prometheus connection failed")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockMonitoringService{}
			tc.setupMock(mock)
			controller := NewMonitoringController(mock)

			c, w := newTestContext(http.MethodGet, "/api/v1/monitoring/metrics", nil)
			controller.GetMetrics(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestMonitoringController_GetAlerts(t *testing.T) {
	tests := []struct {
		name           string
		setupMock      func(m *mockMonitoringService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "获取告警列表成功",
			setupMock: func(m *mockMonitoringService) {
				m.getAlertsFunc = func() ([]models.Alert, error) {
					return []models.Alert{
						{ID: 1, Message: "GPU高利用率", Level: "warning", Status: "active"},
						{ID: 2, Message: "磁盘空间不足", Level: "critical", Status: "active"},
					}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "空告警列表",
			setupMock: func(m *mockMonitoringService) {
				m.getAlertsFunc = func() ([]models.Alert, error) {
					return []models.Alert{}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "服务层错误",
			setupMock: func(m *mockMonitoringService) {
				m.getAlertsFunc = func() ([]models.Alert, error) {
					return nil, errors.New("db error")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockMonitoringService{}
			tc.setupMock(mock)
			controller := NewMonitoringController(mock)

			c, w := newTestContext(http.MethodGet, "/api/v1/monitoring/alerts", nil)
			controller.GetAlerts(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestMonitoringController_ResolveAlert(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockMonitoringService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:    "解决告警成功",
			idParam: "1",
			setupMock: func(m *mockMonitoringService) {
				m.resolveAlertFunc = func(id uint) (*models.Alert, error) {
					assert.Equal(t, uint(1), id)
					return &models.Alert{ID: 1, Message: "GPU高利用率", Status: "resolved"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "abc",
			setupMock:      func(m *mockMonitoringService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:    "告警不存在",
			idParam: "999",
			setupMock: func(m *mockMonitoringService) {
				m.resolveAlertFunc = func(id uint) (*models.Alert, error) {
					return nil, errors.New("alert not found")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockMonitoringService{}
			tc.setupMock(mock)
			controller := NewMonitoringController(mock)

			c, w := newTestContextWithParams(http.MethodPost, "/api/v1/monitoring/alerts/"+tc.idParam+"/resolve", nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.ResolveAlert(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}
