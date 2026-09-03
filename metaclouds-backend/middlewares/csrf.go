package middlewares

import (
	"crypto/subtle"
	"net/http"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/config"
	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/logger"
	"metaclouds-backend/pkg/response"
)

const (
	// csrfCookieName 是双提交令牌的 Cookie 名（与 controllers 中保持一致）。
	csrfCookieName = "csrf_token"
	// csrfHeaderName 是前端在请求头中回传的令牌名。
	csrfHeaderName = "X-CSRF-Token"
	// authCookieName 是存放 JWT 的 httpOnly Cookie 名（与 controllers 中保持一致）。
	authCookieName = "access_token"
)

// csrfProtectedMethods 是需要校验双提交令牌的状态变更方法。
var csrfProtectedMethods = map[string]bool{
	http.MethodPost:   true,
	http.MethodPut:    true,
	http.MethodDelete: true,
	http.MethodPatch:  true,
}

// NewCSRFProtect 返回 CSRF 双提交令牌中间件。
//
// 机制：登录/刷新时后端把同一随机值分别写入 csrf_token Cookie（非 httpOnly）与
// （经 GET /auth/csrf）响应体。同源 JS 可读 Cookie 并将该值放入 X-CSRF-Token 请求头；
// 中间件校验「头值 == Cookie 值」即可证明请求由同源 JS 发起，从而抵御 CSRF。
//
// 通道区分：
//   - 走 Bearer 头的非浏览器客户端（curl/SDK）无 csrf Cookie，跳过校验（它们不受 CSRF 影响）。
//   - 仅当请求携带 access_token Cookie（浏览器会话）时才强制校验。
//   - GET/HEAD/OPTIONS 等幂等读取方法不校验。
//
// 比较使用常量时间，避免计时侧信道泄露令牌差异。
func NewCSRFProtect(cfg *config.Config) gin.HandlerFunc {
	return func(c *gin.Context) {
		// 幂等读取方法不校验。
		if !csrfProtectedMethods[c.Request.Method] {
			c.Next()
			return
		}
		// 无 access_token Cookie：视为非浏览器（Bearer）客户端，跳过 CSRF 校验。
		if _, err := c.Cookie(authCookieName); err != nil {
			c.Next()
			return
		}

		headerToken := c.GetHeader(csrfHeaderName)
		cookieToken, cookieErr := c.Cookie(csrfCookieName)
		if headerToken == "" || cookieErr != nil || cookieToken == "" {
			logger.WarnWithCtx(c, "CSRF validation failed - token missing",
				"path", c.Request.URL.Path,
				"method", c.Request.Method,
				"has_header", headerToken != "",
				"has_cookie", cookieErr == nil)
			response.Error(c, errors.Forbidden("CSRF token missing or invalid"))
			c.Abort()
			return
		}

		// 常量时间比较，避免计时侧信道。
		if subtle.ConstantTimeCompare([]byte(headerToken), []byte(cookieToken)) != 1 {
			logger.WarnWithCtx(c, "CSRF validation failed - token mismatch",
				"path", c.Request.URL.Path,
				"method", c.Request.Method)
			response.Error(c, errors.Forbidden("CSRF token mismatch"))
			c.Abort()
			return
		}

		c.Next()
	}
}
