package middlewares

import (
	"metaclouds-backend/config"
	"metaclouds-backend/pkg/logger"

	"github.com/gin-gonic/gin"
)

type SecurityHeadersConfig struct {
	EnableHSTS bool
	HSTSMaxAge int
}

func SecurityHeaders() gin.HandlerFunc {
	return NewSecurityHeaders(nil)
}

func NewSecurityHeaders(cfg *config.Config) gin.HandlerFunc {
	return func(c *gin.Context) {
		environment := "development"
		if cfg != nil {
			environment = cfg.Environment
		}

		logger.DebugWithCtx(c, "Security headers middleware processing request",
			"environment", environment,
			"path", c.Request.URL.Path,
			"method", c.Request.Method)

		c.Header("X-Content-Type-Options", "nosniff")
		c.Header("X-Frame-Options", "DENY")
		c.Header("X-XSS-Protection", "1; mode=block")
		c.Header("Referrer-Policy", "strict-origin-when-cross-origin")
		c.Header("Permissions-Policy", "geolocation=(), microphone=(), camera=()")

		if cfg != nil && cfg.Environment == "production" {
			c.Header("Strict-Transport-Security", "max-age=31536000; includeSubDomains")
			logger.DebugWithCtx(c, "HSTS header enabled for production environment")
		}

		if cfg != nil && cfg.Environment == "production" {
			c.Header("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self'")
			logger.DebugWithCtx(c, "Production CSP policy applied")
		} else {
			c.Header("Content-Security-Policy", "default-src 'self'; script-src 'self' 'unsafe-inline' https://cdnjs.cloudflare.com; style-src 'self' 'unsafe-inline' https://cdnjs.cloudflare.com; img-src 'self' data:; font-src 'self'; connect-src 'self'")
			logger.DebugWithCtx(c, "Development CSP policy applied")
		}

		c.Header("Server", "Metaclouds")

		c.Next()
	}
}
