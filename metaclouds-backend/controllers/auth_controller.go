package controllers

import (
	"errors"
	"time"

	"github.com/gin-gonic/gin"

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

func (c *AuthController) Refresh(ctx *gin.Context) {
	userID, exists := ctx.Get("user_id")
	if !exists {
		response.Error(ctx, errors.New("user not authenticated"))
		return
	}

	resp, err := c.authService.Refresh(userID.(uint))
	if err != nil {
		response.Error(ctx, err)
		return
	}

	response.Success(ctx, resp)
}

func (c *AuthController) GetProfile(ctx *gin.Context) {
	userID, exists := ctx.Get("user_id")
	if !exists {
		response.Error(ctx, errors.New("user not authenticated"))
		return
	}

	username, _ := ctx.Get("username")
	email, _ := ctx.Get("email")
	role, _ := ctx.Get("role")
	tenantID, _ := ctx.Get("tenant_id")

	response.Success(ctx, gin.H{
		"user_id":   userID,
		"username":  username,
		"email":     email,
		"role":      role,
		"tenant_id": tenantID,
	})
}
