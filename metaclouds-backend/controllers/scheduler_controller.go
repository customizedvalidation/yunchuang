package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/models"
	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type SchedulerController struct {
	schedulerService SchedulerServiceInterface
}

func NewSchedulerController(schedulerService SchedulerServiceInterface) *SchedulerController {
	return &SchedulerController{schedulerService: schedulerService}
}

func (c *SchedulerController) GetSchedulerIntegrations(ctx *gin.Context) {
	integrations, err := c.schedulerService.ListSchedulerIntegrations()
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, integrations)
}

func (c *SchedulerController) GetSchedulerIntegration(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	integration, err := c.schedulerService.GetSchedulerIntegration(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, integration)
}

func (c *SchedulerController) CreateSchedulerIntegration(ctx *gin.Context) {
	var integration models.SchedulerIntegration
	if err := ctx.ShouldBindJSON(&integration); err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.schedulerService.CreateSchedulerIntegration(&integration); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Created(ctx, integration)
}

func (c *SchedulerController) UpdateSchedulerIntegration(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	var integration models.SchedulerIntegration
	if err := ctx.ShouldBindJSON(&integration); err != nil {
		response.Error(ctx, err)
		return
	}
	integration.ID = uint(id)
	if err := c.schedulerService.UpdateSchedulerIntegration(&integration); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, integration)
}

func (c *SchedulerController) DeleteSchedulerIntegration(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.schedulerService.DeleteSchedulerIntegration(uint(id)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.NoContent(ctx)
}

func (c *SchedulerController) GetQueues(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	queues, err := c.schedulerService.GetQueues(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, queues)
}

func (c *SchedulerController) GetNodes(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	nodes, err := c.schedulerService.GetNodes(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, nodes)
}

func (c *SchedulerController) SyncJobs(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	results, err := c.schedulerService.SyncJobs(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, results)
}

func (c *SchedulerController) HealthCheck(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	healthy, err := c.schedulerService.HealthCheck(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, gin.H{"healthy": healthy})
}

// SchedulerServiceInterface 定义调度器服务接口
type SchedulerServiceInterface interface {
	ListSchedulerIntegrations() ([]models.SchedulerIntegration, error)
	GetSchedulerIntegration(id uint) (*models.SchedulerIntegration, error)
	CreateSchedulerIntegration(integration *models.SchedulerIntegration) error
	UpdateSchedulerIntegration(integration *models.SchedulerIntegration) error
	DeleteSchedulerIntegration(id uint) error
	GetQueues(id uint) ([]services.QueueInfo, error)
	GetNodes(id uint) ([]services.NodeInfo, error)
	SyncJobs(id uint) ([]services.JobSyncResult, error)
	HealthCheck(id uint) (bool, error)
}

var _ SchedulerServiceInterface = (*services.SchedulerService)(nil)
