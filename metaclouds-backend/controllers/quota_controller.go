package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/models"
	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type QuotaController struct {
	quotaService QuotaServiceInterface
}

func NewQuotaController(quotaService QuotaServiceInterface) *QuotaController {
	return &QuotaController{quotaService: quotaService}
}

func (c *QuotaController) GetQuotas(ctx *gin.Context) {
	scopeType := ctx.Query("scope_type")
	scopeID, _ := strconv.ParseUint(ctx.Query("scope_id"), 10, 32)

	quotas, err := c.quotaService.GetQuotas(scopeType, uint(scopeID))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, quotas)
}

func (c *QuotaController) GetQuota(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	quota, err := c.quotaService.GetQuota(uint(id))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, quota)
}

func (c *QuotaController) CreateQuota(ctx *gin.Context) {
	var quota models.ResourceQuota
	if err := ctx.ShouldBindJSON(&quota); err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.quotaService.CreateQuota(&quota); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Created(ctx, quota)
}

func (c *QuotaController) UpdateQuota(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	var quota models.ResourceQuota
	if err := ctx.ShouldBindJSON(&quota); err != nil {
		response.Error(ctx, err)
		return
	}
	quota.ID = uint(id)
	if err := c.quotaService.UpdateQuota(&quota); err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, quota)
}

func (c *QuotaController) DeleteQuota(ctx *gin.Context) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	if err := c.quotaService.DeleteQuota(uint(id)); err != nil {
		response.Error(ctx, err)
		return
	}
	response.NoContent(ctx)
}

func (c *QuotaController) GetQuotaUsage(ctx *gin.Context) {
	scopeType := ctx.Query("scope_type")
	scopeID, _ := strconv.ParseUint(ctx.Query("scope_id"), 10, 32)

	summary, err := c.quotaService.GetQuotaUsageSummary(scopeType, uint(scopeID))
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, summary)
}

type CheckQuotaRequest struct {
	ScopeType    string  `json:"scope_type" binding:"required"`
	ScopeID      uint    `json:"scope_id" binding:"required"`
	ResourceType string  `json:"resource_type" binding:"required"`
	Requested    int     `json:"requested"`
	GPUFraction  float64 `json:"gpu_fraction"`
}

func (c *QuotaController) CheckQuota(ctx *gin.Context) {
	var req CheckQuotaRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}
	allowed, err := c.quotaService.CheckQuota(req.ScopeType, req.ScopeID, req.ResourceType, req.Requested, req.GPUFraction)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, gin.H{"allowed": allowed})
}

// QuotaServiceInterface 定义配额服务接口
type QuotaServiceInterface interface {
	GetQuotas(scopeType string, scopeID uint) ([]models.ResourceQuota, error)
	GetQuota(id uint) (*models.ResourceQuota, error)
	CreateQuota(quota *models.ResourceQuota) error
	UpdateQuota(quota *models.ResourceQuota) error
	DeleteQuota(id uint) error
	CheckQuota(scopeType string, scopeID uint, resourceType string, requested int, gpuFraction float64) (bool, error)
	ConsumeQuota(scopeType string, scopeID uint, resourceType string, amount int, gpuFraction float64) error
	ReleaseQuota(scopeType string, scopeID uint, resourceType string, amount int, gpuFraction float64) error
	GetQuotaUsageSummary(scopeType string, scopeID uint) (map[string]interface{}, error)
}

var _ QuotaServiceInterface = (*services.QuotaService)(nil)
