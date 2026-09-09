package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/models"
	"metaclouds-backend/pkg/response"
)

type CheckpointController struct {
	accelerationService AccelerationExtServiceInterface
}

func NewCheckpointController(accelerationService AccelerationExtServiceInterface) *CheckpointController {
	return &CheckpointController{accelerationService: accelerationService}
}

func (c *CheckpointController) GetCheckpoints(ctx *gin.Context) {
	jobID, _ := strconv.ParseUint(ctx.Query("job_id"), 10, 32)
	checkpoints, err := c.accelerationService.ListCheckpoints(uint(jobID))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, checkpoints)
}

func (c *CheckpointController) CreateCheckpoint(ctx *gin.Context) {
	var checkpoint models.Checkpoint
	if err := ctx.ShouldBindJSON(&checkpoint); err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.accelerationService.CreateCheckpoint(&checkpoint); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Created(ctx, checkpoint)
}

func (c *CheckpointController) GetCheckpoint(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	checkpoint, err := c.accelerationService.GetCheckpoint(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, checkpoint)
}

func (c *CheckpointController) DeleteCheckpoint(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.accelerationService.DeleteCheckpoint(uint(id)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.NoContent(ctx)
}

func (c *CheckpointController) GetLatestCheckpoint(ctx *gin.Context) {
	jobID, err := strconv.ParseUint(ctx.Param("jobId"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	checkpoint, err := c.accelerationService.GetLatestCheckpoint(uint(jobID))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, checkpoint)
}
