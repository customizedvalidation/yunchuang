package middlewares

import (
	"strconv"
	"time"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/logger"
)

func TimingMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		start := time.Now()

		c.Next()

		duration := time.Since(start)

		method := c.Request.Method
		path := c.Request.URL.Path
		statusCode := c.Writer.Status()
		clientIP := c.ClientIP()

		if statusCode >= 500 {
			logger.InfoWithCtx(c, "Request failed",
				"method", method,
				"path", path,
				"status", statusCode,
				"duration", duration,
				"client_ip", clientIP,
			)
		} else if statusCode >= 400 {
			logger.WarnWithCtx(c, "Request error",
				"method", method,
				"path", path,
				"status", statusCode,
				"duration", duration,
				"client_ip", clientIP,
			)
		} else {
			logger.InfoWithCtx(c, "Request completed",
				"method", method,
				"path", path,
				"status", statusCode,
				"duration", duration,
				"client_ip", clientIP,
			)
		}
	}
}

func TimingMiddlewareWithThreshold(threshold time.Duration) gin.HandlerFunc {
	return func(c *gin.Context) {
		start := time.Now()

		c.Next()

		duration := time.Since(start)

		method := c.Request.Method
		path := c.Request.URL.Path
		statusCode := c.Writer.Status()
		clientIP := c.ClientIP()

		overThreshold := duration > threshold

		if statusCode >= 500 {
			logger.InfoWithCtx(c, "[TIMING] Request failed",
				"method", method,
				"path", path,
				"status", statusCode,
				"duration", duration,
				"client_ip", clientIP,
				"over_threshold", overThreshold,
			)
		} else if statusCode >= 400 {
			logger.WarnWithCtx(c, "[TIMING] Request error",
				"method", method,
				"path", path,
				"status", statusCode,
				"duration", duration,
				"client_ip", clientIP,
				"over_threshold", overThreshold,
			)
		} else if overThreshold {
			logger.WarnWithCtx(c, "[TIMING] Request slow",
				"method", method,
				"path", path,
				"status", statusCode,
				"duration", duration,
				"client_ip", clientIP,
				"threshold", threshold,
			)
		} else {
			logger.InfoWithCtx(c, "[TIMING] Request completed",
				"method", method,
				"path", path,
				"status", statusCode,
				"duration", duration,
				"client_ip", clientIP,
			)
		}
	}
}

type TimingConfig struct {
	SlowThreshold time.Duration
	LogBody       bool
	LogResponse   bool
	ExcludePaths  []string
}

func TimingMiddlewareWithConfig(config TimingConfig) gin.HandlerFunc {
	return func(c *gin.Context) {
		path := c.Request.URL.Path

		for _, excludePath := range config.ExcludePaths {
			if excludePath == path {
				c.Next()
				return
			}
		}

		start := time.Now()

		c.Next()

		duration := time.Since(start)

		method := c.Request.Method
		statusCode := c.Writer.Status()
		clientIP := c.ClientIP()

		fields := []interface{}{
			"method", method,
			"path", path,
			"status", strconv.Itoa(statusCode),
			"duration", duration.String(),
			"client_ip", clientIP,
		}

		overThreshold := config.SlowThreshold > 0 && duration > config.SlowThreshold
		if overThreshold {
			fields = append(fields, "threshold", config.SlowThreshold.String())
		}

		if statusCode >= 500 {
			logger.InfoWithCtx(c, "Request failed", fields...)
		} else if statusCode >= 400 {
			logger.WarnWithCtx(c, "Request error", fields...)
		} else if overThreshold {
			logger.WarnWithCtx(c, "Request slow", fields...)
		} else {
			logger.InfoWithCtx(c, "Request completed", fields...)
		}
	}
}

var DefaultTimingMiddleware = TimingMiddlewareWithThreshold(500 * time.Millisecond)
