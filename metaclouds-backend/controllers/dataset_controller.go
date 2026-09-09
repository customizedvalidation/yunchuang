package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/models"
	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type DatasetController struct {
	accelerationService AccelerationExtServiceInterface
}

func NewDatasetController(accelerationService AccelerationExtServiceInterface) *DatasetController {
	return &DatasetController{accelerationService: accelerationService}
}

// --- Dataset CRUD ---

func (c *DatasetController) GetDatasets(ctx *gin.Context) {
	tenantID, _ := strconv.ParseUint(ctx.Query("tenant_id"), 10, 32)
	datasets, err := c.accelerationService.ListDatasets(uint(tenantID))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, datasets)
}

func (c *DatasetController) GetDataset(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	dataset, err := c.accelerationService.GetDataset(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, dataset)
}

func (c *DatasetController) CreateDataset(ctx *gin.Context) {
	var dataset models.Dataset
	if err := ctx.ShouldBindJSON(&dataset); err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.accelerationService.CreateDataset(&dataset); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Created(ctx, dataset)
}

func (c *DatasetController) UpdateDataset(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	var dataset models.Dataset
	if err := ctx.ShouldBindJSON(&dataset); err != nil {
		response.Error(ctx, err)
		return
	}
	dataset.ID = uint(id)
	if err := c.accelerationService.UpdateDataset(&dataset); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, dataset)
}

func (c *DatasetController) DeleteDataset(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.accelerationService.DeleteDataset(uint(id)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.NoContent(ctx)
}

// --- FluidCache CRUD ---

func (c *DatasetController) GetFluidCaches(ctx *gin.Context) {
	datasetID, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	caches, err := c.accelerationService.ListFluidCaches(uint(datasetID))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, caches)
}

func (c *DatasetController) CreateFluidCache(ctx *gin.Context) {
	datasetID, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	var cache models.FluidCache
	if err := ctx.ShouldBindJSON(&cache); err != nil {
		response.Error(ctx, err)
		return
	}
	cache.DatasetID = uint(datasetID)
	if err := c.accelerationService.CreateFluidCache(&cache); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Created(ctx, cache)
}

func (c *DatasetController) UpdateFluidCache(ctx *gin.Context) {
	cacheID, err := strconv.ParseUint(ctx.Param("cacheId"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	var cache models.FluidCache
	if err := ctx.ShouldBindJSON(&cache); err != nil {
		response.Error(ctx, err)
		return
	}
	cache.ID = uint(cacheID)
	if err := c.accelerationService.UpdateFluidCache(&cache); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, cache)
}

func (c *DatasetController) DeleteFluidCache(ctx *gin.Context) {
	cacheID, err := strconv.ParseUint(ctx.Param("cacheId"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.accelerationService.DeleteFluidCache(uint(cacheID)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.NoContent(ctx)
}

func (c *DatasetController) EnableFluidCache(ctx *gin.Context) {
	cacheID, err := strconv.ParseUint(ctx.Param("cacheId"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.accelerationService.EnableFluidCache(uint(cacheID)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, gin.H{"message": "fluid cache enabled"})
}

func (c *DatasetController) DisableFluidCache(ctx *gin.Context) {
	cacheID, err := strconv.ParseUint(ctx.Param("cacheId"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.accelerationService.DisableFluidCache(uint(cacheID)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, gin.H{"message": "fluid cache disabled"})
}

func (c *DatasetController) TriggerPrefetch(ctx *gin.Context) {
	cacheID, err := strconv.ParseUint(ctx.Param("cacheId"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.accelerationService.TriggerPrefetch(uint(cacheID)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, gin.H{"message": "prefetch triggered"})
}

// AccelerationExtServiceInterface 定义加速扩展服务接口（Dataset + FluidCache + 训练/推理配置 + Checkpoint）
type AccelerationExtServiceInterface interface {
	ListDatasets(tenantID uint) ([]models.Dataset, error)
	GetDataset(id uint) (*models.Dataset, error)
	CreateDataset(dataset *models.Dataset) error
	UpdateDataset(dataset *models.Dataset) error
	DeleteDataset(id uint) error

	ListFluidCaches(datasetID uint) ([]models.FluidCache, error)
	GetFluidCache(id uint) (*models.FluidCache, error)
	CreateFluidCache(cache *models.FluidCache) error
	UpdateFluidCache(cache *models.FluidCache) error
	DeleteFluidCache(id uint) error
	EnableFluidCache(cacheID uint) error
	DisableFluidCache(cacheID uint) error
	TriggerPrefetch(cacheID uint) error

	ListCheckpoints(jobID uint) ([]models.Checkpoint, error)
	GetCheckpoint(id uint) (*models.Checkpoint, error)
	CreateCheckpoint(checkpoint *models.Checkpoint) error
	DeleteCheckpoint(id uint) error
	GetLatestCheckpoint(jobID uint) (*models.Checkpoint, error)
}

var _ AccelerationExtServiceInterface = (*services.AccelerationService)(nil)
