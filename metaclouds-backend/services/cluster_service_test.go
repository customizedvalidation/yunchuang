package services

import (
	"sort"
	"testing"

	"github.com/stretchr/testify/assert"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
)

func TestClusterService_GetClusters(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	cluster1 := &models.Cluster{
		ID:          1,
		Name:        "Cluster 1",
		Description: "Test cluster",
		Status:      "running",
		Nodes:       3,
		GPUs:        6,
	}
	cluster2 := &models.Cluster{
		ID:          2,
		Name:        "Cluster 2",
		Description: "Another cluster",
		Status:      "stopped",
		Nodes:       2,
		GPUs:        4,
	}

	db.Mu.Lock()
	db.Clusters[1] = cluster1
	db.Clusters[2] = cluster2
	db.Mu.Unlock()

	service := NewClusterService(db, cfg)

	clusters, err := service.GetClusters()

	assert.NoError(t, err)
	assert.Len(t, clusters, 2)

	// GetClusters 遍历 map，返回顺序不固定；断言名称集合而非下标顺序。
	names := []string{clusters[0].Name, clusters[1].Name}
	sort.Strings(names)
	assert.Equal(t, []string{"Cluster 1", "Cluster 2"}, names)
}

func TestClusterService_GetCluster(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	cluster := &models.Cluster{
		ID:          1,
		Name:        "Test Cluster",
		Description: "Description",
		Status:      "running",
		Nodes:       3,
		GPUs:        6,
	}

	db.Mu.Lock()
	db.Clusters[1] = cluster
	db.Mu.Unlock()

	service := NewClusterService(db, cfg)

	result, err := service.GetCluster(1)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, "Test Cluster", result.Name)
	assert.Equal(t, 3, result.Nodes)
}

func TestClusterService_GetCluster_NotFound(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	service := NewClusterService(db, cfg)

	result, err := service.GetCluster(999)

	assert.Error(t, err)
	assert.Nil(t, result)
	assert.Contains(t, err.Error(), "cluster not found")
}

func TestClusterService_CreateCluster(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	service := NewClusterService(db, cfg)

	req := CreateClusterRequest{
		Name:        "New Cluster",
		Description: "New cluster description",
	}

	cluster, err := service.CreateCluster(req)

	assert.NoError(t, err)
	assert.NotNil(t, cluster)
	assert.Equal(t, "New Cluster", cluster.Name)
	assert.Equal(t, "New cluster description", cluster.Description)
	assert.Equal(t, "active", cluster.Status)

	db.Mu.RLock()
	_, exists := db.Clusters[cluster.ID]
	db.Mu.RUnlock()
	assert.True(t, exists)
}

func TestClusterService_UpdateCluster(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	cluster := &models.Cluster{
		ID:          1,
		Name:        "Original",
		Description: "Original desc",
		Status:      "running",
	}

	db.Mu.Lock()
	db.Clusters[1] = cluster
	db.Mu.Unlock()

	service := NewClusterService(db, cfg)

	req := UpdateClusterRequest{
		Name:        "Updated",
		Description: "Updated desc",
	}

	result, err := service.UpdateCluster(1, req)

	assert.NoError(t, err)
	assert.NotNil(t, result)
	assert.Equal(t, "Updated", result.Name)
	assert.Equal(t, "Updated desc", result.Description)
	assert.Equal(t, "running", result.Status)
}

func TestClusterService_DeleteCluster(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	cluster := &models.Cluster{
		ID:   1,
		Name: "To Delete",
	}

	db.Mu.Lock()
	db.Clusters[1] = cluster
	db.Mu.Unlock()

	service := NewClusterService(db, cfg)

	err := service.DeleteCluster(1)

	assert.NoError(t, err)

	db.Mu.RLock()
	_, exists := db.Clusters[1]
	db.Mu.RUnlock()
	assert.False(t, exists)
}

func TestClusterService_DeleteCluster_NotFound(t *testing.T) {
	cfg := &config.Config{}
	db := models.MustNewMemoryStore()

	service := NewClusterService(db, cfg)

	err := service.DeleteCluster(999)

	assert.Error(t, err)
	assert.Contains(t, err.Error(), "cluster not found")
}
