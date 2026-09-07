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

func TestClusterController_GetClusters(t *testing.T) {
	tests := []struct {
		name           string
		setupMock      func(m *mockClusterService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "获取集群列表成功",
			setupMock: func(m *mockClusterService) {
				m.getClustersFunc = func() ([]models.Cluster, error) {
					return []models.Cluster{{ID: 1, Name: "cluster-1"}, {ID: 2, Name: "cluster-2"}}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "空列表-返回空数组",
			setupMock: func(m *mockClusterService) {
				m.getClustersFunc = func() ([]models.Cluster, error) {
					return []models.Cluster{}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name: "服务层错误",
			setupMock: func(m *mockClusterService) {
				m.getClustersFunc = func() ([]models.Cluster, error) {
					return nil, errors.New("database error")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockClusterService{}
			tc.setupMock(mock)
			controller := NewClusterController(mock)

			c, w := newTestContext(http.MethodGet, "/api/v1/clusters", nil)
			controller.GetClusters(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestClusterController_GetCluster(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockClusterService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:    "获取单个集群成功",
			idParam: "1",
			setupMock: func(m *mockClusterService) {
				m.getClusterFunc = func(id uint) (*models.Cluster, error) {
					assert.Equal(t, uint(1), id)
					return &models.Cluster{ID: 1, Name: "cluster-1", Status: "active"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID参数无效-非数字",
			idParam:        "abc",
			setupMock:      func(m *mockClusterService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:    "集群不存在",
			idParam: "999",
			setupMock: func(m *mockClusterService) {
				m.getClusterFunc = func(id uint) (*models.Cluster, error) {
					return nil, errors.New("cluster not found")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockClusterService{}
			tc.setupMock(mock)
			controller := NewClusterController(mock)

			c, w := newTestContextWithParams(http.MethodGet, "/api/v1/clusters/"+tc.idParam, nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.GetCluster(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestClusterController_CreateCluster(t *testing.T) {
	tests := []struct {
		name           string
		requestBody    interface{}
		setupMock      func(m *mockClusterService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "创建集群成功",
			requestBody: map[string]interface{}{
				"name":  "new-cluster",
				"nodes": 3,
				"gpus":  8,
			},
			setupMock: func(m *mockClusterService) {
				m.createClusterFunc = func(req services.CreateClusterRequest) (*models.Cluster, error) {
					assert.Equal(t, "new-cluster", req.Name)
					return &models.Cluster{ID: 1, Name: "new-cluster", Status: "active"}, nil
				}
			},
			expectedStatus: http.StatusCreated,
			expectSuccess:  true,
		},
		{
			name:           "缺少必填字段-name",
			requestBody:    map[string]interface{}{"nodes": 3},
			setupMock:      func(m *mockClusterService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name: "负值被拒绝-nodes",
			requestBody: map[string]interface{}{
				"name":  "bad-cluster",
				"nodes": -1,
			},
			setupMock:      func(m *mockClusterService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name: "服务层冲突-名称已存在",
			requestBody: map[string]interface{}{
				"name": "existing-cluster",
			},
			setupMock: func(m *mockClusterService) {
				m.createClusterFunc = func(req services.CreateClusterRequest) (*models.Cluster, error) {
					return nil, errors.New("cluster name already exists")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockClusterService{}
			tc.setupMock(mock)
			controller := NewClusterController(mock)

			c, w := newTestContext(http.MethodPost, "/api/v1/clusters", tc.requestBody)
			controller.CreateCluster(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestClusterController_UpdateCluster(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		requestBody    interface{}
		setupMock      func(m *mockClusterService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:        "更新集群成功",
			idParam:     "1",
			requestBody: map[string]interface{}{"name": "updated-cluster"},
			setupMock: func(m *mockClusterService) {
				m.updateClusterFunc = func(id uint, req services.UpdateClusterRequest) (*models.Cluster, error) {
					assert.Equal(t, uint(1), id)
					return &models.Cluster{ID: 1, Name: "updated-cluster"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "ID无效",
			idParam:        "invalid",
			requestBody:    map[string]interface{}{"name": "test"},
			setupMock:      func(m *mockClusterService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name:        "集群不存在",
			idParam:     "999",
			requestBody: map[string]interface{}{"name": "test"},
			setupMock: func(m *mockClusterService) {
				m.updateClusterFunc = func(id uint, req services.UpdateClusterRequest) (*models.Cluster, error) {
					return nil, errors.New("cluster not found")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockClusterService{}
			tc.setupMock(mock)
			controller := NewClusterController(mock)

			c, w := newTestContextWithParams(http.MethodPut, "/api/v1/clusters/"+tc.idParam, tc.requestBody,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.UpdateCluster(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestClusterController_DeleteCluster(t *testing.T) {
	tests := []struct {
		name           string
		idParam        string
		setupMock      func(m *mockClusterService)
		expectedStatus int
	}{
		{
			name:    "删除集群成功-返回204",
			idParam: "1",
			setupMock: func(m *mockClusterService) {
				m.deleteClusterFunc = func(id uint) error {
					assert.Equal(t, uint(1), id)
					return nil
				}
			},
			expectedStatus: http.StatusNoContent,
		},
		{
			name:           "ID无效",
			idParam:        "abc",
			setupMock:      func(m *mockClusterService) {},
			expectedStatus: http.StatusInternalServerError,
		},
		{
			name:    "集群不存在",
			idParam: "999",
			setupMock: func(m *mockClusterService) {
				m.deleteClusterFunc = func(id uint) error {
					return appErrors.NotFound("cluster not found")
				}
			},
			expectedStatus: http.StatusNotFound,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockClusterService{}
			tc.setupMock(mock)
			controller := NewClusterController(mock)

			c, w := newTestContextWithParams(http.MethodDelete, "/api/v1/clusters/"+tc.idParam, nil,
				gin.Params{{Key: "id", Value: tc.idParam}})
			controller.DeleteCluster(c)

			assert.Equal(t, tc.expectedStatus, c.Writer.Status())
			_ = w
		})
	}
}
