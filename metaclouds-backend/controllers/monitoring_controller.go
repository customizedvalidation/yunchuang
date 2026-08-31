package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type MonitoringController struct {
	monitoringService *services.MonitoringService
}

func NewMonitoringController(monitoringService *services.MonitoringService) *MonitoringController {
	return &MonitoringController{
		monitoringService: monitoringService,
	}
}

func (c *MonitoringController) GetMetrics(ctx *gin.Context) {
	metrics, err := c.monitoringService.GetMetrics()
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, metrics)
}

func (c *MonitoringController) GetAlerts(ctx *gin.Context) {
	alerts, err := c.monitoringService.GetAlerts()
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, alerts)
}

func (c *MonitoringController) ResolveAlert(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	alert, err := c.monitoringService.ResolveAlert(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, alert)
}