package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type SecurityController struct {
	securityService *services.SecurityService
}

func NewSecurityController(securityService *services.SecurityService) *SecurityController {
	return &SecurityController{
		securityService: securityService,
	}
}

func (c *SecurityController) GetSecurityPolicies(ctx *gin.Context) {
	policies, err := c.securityService.GetSecurityPolicies()
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, policies)
}

func (c *SecurityController) GetSecurityPolicy(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	policy, err := c.securityService.GetSecurityPolicy(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, policy)
}

func (c *SecurityController) CreateSecurityPolicy(ctx *gin.Context) {
	var req services.CreateSecurityPolicyRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}

	policy, err := c.securityService.CreateSecurityPolicy(req)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Created(ctx, policy)
}

func (c *SecurityController) UpdateSecurityPolicy(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	var req services.UpdateSecurityPolicyRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}

	policy, err := c.securityService.UpdateSecurityPolicy(uint(id), req)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, policy)
}

func (c *SecurityController) DeleteSecurityPolicy(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	if err := c.securityService.DeleteSecurityPolicy(uint(id)); err != nil {
		response.Error(ctx, err)
		return
	}

	response.NoContent(ctx)
}