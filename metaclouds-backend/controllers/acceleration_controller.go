package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type AccelerationController struct {
	accelerationService *services.AccelerationService
}

func NewAccelerationController(accelerationService *services.AccelerationService) *AccelerationController {
	return &AccelerationController{
		accelerationService: accelerationService,
	}
}

func (c *AccelerationController) GetAccelerationSuites(ctx *gin.Context) {
	suites, err := c.accelerationService.GetAccelerationSuites()
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, suites)
}

func (c *AccelerationController) GetAccelerationSuite(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	suite, err := c.accelerationService.GetAccelerationSuite(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, suite)
}

func (c *AccelerationController) CreateAccelerationSuite(ctx *gin.Context) {
	var req services.CreateAccelerationSuiteRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}

	suite, err := c.accelerationService.CreateAccelerationSuite(req)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Created(ctx, suite)
}

func (c *AccelerationController) UpdateAccelerationSuite(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	var req services.UpdateAccelerationSuiteRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}

	suite, err := c.accelerationService.UpdateAccelerationSuite(uint(id), req)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, suite)
}

func (c *AccelerationController) DeleteAccelerationSuite(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	if err := c.accelerationService.DeleteAccelerationSuite(uint(id)); err != nil {
		response.Error(ctx, err)
		return
	}

	response.NoContent(ctx)
}