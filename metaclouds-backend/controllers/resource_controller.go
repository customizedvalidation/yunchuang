package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type ResourceController struct {
	resourceService *services.ResourceService
}

func NewResourceController(resourceService *services.ResourceService) *ResourceController {
	return &ResourceController{
		resourceService: resourceService,
	}
}

func (c *ResourceController) GetResources(ctx *gin.Context) {
	resources, err := c.resourceService.GetResources()
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, resources)
}

func (c *ResourceController) GetResource(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	resource, err := c.resourceService.GetResource(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, resource)
}

func (c *ResourceController) UpdateResource(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	var req services.UpdateResourceRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}

	resource, err := c.resourceService.UpdateResource(uint(id), req)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, resource)
}