package controllers

import (
	"errors"
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type K8SController struct {
	k8sService *services.K8SService
}

func NewK8SController(k8sService *services.K8SService) *K8SController {
	return &K8SController{
		k8sService: k8sService,
	}
}

func (c *K8SController) SubmitJob(ctx *gin.Context) {
	jobIDStr := ctx.Param("id")
	if jobIDStr == "" {
		response.Error(ctx, errors.New("job_id is required"))
		return
	}
	jobID, err := strconv.ParseUint(jobIDStr, 10, 32)
	if err != nil {
		response.Error(ctx, errors.New("invalid job_id"))
		return
	}
	req := services.SubmitJobRequest{JobID: uint(jobID)}

	result, err := c.k8sService.SubmitJob(req)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, result)
}

func (c *K8SController) GetJobStatus(ctx *gin.Context) {
	jobIDStr := ctx.Param("id")
	if jobIDStr == "" {
		response.Error(ctx, errors.New("job_id is required"))
		return
	}

	jobID, err := strconv.ParseUint(jobIDStr, 10, 32)
	if err != nil {
		response.Error(ctx, errors.New("invalid job_id"))
		return
	}

	result, err := c.k8sService.GetJobStatus(uint(jobID))
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, result)
}

func (c *K8SController) CancelJob(ctx *gin.Context) {
	jobIDStr := ctx.Param("id")
	if jobIDStr == "" {
		response.Error(ctx, errors.New("job_id is required"))
		return
	}

	jobID, err := strconv.ParseUint(jobIDStr, 10, 32)
	if err != nil {
		response.Error(ctx, errors.New("invalid job_id"))
		return
	}

	result, err := c.k8sService.CancelJob(uint(jobID))
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, result)
}

func (c *K8SController) GetGPUResources(ctx *gin.Context) {
	resources, err := c.k8sService.GetGPUResources()
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, resources)
}

func (c *K8SController) GetClusterStatus(ctx *gin.Context) {
	clusterIDStr := ctx.Param("id")
	if clusterIDStr == "" {
		response.Error(ctx, errors.New("cluster_id is required"))
		return
	}

	clusterID, err := strconv.ParseUint(clusterIDStr, 10, 32)
	if err != nil {
		response.Error(ctx, errors.New("invalid cluster_id"))
		return
	}

	status, err := c.k8sService.GetClusterStatus(uint(clusterID))
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, status)
}
