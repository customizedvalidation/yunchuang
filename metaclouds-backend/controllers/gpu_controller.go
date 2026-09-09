package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/models"
	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type GPUController struct {
	gpuService GPUServiceInterface
}

func NewGPUController(gpuService GPUServiceInterface) *GPUController {
	return &GPUController{gpuService: gpuService}
}

func (c *GPUController) GetGPUDevices(ctx *gin.Context) {
	clusterID, _ := strconv.ParseUint(ctx.Query("cluster_id"), 10, 32)
	vendor := ctx.Query("vendor")
	status := ctx.Query("status")

	devices, err := c.gpuService.ListGPUDevices(uint(clusterID), vendor, status)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, devices)
}

func (c *GPUController) GetGPUDevice(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	device, err := c.gpuService.GetGPUDevice(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, device)
}

func (c *GPUController) CreateGPUDevice(ctx *gin.Context) {
	var device models.GPUDevice
	if err := ctx.ShouldBindJSON(&device); err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.gpuService.CreateGPUDevice(&device); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Created(ctx, device)
}

func (c *GPUController) UpdateGPUDevice(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	var device models.GPUDevice
	if err := ctx.ShouldBindJSON(&device); err != nil {
		response.Error(ctx, err)
		return
	}
	device.ID = uint(id)
	if err := c.gpuService.UpdateGPUDevice(&device); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, device)
}

func (c *GPUController) DeleteGPUDevice(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.gpuService.DeleteGPUDevice(uint(id)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.NoContent(ctx)
}

func (c *GPUController) GetGPUAllocations(ctx *gin.Context) {
	jobID, _ := strconv.ParseUint(ctx.Query("job_id"), 10, 32)
	allocations, err := c.gpuService.GetGPUAllocations(uint(jobID))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, allocations)
}

type AllocateGPURequest struct {
	JobID    uint    `json:"job_id" binding:"required"`
	TenantID uint    `json:"tenant_id"`
	UserID   uint    `json:"user_id"`
	Fraction float64 `json:"fraction" binding:"required"`
	MemoryGB int     `json:"memory_gb"`
	Vendor   string  `json:"vendor"`
}

func (c *GPUController) AllocateGPU(ctx *gin.Context) {
	var req AllocateGPURequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}
	allocation, err := c.gpuService.AllocateGPU(req.JobID, req.TenantID, req.UserID, req.Fraction, req.MemoryGB, req.Vendor)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Created(ctx, allocation)
}

func (c *GPUController) ReleaseGPU(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.gpuService.ReleaseGPU(uint(id)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.NoContent(ctx)
}

func (c *GPUController) GetGPUUtilization(ctx *gin.Context) {
	clusterID, _ := strconv.ParseUint(ctx.Query("cluster_id"), 10, 32)
	summary, err := c.gpuService.GetGPUUtilizationSummary(uint(clusterID))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, summary)
}

// GPUServiceInterface 定义 GPU 服务接口
type GPUServiceInterface interface {
	ListGPUDevices(clusterID uint, vendor string, status string) ([]models.GPUDevice, error)
	GetGPUDevice(id uint) (*models.GPUDevice, error)
	CreateGPUDevice(device *models.GPUDevice) error
	UpdateGPUDevice(device *models.GPUDevice) error
	DeleteGPUDevice(id uint) error
	AllocateGPU(jobID, tenantID, userID uint, fraction float64, memoryGB int, vendor string) (*models.GPUAllocation, error)
	ReleaseGPU(allocationID uint) error
	GetGPUAllocations(jobID uint) ([]models.GPUAllocation, error)
	GetGPUUtilizationSummary(clusterID uint) (map[string]interface{}, error)
}

// 编译期检查：确保 services.GPUService 实现接口
var _ GPUServiceInterface = (*services.GPUService)(nil)
