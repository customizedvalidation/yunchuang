package controllers

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"net/http"
	"time"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/config"
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
	// 将 JWT 写入 httpOnly Cookie：前端 SPA 不再把令牌存于 JS 可读的 localStorage，
	// 从根源上消除 XSS 窃取令牌的攻击面。Bearer 头仍可被非浏览器客户端使用。
	c.setAuthCookie(ctx, resp.Token)
	// 同步写入双提交 CSRF 令牌（非 httpOnly，供同源 JS 读取并放入 X-CSRF-Token 头）。
	c.setCsrfCookie(ctx, generateCSRFToken())
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

// authCookieName 是存放 JWT 的 httpOnly Cookie 名。
const authCookieName = "access_token"

// setAuthCookie 将 JWT 写入 httpOnly Cookie，使前端 SPA 不再把令牌存于 JS 可读的
// localStorage（XSS 防护）。
//
//   - Secure：仅在生产环境开启。dev 走 http，浏览器不会保存 Secure Cookie，开启会导致
//     本地开发登录后拿不到令牌；生产环境必须 https，故开启。
//   - SameSite：由配置 CookieSameSite 决定（默认 lax）。Lax 即可抵御绝大多数 CSRF
//     （跨站 POST 不携带 Cookie）；跨完全不同域部署时改 none（必须配合 CSRF 双提交令牌，
//     且生产 https 下 SameSite=None 强制 Secure）。
//   - Domain 留空：Cookie 仅作用于当前 host，不向其他子域泄漏。
func (c *AuthController) setAuthCookie(ctx *gin.Context, token string) {
	cfg, err := config.LoadConfig()
	if err != nil {
		logger.ErrorWithCtx(ctx, "failed to load config for auth cookie", err)
		return
	}
	maxAge := cfg.JWTExpirationHours * 3600
	if maxAge <= 0 {
		maxAge = 86400
	}
	ctx.SetSameSite(cfg.CookieSameSiteMode())
	ctx.SetCookie(
		authCookieName,
		token,
		maxAge,
		"/",
		"",
		cfg.Environment == "production",
		true,
	)
}

// clearAuthCookie 使 access_token Cookie 立即失效（用于登出）。
func (c *AuthController) clearAuthCookie(ctx *gin.Context) {
	ctx.SetSameSite(cookieSameSiteOrDefault(ctx))
	ctx.SetCookie(authCookieName, "", -1, "/", "", false, true)
}

// csrfCookieName 是 CSRF 双提交令牌的 Cookie 名（非 httpOnly，供 JS 读取）。
const csrfCookieName = "csrf_token"

// generateCSRFToken 生成 32 字节密码学随机令牌（hex 编码）。
func generateCSRFToken() string {
	b := make([]byte, 32)
	if _, err := rand.Read(b); err != nil {
		// 极不可能失败；失败时退化为基于时间纳秒的兜底值，不影响认证链路。
		return fmt.Sprintf("%d", time.Now().UnixNano())
	}
	return hex.EncodeToString(b)
}

// setCsrfCookie 写入非 httpOnly 的 csrf_token Cookie，供同源 JS 读取并放入 X-CSRF-Token 头。
// 与 access_token 同 SameSite/Secure/Path，但 httpOnly=false（JS 需可读以完成双提交）。
func (c *AuthController) setCsrfCookie(ctx *gin.Context, token string) {
	cfg, err := config.LoadConfig()
	if err != nil {
		logger.ErrorWithCtx(ctx, "failed to load config for csrf cookie", err)
		return
	}
	maxAge := cfg.JWTExpirationHours * 3600
	if maxAge <= 0 {
		maxAge = 86400
	}
	ctx.SetSameSite(cfg.CookieSameSiteMode())
	ctx.SetCookie(csrfCookieName, token, maxAge, "/", "", cfg.Environment == "production", false)
}

// clearCsrfCookie 使 csrf_token Cookie 立即失效（用于登出）。
func (c *AuthController) clearCsrfCookie(ctx *gin.Context) {
	ctx.SetSameSite(cookieSameSiteOrDefault(ctx))
	ctx.SetCookie(csrfCookieName, "", -1, "/", "", false, false)
}

// cookieSameSiteOrDefault 读取配置的 SameSite；配置加载失败时回退 Lax，避免登出路径因
// 配置异常而把 Cookie 写成错误的 SameSite 导致客户端清不掉。
func cookieSameSiteOrDefault(ctx *gin.Context) http.SameSite {
	if cfg, err := config.LoadConfig(); err == nil {
		return cfg.CookieSameSiteMode()
	}
	logger.ErrorWithCtx(ctx, "failed to load config for cookie samesite, falling back to Lax", nil)
	return http.SameSiteLaxMode
}

// GetCSRFToken 返回当前会话的 CSRF 双提交令牌（从 csrf_token Cookie 读取）。
//
// 用于跨域部署下前端 JS 无法读取 Cookie 时，通过带凭据的 GET 获取令牌并放入 X-CSRF-Token 头。
// 未登录（无 csrf_token Cookie）时返回 401，由前端引导重新登录。
func (c *AuthController) GetCSRFToken(ctx *gin.Context) {
	token, err := ctx.Cookie(csrfCookieName)
	if err != nil || token == "" {
		response.Error(ctx, errors.Unauthorized("CSRF token not found, please login first"))
		return
	}
	response.Success(ctx, gin.H{"csrf_token": token})
}

// Logout 清除客户端的 access_token 与 csrf_token Cookie 并完成登出。
//
// 不要求已登录：清除一个不存在的 Cookie 是幂等的，前端在任意状态下调用都安全。
func (c *AuthController) Logout(ctx *gin.Context) {
	c.clearAuthCookie(ctx)
	c.clearCsrfCookie(ctx)
	response.Success(ctx, gin.H{"message": "logged out"})
}
