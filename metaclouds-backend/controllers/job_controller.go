package controllers

import (
	"strconv"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type JobController struct {
	jobService *services.JobService
}

func NewJobController(jobService *services.JobService) *JobController {
	return &JobController{
		jobService: jobService,
	}
}

// actorFromContext 从 JWT 中间件注入的上下文中提取调用者身份。
// tenantID 为 0 表示平台级账号（如内置 admin），此时按管理员处理。
func actorFromContext(ctx *gin.Context) (tenantID, userID uint, isAdmin bool) {
	if v, exists := ctx.Get("tenant_id"); exists {
		if id, ok := v.(uint); ok {
			tenantID = id
		}
	}
	if v, exists := ctx.Get("user_id"); exists {
		if id, ok := v.(uint); ok {
			userID = id
		}
	}
	if v, exists := ctx.Get("role"); exists {
		if role, ok := v.(string); ok {
			isAdmin = role == "admin"
		}
	}
	return tenantID, userID, isAdmin
}

// parseID 解析路径参数中的 ID。解析失败时返回 400 而不是把底层错误直接抛给客户端。
func parseID(ctx *gin.Context) (uint, bool) {
	id, err := strconv.ParseUint(ctx.Param("id"), 10, 32)
	if err != nil {
		response.Error(ctx, errors.BadRequest("invalid id parameter"))
		return 0, false
	}
	return uint(id), true
}

func (c *JobController) GetJobs(ctx *gin.Context) {
	tenantID, _, isAdmin := actorFromContext(ctx)

	jobs, err := c.jobService.GetJobsVisibleTo(tenantID, isAdmin)
	if err != nil {
		response.Error(ctx, err)
		return
	}
	response.Success(ctx, jobs)
}

func (c *JobController) GetJob(ctx *gin.Context) {
	id, ok := parseID(ctx)
	if !ok {
		return
	}
	tenantID, _, isAdmin := actorFromContext(ctx)

	job, err := c.jobService.GetJobVisibleTo(id, tenantID, isAdmin)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, job)
}

func (c *JobController) CreateJob(ctx *gin.Context) {
	var req services.CreateJobRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, errors.BadRequest("invalid request body"))
		return
	}

	tenantID, userID, isAdmin := actorFromContext(ctx)

	// 租户与归属用户一律由服务端依据令牌判定，不采信请求体。
	job, err := c.jobService.CreateJobForUser(req, tenantID, userID, isAdmin)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Created(ctx, job)
}

func (c *JobController) UpdateJob(ctx *gin.Context) {
	id, ok := parseID(ctx)
	if !ok {
		return
	}

	var req services.UpdateJobRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, errors.BadRequest("invalid request body"))
		return
	}

	tenantID, _, isAdmin := actorFromContext(ctx)

	job, err := c.jobService.UpdateJobForTenant(id, tenantID, isAdmin, req)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, job)
}

func (c *JobController) DeleteJob(ctx *gin.Context) {
	id, ok := parseID(ctx)
	if !ok {
		return
	}
	tenantID, _, isAdmin := actorFromContext(ctx)

	if err := c.jobService.DeleteJobForTenant(id, tenantID, isAdmin); err != nil {
		response.Error(ctx, err)
		return
	}

	response.NoContent(ctx)
}

func (c *JobController) CancelJob(ctx *gin.Context) {
	id, ok := parseID(ctx)
	if !ok {
		return
	}
	tenantID, _, isAdmin := actorFromContext(ctx)

	job, err := c.jobService.CancelJobForTenant(id, tenantID, isAdmin)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, job)
}
