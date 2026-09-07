package controllers

import (
	"errors"
	"net/http"
	"testing"

	"github.com/gin-gonic/gin"
	"github.com/stretchr/testify/assert"

	"metaclouds-backend/models"
	"metaclouds-backend/services"
)

func TestResourceController_GetResources(t *testing.T) {
	tests := []struct {
		name           string
		setupMock      func(m *mockResourceService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "获取资源列表成功",
			setupMock: func(m *mockResourceService) {
				m.getResourcesFunc = func() ([]models.Resource, error) {
					return []models.Resource{
						{ID: 1, Name: "gpu-0", Type: "gpu", Status: "available"},
						{ID: 2, Name: "gpu-1", Type: "gpu", Status: "used"},
					}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "空列表",
			setupMock: func(m *mockResourceService) {
				m.getResourcesFunc = func() ([]models.Resource, error) {
					return []models.Resource{}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "服务层错误",
			setupMock: func(m *mockResourceService) {
				m.getResourcesFunc = func() ([]models.Resource, error) {
					return nil, errors.New("db error")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockResourceService{}
			tc.setupMock(mock)
			controller := NewResourceController(mock)

			c, w := newTestContext(http.MethodGet, "/api/v1/resources", nil)
			controller.GetResources(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestResourceController_GetResource(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockResourceService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:    "获取单个资源成功",
			idParam: "1",
			setupMock: func(m *mockResourceService) {
				m.getResourceFunc = func(id uint) (*models.Resource, error) {
					assert.Equal(t, uint(1), id)
					return &models.Resource{ID: 1, Name: "gpu-0", Type: "gpu"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "xyz",
			setupMock:      func(m *mockResourceService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:    "资源不存在",
			idParam: "999",
			setupMock: func(m *mockResourceService) {
				m.getResourceFunc = func(id uint) (*models.Resource, error) {
					return nil, errors.New("resource not found")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockResourceService{}
			tc.setupMock(mock)
			controller := NewResourceController(mock)

			c, w := newTestContextWithParams(http.MethodGet, "/api/v1/resources/"+tc.idParam, nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.GetResource(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestResourceController_UpdateResource(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		requestBody    interface{}
		setupMock      func(m *mockResourceService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:        "更新资源成功",
			idParam:     "1",
			requestBody: map[string]interface{}{"status": "maintenance", "total": 10},
			setupMock: func(m *mockResourceService) {
				m.updateResourceFunc = func(id uint, req services.UpdateResourceRequest) (*models.Resource, error) {
					assert.Equal(t, uint(1), id)
					assert.Equal(t, "maintenance", req.Status)
					return &models.Resource{ID: 1, Name: "gpu-0", Status: "maintenance"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "bad",
			requestBody:    map[string]interface{}{"status": "ok"},
			setupMock:      func(m *mockResourceService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:        "负值被拒绝-total",
			idParam:     "1",
			requestBody: map[string]interface{}{"total": -5},
			setupMock:   func(m *mockResourceService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name:        "资源不存在",
			idParam:     "999",
			requestBody: map[string]interface{}{"status": "ok"},
			setupMock: func(m *mockResourceService) {
				m.updateResourceFunc = func(id uint, req services.UpdateResourceRequest) (*models.Resource, error) {
					return nil, errors.New("resource not found")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockResourceService{}
			tc.setupMock(mock)
			controller := NewResourceController(mock)

			c, w := newTestContextWithParams(http.MethodPut, "/api/v1/resources/"+tc.idParam, tc.requestBody,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.UpdateResource(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}
