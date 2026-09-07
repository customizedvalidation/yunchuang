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

func TestTenantController_GetTenants(t *testing.T) {
	tests := []struct {
		name           string
		setupMock      func(m *mockTenantService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "获取租户列表成功",
			setupMock: func(m *mockTenantService) {
				m.getTenantsFunc = func() ([]models.Tenant, error) {
					return []models.Tenant{{ID: 1, Name: "tenant-a"}, {ID: 2, Name: "tenant-b"}}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "空列表",
			setupMock: func(m *mockTenantService) {
				m.getTenantsFunc = func() ([]models.Tenant, error) {
					return []models.Tenant{}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "服务层错误",
			setupMock: func(m *mockTenantService) {
				m.getTenantsFunc = func() ([]models.Tenant, error) {
					return nil, errors.New("db error")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockTenantService{}
			tc.setupMock(mock)
			controller := NewTenantController(mock)

			c, w := newTestContext(http.MethodGet, "/api/v1/tenants", nil)
			controller.GetTenants(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestTenantController_GetTenant(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockTenantService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:    "获取单个租户成功",
			idParam: "1",
			setupMock: func(m *mockTenantService) {
				m.getTenantFunc = func(id uint) (*models.Tenant, error) {
					return &models.Tenant{ID: 1, Name: "tenant-a", Status: "active"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "xyz",
			setupMock:      func(m *mockTenantService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:    "租户不存在",
			idParam: "999",
			setupMock: func(m *mockTenantService) {
				m.getTenantFunc = func(id uint) (*models.Tenant, error) {
					return nil, errors.New("tenant not found")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockTenantService{}
			tc.setupMock(mock)
			controller := NewTenantController(mock)

			c, w := newTestContextWithParams(http.MethodGet, "/api/v1/tenants/"+tc.idParam, nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.GetTenant(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestTenantController_CreateTenant(t *testing.T) {
	tests := []struct {
		name           string
		requestBody    interface{}
		setupMock      func(m *mockTenantService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "创建租户成功",
			requestBody: map[string]interface{}{
				"name":       "new-tenant",
				"gpu_quota":  10,
				"cpu_quota":  20,
			},
			setupMock: func(m *mockTenantService) {
				m.createTenantFunc = func(req services.CreateTenantRequest) (*models.Tenant, error) {
					assert.Equal(t, "new-tenant", req.Name)
					return &models.Tenant{ID: 1, Name: "new-tenant", Status: "active"}, nil
				}
			},
			expectedStatus: http.StatusCreated,
			expectSuccess:  true,
		},
		{
			name:           "缺少必填字段-name",
			requestBody:    map[string]interface{}{"gpu_quota": 5},
			setupMock:      func(m *mockTenantService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name: "负值被拒绝-gpu_quota",
			requestBody: map[string]interface{}{
				"name":      "bad-tenant",
				"gpu_quota": -1,
			},
			setupMock:      func(m *mockTenantService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name: "名称已存在",
			requestBody: map[string]interface{}{
				"name": "existing-tenant",
			},
			setupMock: func(m *mockTenantService) {
				m.createTenantFunc = func(req services.CreateTenantRequest) (*models.Tenant, error) {
					return nil, errors.New("tenant name already exists")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockTenantService{}
			tc.setupMock(mock)
			controller := NewTenantController(mock)

			c, w := newTestContext(http.MethodPost, "/api/v1/tenants", tc.requestBody)
			controller.CreateTenant(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestTenantController_UpdateTenant(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		requestBody    interface{}
		setupMock      func(m *mockTenantService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:        "更新租户成功",
			idParam:     "1",
			requestBody: map[string]interface{}{"name": "updated-tenant"},
			setupMock: func(m *mockTenantService) {
				m.updateTenantFunc = func(id uint, req services.UpdateTenantRequest) (*models.Tenant, error) {
					return &models.Tenant{ID: 1, Name: "updated-tenant"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "bad",
			requestBody:    map[string]interface{}{"name": "test"},
			setupMock:      func(m *mockTenantService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:        "租户不存在",
			idParam:     "999",
			requestBody: map[string]interface{}{"name": "test"},
			setupMock: func(m *mockTenantService) {
				m.updateTenantFunc = func(id uint, req services.UpdateTenantRequest) (*models.Tenant, error) {
					return nil, errors.New("tenant not found")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockTenantService{}
			tc.setupMock(mock)
			controller := NewTenantController(mock)

			c, w := newTestContextWithParams(http.MethodPut, "/api/v1/tenants/"+tc.idParam, tc.requestBody,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.UpdateTenant(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestTenantController_DeleteTenant(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockTenantService)
		expectedStatus int
	}{
		{
			name:    "删除租户成功-204",
			idParam: "1",
			setupMock: func(m *mockTenantService) {
				m.deleteTenantFunc = func(id uint) error {
					return nil
				}
			},
			expectedStatus: http.StatusNoContent,
		},
		{
			name:           "ID无效",
			idParam:        "abc",
			setupMock:      func(m *mockTenantService) {},
			expectedStatus: http.StatusInternalServerError,
		},
		{
			name:    "租户不存在",
			idParam: "999",
			setupMock: func(m *mockTenantService) {
				m.deleteTenantFunc = func(id uint) error {
					return appErrors.NotFound("tenant not found")
				}
			},
			expectedStatus: http.StatusNotFound,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockTenantService{}
			tc.setupMock(mock)
			controller := NewTenantController(mock)

			c, w := newTestContextWithParams(http.MethodDelete, "/api/v1/tenants/"+tc.idParam, nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.DeleteTenant(c)

			assert.Equal(t, tc.expectedStatus, c.Writer.Status())
			_ = w
		})
	}
}
