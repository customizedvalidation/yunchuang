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

func TestSecurityController_GetSecurityPolicies(t *testing.T) {
	tests := []struct {
		name           string
		setupMock      func(m *mockSecurityService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "获取安全策略列表成功",
			setupMock: func(m *mockSecurityService) {
				m.getSecurityPoliciesFunc = func() ([]models.SecurityPolicy, error) {
					return []models.SecurityPolicy{
						{ID: 1, Name: "default-policy", Type: "network", Status: "active"},
					}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "空列表",
			setupMock: func(m *mockSecurityService) {
				m.getSecurityPoliciesFunc = func() ([]models.SecurityPolicy, error) {
					return []models.SecurityPolicy{}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "服务层错误",
			setupMock: func(m *mockSecurityService) {
				m.getSecurityPoliciesFunc = func() ([]models.SecurityPolicy, error) {
					return nil, errors.New("db error")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockSecurityService{}
			tc.setupMock(mock)
			controller := NewSecurityController(mock)

			c, w := newTestContext(http.MethodGet, "/api/v1/security/policies", nil)
			controller.GetSecurityPolicies(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestSecurityController_GetSecurityPolicy(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockSecurityService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:    "获取单个安全策略成功",
			idParam: "1",
			setupMock: func(m *mockSecurityService) {
				m.getSecurityPolicyFunc = func(id uint) (*models.SecurityPolicy, error) {
					return &models.SecurityPolicy{ID: 1, Name: "default-policy", Status: "active"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "xyz",
			setupMock:      func(m *mockSecurityService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:    "策略不存在",
			idParam: "999",
			setupMock: func(m *mockSecurityService) {
				m.getSecurityPolicyFunc = func(id uint) (*models.SecurityPolicy, error) {
					return nil, errors.New("security policy not found")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockSecurityService{}
			tc.setupMock(mock)
			controller := NewSecurityController(mock)

			c, w := newTestContextWithParams(http.MethodGet, "/api/v1/security/policies/"+tc.idParam, nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.GetSecurityPolicy(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestSecurityController_CreateSecurityPolicy(t *testing.T) {
	tests := []struct {
		name           string
		requestBody    interface{}
		setupMock      func(m *mockSecurityService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "创建安全策略成功",
			requestBody: map[string]interface{}{
				"name":  "new-policy",
				"type":  "network",
				"rules": "allow all",
			},
			setupMock: func(m *mockSecurityService) {
				m.createSecurityPolicyFunc = func(req services.CreateSecurityPolicyRequest) (*models.SecurityPolicy, error) {
					assert.Equal(t, "new-policy", req.Name)
					return &models.SecurityPolicy{ID: 1, Name: "new-policy", Status: "active"}, nil
				}
			},
			expectedStatus: http.StatusCreated,
			expectSuccess:  true,
		},
		{
			name:           "缺少必填字段-name",
			requestBody:    map[string]interface{}{"type": "network"},
			setupMock:      func(m *mockSecurityService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name: "名称已存在",
			requestBody: map[string]interface{}{
				"name": "existing-policy",
			},
			setupMock: func(m *mockSecurityService) {
				m.createSecurityPolicyFunc = func(req services.CreateSecurityPolicyRequest) (*models.SecurityPolicy, error) {
					return nil, errors.New("security policy name already exists")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockSecurityService{}
			tc.setupMock(mock)
			controller := NewSecurityController(mock)

			c, w := newTestContext(http.MethodPost, "/api/v1/security/policies", tc.requestBody)
			controller.CreateSecurityPolicy(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestSecurityController_UpdateSecurityPolicy(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		requestBody    interface{}
		setupMock      func(m *mockSecurityService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:        "更新安全策略成功",
			idParam:     "1",
			requestBody: map[string]interface{}{"name": "updated-policy"},
			setupMock: func(m *mockSecurityService) {
				m.updateSecurityPolicyFunc = func(id uint, req services.UpdateSecurityPolicyRequest) (*models.SecurityPolicy, error) {
					return &models.SecurityPolicy{ID: 1, Name: "updated-policy"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "bad",
			requestBody:    map[string]interface{}{"name": "test"},
			setupMock:      func(m *mockSecurityService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:        "策略不存在",
			idParam:     "999",
			requestBody: map[string]interface{}{"name": "test"},
			setupMock: func(m *mockSecurityService) {
				m.updateSecurityPolicyFunc = func(id uint, req services.UpdateSecurityPolicyRequest) (*models.SecurityPolicy, error) {
					return nil, errors.New("security policy not found")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockSecurityService{}
			tc.setupMock(mock)
			controller := NewSecurityController(mock)

			c, w := newTestContextWithParams(http.MethodPut, "/api/v1/security/policies/"+tc.idParam, tc.requestBody,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.UpdateSecurityPolicy(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestSecurityController_DeleteSecurityPolicy(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockSecurityService)
		expectedStatus int
	}{
		{
			name:    "删除安全策略成功-204",
			idParam: "1",
			setupMock: func(m *mockSecurityService) {
				m.deleteSecurityPolicyFunc = func(id uint) error {
					return nil
				}
			},
			expectedStatus: http.StatusNoContent,
		},
		{
			name:           "ID无效",
			idParam:        "abc",
			setupMock:      func(m *mockSecurityService) {},
			expectedStatus: http.StatusInternalServerError,
		},
		{
			name:    "策略不存在",
			idParam: "999",
			setupMock: func(m *mockSecurityService) {
				m.deleteSecurityPolicyFunc = func(id uint) error {
					return appErrors.NotFound("security policy not found")
				}
			},
			expectedStatus: http.StatusNotFound,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockSecurityService{}
			tc.setupMock(mock)
			controller := NewSecurityController(mock)

			c, w := newTestContextWithParams(http.MethodDelete, "/api/v1/security/policies/"+tc.idParam, nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.DeleteSecurityPolicy(c)

			assert.Equal(t, tc.expectedStatus, c.Writer.Status())
			_ = w
		})
	}
}
