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

func TestJobController_GetJobs(t *testing.T) {
	tests := []struct {
		name           string
		tenantID       uint
		role           string
		setupMock      func(m *mockJobService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:     "管理员获取全部作业",
			tenantID: 0,
			role:     "admin",
			setupMock: func(m *mockJobService) {
				m.getJobsVisibleToFunc = func(tenantID uint, isAdmin bool) ([]models.Job, error) {
					assert.True(t, isAdmin)
					return []models.Job{{ID: 1, Name: "job-1"}, {ID: 2, Name: "job-2"}}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:     "普通用户获取本租户作业",
			tenantID: 5,
			role:     "user",
			setupMock: func(m *mockJobService) {
				m.getJobsVisibleToFunc = func(tenantID uint, isAdmin bool) ([]models.Job, error) {
					assert.False(t, isAdmin)
					assert.Equal(t, uint(5), tenantID)
					return []models.Job{{ID: 3, Name: "tenant-job"}}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:     "空列表",
			tenantID: 1,
			role:     "user",
			setupMock: func(m *mockJobService) {
				m.getJobsVisibleToFunc = func(tenantID uint, isAdmin bool) ([]models.Job, error) {
					return []models.Job{}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:     "服务层错误",
			tenantID: 1,
			role:     "user",
			setupMock: func(m *mockJobService) {
				m.getJobsVisibleToFunc = func(tenantID uint, isAdmin bool) ([]models.Job, error) {
					return nil, errors.New("db error")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockJobService{}
			tc.setupMock(mock)
			controller := NewJobController(mock)

			c, w := newTestContextWithAuth(http.MethodGet, "/api/v1/jobs", nil, 1, tc.tenantID, tc.role)
			controller.GetJobs(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestJobController_GetJob(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		tenantID       uint
		role           string
		setupMock      func(m *mockJobService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:     "获取作业成功",
			idParam:  "1",
			tenantID: 1,
			role:     "user",
			setupMock: func(m *mockJobService) {
				m.getJobVisibleToFunc = func(id, tenantID uint, isAdmin bool) (*models.Job, error) {
					assert.Equal(t, uint(1), id)
					return &models.Job{ID: 1, Name: "job-1", Status: "running"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "abc",
			tenantID:       1,
			role:           "user",
			setupMock:      func(m *mockJobService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name:     "越权访问-返回404",
			idParam:  "5",
			tenantID: 1,
			role:     "user",
			setupMock: func(m *mockJobService) {
				m.getJobVisibleToFunc = func(id, tenantID uint, isAdmin bool) (*models.Job, error) {
					return nil, appErrors.NotFound("job not found")
				}
			},
			expectedStatus: http.StatusNotFound,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockJobService{}
			tc.setupMock(mock)
			controller := NewJobController(mock)

			c, w := newTestContextWithAuth(http.MethodGet, "/api/v1/jobs/"+tc.idParam, nil, 1, tc.tenantID, tc.role)
			c.Params = gin.Params{{Key: "id", Value: tc.idParam}}
			controller.GetJob(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestJobController_CreateJob(t *testing.T) {
	tests := []struct {
		name           string
		requestBody    interface{}
		tenantID       uint
		userID         uint
		role           string
		setupMock      func(m *mockJobService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "创建作业成功",
			requestBody: map[string]interface{}{
				"name": "new-job",
				"gpus": 2,
			},
			tenantID: 1,
			userID:   5,
			role:     "user",
			setupMock: func(m *mockJobService) {
				m.createJobForUserFunc = func(req services.CreateJobRequest, tenantID, userID uint, isAdmin bool) (*models.Job, error) {
					assert.Equal(t, "new-job", req.Name)
					assert.Equal(t, uint(1), tenantID)
					assert.Equal(t, uint(5), userID)
					return &models.Job{ID: 1, Name: "new-job", Status: "pending"}, nil
				}
			},
			expectedStatus: http.StatusCreated,
			expectSuccess:  true,
		},
		{
			name:           "缺少必填字段-name",
			requestBody:    map[string]interface{}{"gpus": 1},
			tenantID:       1,
			userID:         1,
			role:           "user",
			setupMock:      func(m *mockJobService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name: "负值被拒绝-gpus",
			requestBody: map[string]interface{}{
				"name": "bad-job",
				"gpus": -1,
			},
			tenantID:       1,
			userID:         1,
			role:           "user",
			setupMock:      func(m *mockJobService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name: "服务层错误-配额不足",
			requestBody: map[string]interface{}{
				"name": "big-job",
				"gpus": 100,
			},
			tenantID: 1,
			userID:   1,
			role:     "user",
			setupMock: func(m *mockJobService) {
				m.createJobForUserFunc = func(req services.CreateJobRequest, tenantID, userID uint, isAdmin bool) (*models.Job, error) {
					return nil, appErrors.BadRequest("GPU quota exceeded")
				}
			},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockJobService{}
			tc.setupMock(mock)
			controller := NewJobController(mock)

			c, w := newTestContextWithAuth(http.MethodPost, "/api/v1/jobs", tc.requestBody, tc.userID, tc.tenantID, tc.role)
			controller.CreateJob(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestJobController_UpdateJob(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		requestBody    interface{}
		tenantID       uint
		role           string
		setupMock      func(m *mockJobService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:        "更新作业成功",
			idParam:     "1",
			requestBody: map[string]interface{}{"name": "updated-job"},
			tenantID:    1,
			role:        "user",
			setupMock: func(m *mockJobService) {
				m.updateJobForTenantFunc = func(id, tenantID uint, isAdmin bool, req services.UpdateJobRequest) (*models.Job, error) {
					assert.Equal(t, uint(1), id)
					return &models.Job{ID: 1, Name: "updated-job"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "bad",
			requestBody:    map[string]interface{}{"name": "test"},
			tenantID:       1,
			role:           "user",
			setupMock:      func(m *mockJobService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name:        "越权更新-返回404",
			idParam:     "5",
			requestBody: map[string]interface{}{"name": "test"},
			tenantID:    1,
			role:        "user",
			setupMock: func(m *mockJobService) {
				m.updateJobForTenantFunc = func(id, tenantID uint, isAdmin bool, req services.UpdateJobRequest) (*models.Job, error) {
					return nil, appErrors.NotFound("job not found")
				}
			},
			expectedStatus: http.StatusNotFound,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockJobService{}
			tc.setupMock(mock)
			controller := NewJobController(mock)

			c, w := newTestContextWithAuth(http.MethodPut, "/api/v1/jobs/"+tc.idParam, tc.requestBody, 1, tc.tenantID, tc.role)
			c.Params = gin.Params{{Key: "id", Value: tc.idParam}}
			controller.UpdateJob(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestJobController_DeleteJob(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		tenantID       uint
		role           string
		setupMock      func(m *mockJobService)
		expectedStatus int
	}{
		{
			name:     "删除作业成功-204",
			idParam:  "1",
			tenantID: 1,
			role:     "user",
			setupMock: func(m *mockJobService) {
				m.deleteJobForTenantFunc = func(id, tenantID uint, isAdmin bool) error {
					return nil
				}
			},
			expectedStatus: http.StatusNoContent,
		},
		{
			name:           "ID无效",
			idParam:        "xyz",
			tenantID:       1,
			role:           "user",
			setupMock:      func(m *mockJobService) {},
			expectedStatus: http.StatusBadRequest,
		},
		{
			name:     "作业不存在",
			idParam:  "999",
			tenantID: 1,
			role:     "user",
			setupMock: func(m *mockJobService) {
				m.deleteJobForTenantFunc = func(id, tenantID uint, isAdmin bool) error {
					return appErrors.NotFound("job not found")
				}
			},
			expectedStatus: http.StatusNotFound,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockJobService{}
			tc.setupMock(mock)
			controller := NewJobController(mock)

			c, w := newTestContextWithAuth(http.MethodDelete, "/api/v1/jobs/"+tc.idParam, nil, 1, tc.tenantID, tc.role)
			c.Params = gin.Params{{Key: "id", Value: tc.idParam}}
			controller.DeleteJob(c)

			assert.Equal(t, tc.expectedStatus, c.Writer.Status())
			_ = w
		})
	}
}

func TestJobController_CancelJob(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		tenantID       uint
		role           string
		setupMock      func(m *mockJobService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:     "取消作业成功",
			idParam:  "1",
			tenantID: 1,
			role:     "user",
			setupMock: func(m *mockJobService) {
				m.cancelJobForTenantFunc = func(id, tenantID uint, isAdmin bool) (*models.Job, error) {
					return &models.Job{ID: 1, Name: "job-1", Status: "cancelled"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "bad",
			tenantID:       1,
			role:           "user",
			setupMock:      func(m *mockJobService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name:     "作业状态不允许取消",
			idParam:  "1",
			tenantID: 1,
			role:     "user",
			setupMock: func(m *mockJobService) {
				m.cancelJobForTenantFunc = func(id, tenantID uint, isAdmin bool) (*models.Job, error) {
					return nil, appErrors.BadRequest("only running or pending jobs can be cancelled")
				}
			},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockJobService{}
			tc.setupMock(mock)
			controller := NewJobController(mock)

			c, w := newTestContextWithAuth(http.MethodPost, "/api/v1/jobs/"+tc.idParam+"/cancel", nil, 1, tc.tenantID, tc.role)
			c.Params = gin.Params{{Key: "id", Value: tc.idParam}}
			controller.CancelJob(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestActorFromContext(t *testing.T) {
	tests := []struct {
		name            string
		setCtx          func(c *gin.Context)
		expectedTenant  uint
		expectedUser    uint
		expectedIsAdmin bool
	}{
		{
			name: "完整上下文-管理员",
			setCtx: func(c *gin.Context) {
				c.Set("tenant_id", uint(0))
				c.Set("user_id", uint(1))
				c.Set("role", "admin")
			},
			expectedTenant:  0,
			expectedUser:    1,
			expectedIsAdmin: true,
		},
		{
			name: "完整上下文-普通用户",
			setCtx: func(c *gin.Context) {
				c.Set("tenant_id", uint(5))
				c.Set("user_id", uint(10))
				c.Set("role", "user")
			},
			expectedTenant:  5,
			expectedUser:    10,
			expectedIsAdmin: false,
		},
		{
			name:            "空上下文",
			setCtx:          func(c *gin.Context) {},
			expectedTenant:  0,
			expectedUser:    0,
			expectedIsAdmin: false,
		},
		{
			name: "类型错误-tenant_id",
			setCtx: func(c *gin.Context) {
				c.Set("tenant_id", "not-uint")
				c.Set("user_id", uint(1))
			},
			expectedTenant:  0,
			expectedUser:    1,
			expectedIsAdmin: false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			c, _ := gin.CreateTestContext(nil)
			tc.setCtx(c)
			tenantID, userID, isAdmin := actorFromContext(c)
			assert.Equal(t, tc.expectedTenant, tenantID)
			assert.Equal(t, tc.expectedUser, userID)
			assert.Equal(t, tc.expectedIsAdmin, isAdmin)
		})
	}
}

func TestParseID(t *testing.T) {
	tests := []struct {
		name      string
		idParam   string
		expected  uint
		expectOK  bool
	}{
		{"有效ID", "42", 42, true},
		{"零值", "0", 0, true},
		{"非数字", "abc", 0, false},
		{"负数", "-1", 0, false},
		{"空字符串", "", 0, false},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			c, w := newTestContextWithParams(http.MethodGet, "/test/"+tc.idParam, nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			id, ok := parseID(c)
			assert.Equal(t, tc.expectOK, ok)
			if tc.expectOK {
				assert.Equal(t, tc.expected, id)
			} else {
				assert.Equal(t, http.StatusBadRequest, w.Code)
			}
		})
	}
}
