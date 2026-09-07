package controllers

import (
	"errors"
	"net/http"
	"testing"

	"github.com/gin-gonic/gin"
	"github.com/stretchr/testify/assert"

	"metaclouds-backend/models"
	appErrors "metaclouds-backend/pkg/errors"
	"metaclouds-backend/services"
)

func TestAccelerationController_GetAccelerationSuites(t *testing.T) {
	tests := []struct {
		name           string
		setupMock      func(m *mockAccelerationService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "获取加速套件列表成功",
			setupMock: func(m *mockAccelerationService) {
				m.getAccelerationSuitesFunc = func() ([]models.AccelerationSuite, error) {
					return []models.AccelerationSuite{
						{ID: 1, Name: "cuda-toolkit", Type: "cuda", Version: "12.0"},
					}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "空列表",
			setupMock: func(m *mockAccelerationService) {
				m.getAccelerationSuitesFunc = func() ([]models.AccelerationSuite, error) {
					return []models.AccelerationSuite{}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "服务层错误",
			setupMock: func(m *mockAccelerationService) {
				m.getAccelerationSuitesFunc = func() ([]models.AccelerationSuite, error) {
					return nil, errors.New("db error")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockAccelerationService{}
			tc.setupMock(mock)
			controller := NewAccelerationController(mock)

			c, w := newTestContext(http.MethodGet, "/api/v1/accelerations", nil)
			controller.GetAccelerationSuites(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestAccelerationController_GetAccelerationSuite(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockAccelerationService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:    "获取单个加速套件成功",
			idParam: "1",
			setupMock: func(m *mockAccelerationService) {
				m.getAccelerationSuiteFunc = func(id uint) (*models.AccelerationSuite, error) {
					return &models.AccelerationSuite{ID: 1, Name: "cuda-toolkit", Status: "active"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "xyz",
			setupMock:      func(m *mockAccelerationService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:    "套件不存在",
			idParam: "999",
			setupMock: func(m *mockAccelerationService) {
				m.getAccelerationSuiteFunc = func(id uint) (*models.AccelerationSuite, error) {
					return nil, errors.New("acceleration suite not found")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockAccelerationService{}
			tc.setupMock(mock)
			controller := NewAccelerationController(mock)

			c, w := newTestContextWithParams(http.MethodGet, "/api/v1/accelerations/"+tc.idParam, nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.GetAccelerationSuite(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestAccelerationController_CreateAccelerationSuite(t *testing.T) {
	tests := []struct {
		name           string
		requestBody    interface{}
		setupMock      func(m *mockAccelerationService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "创建加速套件成功",
			requestBody: map[string]interface{}{
				"name":    "new-suite",
				"type":    "cuda",
				"version": "12.1",
			},
			setupMock: func(m *mockAccelerationService) {
				m.createAccelerationSuiteFunc = func(req services.CreateAccelerationSuiteRequest) (*models.AccelerationSuite, error) {
					assert.Equal(t, "new-suite", req.Name)
					return &models.AccelerationSuite{ID: 1, Name: "new-suite", Status: "active"}, nil
				}
			},
			expectedStatus: http.StatusCreated,
			expectSuccess:  true,
		},
		{
			name:           "缺少必填字段-name",
			requestBody:    map[string]interface{}{"type": "cuda"},
			setupMock:      func(m *mockAccelerationService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name: "名称已存在",
			requestBody: map[string]interface{}{
				"name": "existing-suite",
			},
			setupMock: func(m *mockAccelerationService) {
				m.createAccelerationSuiteFunc = func(req services.CreateAccelerationSuiteRequest) (*models.AccelerationSuite, error) {
					return nil, errors.New("acceleration suite name already exists")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockAccelerationService{}
			tc.setupMock(mock)
			controller := NewAccelerationController(mock)

			c, w := newTestContext(http.MethodPost, "/api/v1/accelerations", tc.requestBody)
			controller.CreateAccelerationSuite(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestAccelerationController_UpdateAccelerationSuite(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		requestBody    interface{}
		setupMock      func(m *mockAccelerationService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:        "更新加速套件成功",
			idParam:     "1",
			requestBody: map[string]interface{}{"name": "updated-suite"},
			setupMock: func(m *mockAccelerationService) {
				m.updateAccelerationSuiteFunc = func(id uint, req services.UpdateAccelerationSuiteRequest) (*models.AccelerationSuite, error) {
					return &models.AccelerationSuite{ID: 1, Name: "updated-suite"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "bad",
			requestBody:    map[string]interface{}{"name": "test"},
			setupMock:      func(m *mockAccelerationService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:        "套件不存在",
			idParam:     "999",
			requestBody: map[string]interface{}{"name": "test"},
			setupMock: func(m *mockAccelerationService) {
				m.updateAccelerationSuiteFunc = func(id uint, req services.UpdateAccelerationSuiteRequest) (*models.AccelerationSuite, error) {
					return nil, errors.New("acceleration suite not found")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockAccelerationService{}
			tc.setupMock(mock)
			controller := NewAccelerationController(mock)

			c, w := newTestContextWithParams(http.MethodPut, "/api/v1/accelerations/"+tc.idParam, tc.requestBody,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.UpdateAccelerationSuite(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestAccelerationController_DeleteAccelerationSuite(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockAccelerationService)
		expectedStatus int
	}{
		{
			name:    "删除加速套件成功-204",
			idParam: "1",
			setupMock: func(m *mockAccelerationService) {
				m.deleteAccelerationSuiteFunc = func(id uint) error {
					return nil
				}
			},
			expectedStatus: http.StatusNoContent,
		},
		{
			name:           "ID无效",
			idParam:        "abc",
			setupMock:      func(m *mockAccelerationService) {},
			expectedStatus: http.StatusInternalServerError,
		},
		{
			name:    "套件不存在",
			idParam: "999",
			setupMock: func(m *mockAccelerationService) {
				m.deleteAccelerationSuiteFunc = func(id uint) error {
					return appErrors.NotFound("acceleration suite not found")
				}
			},
			expectedStatus: http.StatusNotFound,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockAccelerationService{}
			tc.setupMock(mock)
			controller := NewAccelerationController(mock)

			c, w := newTestContextWithParams(http.MethodDelete, "/api/v1/accelerations/"+tc.idParam, nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.DeleteAccelerationSuite(c)

			assert.Equal(t, tc.expectedStatus, c.Writer.Status())
			_ = w
		})
	}
}
