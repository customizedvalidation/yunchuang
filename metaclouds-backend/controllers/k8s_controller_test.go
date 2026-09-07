package controllers

import (
	"errors"
	"net/http"
	"testing"

	"github.com/gin-gonic/gin"
	"github.com/stretchr/testify/assert"

	appErrors "metaclouds-backend/pkg/errors"
	"metaclouds-backend/services"
)

func TestK8SController_SubmitJob(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockK8SService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:    "提交作业成功",
			idParam: "1",
			setupMock: func(m *mockK8SService) {
				m.submitJobFunc = func(req services.SubmitJobRequest) (*services.JobStatusResponse, error) {
					assert.Equal(t, uint(1), req.JobID)
					return &services.JobStatusResponse{JobID: 1, Status: "running", Message: "submitted"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID为空",
			idParam:        "",
			setupMock:      func(m *mockK8SService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:           "ID无效",
			idParam:        "abc",
			setupMock:      func(m *mockK8SService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:    "作业不存在",
			idParam: "999",
			setupMock: func(m *mockK8SService) {
				m.submitJobFunc = func(req services.SubmitJobRequest) (*services.JobStatusResponse, error) {
					return nil, appErrors.NotFound("job not found")
				}
			},
			expectedStatus: http.StatusNotFound,
			expectSuccess:  false,
		},
		{
			name:    "资源不足",
			idParam: "1",
			setupMock: func(m *mockK8SService) {
				m.submitJobFunc = func(req services.SubmitJobRequest) (*services.JobStatusResponse, error) {
					return nil, errors.New("not enough GPU resources available")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockK8SService{}
			tc.setupMock(mock)
			controller := NewK8SController(mock)

			c, w := newTestContextWithParams(http.MethodPost, "/api/v1/k8s/jobs/"+tc.idParam+"/submit", nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.SubmitJob(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestK8SController_GetJobStatus(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockK8SService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:    "获取作业状态成功",
			idParam: "1",
			setupMock: func(m *mockK8SService) {
				m.getJobStatusFunc = func(jobID uint) (*services.JobStatusResponse, error) {
					return &services.JobStatusResponse{JobID: 1, Status: "running", Message: "in progress"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "xyz",
			setupMock:      func(m *mockK8SService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:    "作业不存在",
			idParam: "999",
			setupMock: func(m *mockK8SService) {
				m.getJobStatusFunc = func(jobID uint) (*services.JobStatusResponse, error) {
					return nil, appErrors.NotFound("job not found")
				}
			},
			expectedStatus: http.StatusNotFound,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockK8SService{}
			tc.setupMock(mock)
			controller := NewK8SController(mock)

			c, w := newTestContextWithParams(http.MethodGet, "/api/v1/k8s/jobs/"+tc.idParam+"/status", nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.GetJobStatus(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestK8SController_CancelJob(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockK8SService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:    "取消K8S作业成功",
			idParam: "1",
			setupMock: func(m *mockK8SService) {
				m.cancelJobFunc = func(jobID uint) (*services.JobStatusResponse, error) {
					return &services.JobStatusResponse{JobID: 1, Status: "cancelled", Message: "cancelled"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "bad",
			setupMock:      func(m *mockK8SService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:    "作业状态不允许取消",
			idParam: "1",
			setupMock: func(m *mockK8SService) {
				m.cancelJobFunc = func(jobID uint) (*services.JobStatusResponse, error) {
					return nil, appErrors.BadRequest("only running or pending jobs can be cancelled")
				}
			},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockK8SService{}
			tc.setupMock(mock)
			controller := NewK8SController(mock)

			c, w := newTestContextWithParams(http.MethodPost, "/api/v1/k8s/jobs/"+tc.idParam+"/cancel", nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.CancelJob(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestK8SController_GetGPUResources(t *testing.T) {
	tests := []struct {
		name           string
		setupMock      func(m *mockK8SService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "获取GPU资源成功",
			setupMock: func(m *mockK8SService) {
				m.getGPUResourcesFunc = func() ([]services.GPUResource, error) {
					return []services.GPUResource{
						{ID: 1, Name: "A100-0", Status: "available", Total: 1, Available: 1},
					}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "空列表",
			setupMock: func(m *mockK8SService) {
				m.getGPUResourcesFunc = func() ([]services.GPUResource, error) {
					return []services.GPUResource{}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "服务层错误",
			setupMock: func(m *mockK8SService) {
				m.getGPUResourcesFunc = func() ([]services.GPUResource, error) {
					return nil, errors.New("k8s api error")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockK8SService{}
			tc.setupMock(mock)
			controller := NewK8SController(mock)

			c, w := newTestContext(http.MethodGet, "/api/v1/k8s/gpus", nil)
			controller.GetGPUResources(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestK8SController_GetClusterStatus(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockK8SService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:    "获取集群状态成功",
			idParam: "1",
			setupMock: func(m *mockK8SService) {
				m.getClusterStatusFunc = func(clusterID uint) (*services.ClusterStatus, error) {
					return &services.ClusterStatus{ID: 1, Name: "cluster-1", Status: "healthy", Nodes: 3}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "invalid",
			setupMock:      func(m *mockK8SService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:    "集群不存在",
			idParam: "999",
			setupMock: func(m *mockK8SService) {
				m.getClusterStatusFunc = func(clusterID uint) (*services.ClusterStatus, error) {
					return nil, appErrors.NotFound("cluster not found")
				}
			},
			expectedStatus: http.StatusNotFound,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockK8SService{}
			tc.setupMock(mock)
			controller := NewK8SController(mock)

			c, w := newTestContextWithParams(http.MethodGet, "/api/v1/k8s/clusters/"+tc.idParam+"/status", nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.GetClusterStatus(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}
