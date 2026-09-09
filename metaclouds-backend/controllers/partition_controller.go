package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/models"
	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type PartitionController struct {
	partitionService PartitionServiceInterface
}

func NewPartitionController(partitionService PartitionServiceInterface) *PartitionController {
	return &PartitionController{partitionService: partitionService}
}

func (c *PartitionController) GetPartitions(ctx *gin.Context) {
	clusterID, _ := strconv.ParseUint(ctx.Query("cluster_id"), 10, 32)
	status := ctx.Query("status")

	partitions, err := c.partitionService.ListPartitions(uint(clusterID), status)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, partitions)
}

func (c *PartitionController) GetPartition(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	partition, err := c.partitionService.GetPartition(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, partition)
}

func (c *PartitionController) CreatePartition(ctx *gin.Context) {
	var partition models.Partition
	if err := ctx.ShouldBindJSON(&partition); err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.partitionService.CreatePartition(&partition); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Created(ctx, partition)
}

func (c *PartitionController) UpdatePartition(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	var partition models.Partition
	if err := ctx.ShouldBindJSON(&partition); err != nil {
		response.Error(ctx, err)
		return
	}
	partition.ID = uint(id)
	if err := c.partitionService.UpdatePartition(&partition); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, partition)
}

func (c *PartitionController) DeletePartition(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.partitionService.DeletePartition(uint(id)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.NoContent(ctx)
}

type UpdatePriorityRequest struct {
	Priority int `json:"priority" binding:"required"`
}

func (c *PartitionController) UpdatePriority(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	var req UpdatePriorityRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.partitionService.UpdatePartitionPriority(uint(id), req.Priority); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, gin.H{"message": "priority updated"})
}

type UpdateMaxRuntimeRequest struct {
	MaxRuntime int `json:"max_runtime_minutes" binding:"required"`
}

func (c *PartitionController) UpdateMaxRuntime(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	var req UpdateMaxRuntimeRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.partitionService.UpdatePartitionMaxRuntime(uint(id), req.MaxRuntime); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, gin.H{"message": "max runtime updated"})
}

func (c *PartitionController) GetPermissions(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	permissions, err := c.partitionService.GetPartitionPermissions(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, permissions)
}

type SetPermissionRequest struct {
	PrincipalType string `json:"principal_type" binding:"required"`
	PrincipalID   uint   `json:"principal_id" binding:"required"`
	AccessLevel   string `json:"access_level" binding:"required"`
}

func (c *PartitionController) SetPermission(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	var req SetPermissionRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.partitionService.SetPartitionPermission(uint(id), req.PrincipalType, req.PrincipalID, req.AccessLevel); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Created(ctx, gin.H{"message": "permission set"})
}

func (c *PartitionController) RemovePermission(ctx *gin.Context) {
	permID, err := strconv.ParseUint(ctx.Param("permId"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.partitionService.RemovePartitionPermission(uint(permID)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.NoContent(ctx)
}

// PartitionServiceInterface 定义分区服务接口
type PartitionServiceInterface interface {
	ListPartitions(clusterID uint, status string) ([]models.Partition, error)
	GetPartition(id uint) (*models.Partition, error)
	CreatePartition(partition *models.Partition) error
	UpdatePartition(partition *models.Partition) error
	DeletePartition(id uint) error
	UpdatePartitionPriority(id uint, priority int) error
	UpdatePartitionMaxRuntime(id uint, maxRuntime int) error
	SetPartitionPermission(partitionID uint, principalType string, principalID uint, accessLevel string) error
	RemovePartitionPermission(id uint) error
	GetPartitionPermissions(partitionID uint) ([]models.PartitionPermission, error)
	CheckPartitionAccess(userID uint, partitionID uint, requiredLevel string) (bool, error)
}

var _ PartitionServiceInterface = (*services.PartitionService)(nil)
