package controllers

import (
	"time"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/logger"
	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

type AuthController struct {
	authService *services.AuthService
}

func NewAuthController(authService *services.AuthService) *AuthController {
	return &AuthController{
		authService: authService,
	}
}

func (c *AuthController) Login(ctx *gin.Context) {
	start := time.Now()
	var req services.LoginRequest

	logger.DebugWithCtx(ctx, "Login request received, starting authentication")

	if err := ctx.ShouldBindJSON(&req); err != nil {
		duration := time.Since(start)
		logger.WarnWithCtx(ctx, "Login failed - request binding error", "duration", duration, "error", err)
		response.Error(ctx, err)
		return
	}

	logger.DebugWithCtx(ctx, "Login attempt for user", "username", req.Username)

	resp, err := c.authService.Login(req)
	duration := time.Since(start)

	if err != nil {
		logger.WarnWithCtx(ctx, "Login failed - authentication error", "user", req.Username, "duration", duration, "error", err)
		response.Error(ctx, err)
		return
	}

	logger.InfoWithCtx(ctx, "Login successful", "user", req.Username, "duration", duration)
	response.Success(ctx, resp)
}

func (c *AuthController) Register(ctx *gin.Context) {
	var req services.RegisterRequest
	if err := ctx.ShouldBindJSON(&req); err != nil {
		response.Error(ctx, err)
		return
	}

	user, err := c.authService.Register(req)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Created(ctx, user)
}

// userIDFromContext 取出 JWT 中间件写入的用户 ID。
//
// JWTAuth 校验通过后必然写入 uint 类型的 user_id，此处仍保留类型断言而非直接强转，
// 避免中间件行为变化时把 panic 变成 500。
func userIDFromContext(ctx *gin.Context) (uint, bool) {
	value, exists := ctx.Get("user_id")
	if !exists {
		return 0, false
	}
	userID, ok := value.(uint)
	if !ok || userID == 0 {
		return 0, false
	}
	return userID, true
}

func (c *AuthController) Refresh(ctx *gin.Context) {
	userID, ok := userIDFromContext(ctx)
	if !ok {
		response.Error(ctx, errors.Unauthorized("user not authenticated"))
		return
	}

	resp, err := c.authService.Refresh(userID)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, resp)
}

// GetProfile 返回当前登录用户的资料。
//
// 响应体为 models.UserResponse，与 login/refresh 的 user 字段完全一致
// （此前是扁平 gin.H{user_id,...}，字段名与字段集都对不上）；
// 数据取自 DB 而非 JWT claims，故角色/租户变更后立即生效。
func (c *AuthController) GetProfile(ctx *gin.Context) {
	userID, ok := userIDFromContext(ctx)
	if !ok {
		response.Error(ctx, errors.Unauthorized("user not authenticated"))
		return
	}

	resp, err := c.authService.GetProfile(userID)
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, resp)
}
