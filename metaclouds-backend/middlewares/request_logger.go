package middlewares

import (
	"bytes"
	"io"
	"regexp"
	"time"

	"metaclouds-backend/config"
	"metaclouds-backend/pkg/logger"

	"github.com/gin-gonic/gin"
)

type bodyLogWriter struct {
	gin.ResponseWriter
	body *bytes.Buffer
}

func (w bodyLogWriter) Write(b []byte) (int, error) {
	w.body.Write(b)
	return w.ResponseWriter.Write(b)
}

func RequestLogger() gin.HandlerFunc {
	return NewRequestLogger(nil)
}

func NewRequestLogger(cfg *config.Config) gin.HandlerFunc {
	var slowThreshold time.Duration = 2 * time.Second
	if cfg != nil && cfg.SlowRequestThresholdMs > 0 {
		slowThreshold = time.Duration(cfg.SlowRequestThresholdMs) * time.Millisecond
	}

	return func(c *gin.Context) {
		startTime := time.Now()

		var requestBody []byte
		contentLength := c.Request.ContentLength
		if contentLength > 0 && contentLength < 1024*1024 {
			requestBody, _ = io.ReadAll(c.Request.Body)
			c.Request.Body = io.NopCloser(bytes.NewBuffer(requestBody))
		}

		blw := &bodyLogWriter{body: bytes.NewBufferString(""), ResponseWriter: c.Writer}
		c.Writer = blw

		c.Next()

		duration := time.Since(startTime)
		statusCode := c.Writer.Status()

		logFields := []interface{}{
			"method", c.Request.Method,
			"path", c.Request.URL.Path,
			"status", statusCode,
			"duration", duration.String(),
			"client_ip", c.ClientIP(),
		}

		if contentLength > 0 && contentLength < 1024*1024 {
			logFields = append(logFields, "request_body", sanitizeRequestBody(string(requestBody)))
		}

		responseBody := blw.body.String()
		if len(responseBody) < 1024*10 {
			logFields = append(logFields, "response_body", sanitizeResponseBody(responseBody))
		}

		if duration > slowThreshold {
			logger.WarnWithCtx(c.Request.Context(), "Slow request detected", logFields...)
		} else {
			logger.InfoWithCtx(c.Request.Context(), "Request completed", logFields...)
		}
	}
}

var passwordRegex = regexp.MustCompile(`("password"\s*:\s*")([^"]+)(")`)

func sanitizeRequestBody(body string) string {
	return passwordRegex.ReplaceAllString(body, `$1[REDACTED]$3`)
}

func sanitizeResponseBody(body string) string {
	return passwordRegex.ReplaceAllString(body, `$1[REDACTED]$3`)
}
