package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type TenantController struct {
	tenantService *services.TenantService
}

func NewTenantController(tenantService *services.TenantService) *TenantController {
	return &TenantController{
		tenantService: tenantService,
	}
}

func (c *TenantController) GetTenants(ctx *gin.Context) {
	tenants, err := c.tenantService.GetTenants()
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, tenants)
}

func (c *TenantController) GetTenant(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	tenant, err := c.tenantService.GetTenant(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, tenant)
}

func (c *TenantController) CreateTenant(ctx *gin.Context) {
	var req services.CreateTenantRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}

	tenant, err := c.tenantService.CreateTenant(req)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Created(ctx, tenant)
}

func (c *TenantController) UpdateTenant(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	var req services.UpdateTenantRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}

	tenant, err := c.tenantService.UpdateTenant(uint(id), req)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, tenant)
}

func (c *TenantController) DeleteTenant(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	if err := c.tenantService.DeleteTenant(uint(id)); err != nil {
		response.Error(ctx, err)
		return
	}

	response.NoContent(ctx)
}