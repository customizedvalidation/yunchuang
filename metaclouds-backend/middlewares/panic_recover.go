package middlewares

import (
	"fmt"
	"net/http"
	"runtime/debug"
	"time"

	"metaclouds-backend/pkg/logger"
	"metaclouds-backend/pkg/response"

	"github.com/gin-gonic/gin"
)

func PanicRecovery() gin.HandlerFunc {
	return func(c *gin.Context) {
		start := time.Now()
		clientIP := c.ClientIP()
		path := c.Request.URL.Path
		method := c.Request.Method
		requestID := c.GetHeader("X-Request-ID")

		defer func() {
			if r := recover(); r != nil {
				logger.ErrorWithCtx(c.Request.Context(),
					"Panic recovered - critical error occurred",
					fmt.Errorf("%v", r),
					"stack_trace", string(debug.Stack()),
					"client_ip", clientIP,
					"path", path,
					"method", method,
					"request_id", requestID,
					"duration_until_panic", time.Since(start))

				c.Header("Connection", "close")
				response.Error(c, &panicError{msg: "Internal server error"})
			}
		}()

		logger.DebugWithCtx(c, "Panic recovery middleware active",
			"client_ip", clientIP,
			"path", path,
			"method", method,
			"request_id", requestID)

		c.Next()
	}
}

type panicError struct {
	msg string
}

func (e *panicError) Error() string {
	return e.msg
}

func (e *panicError) StatusCode() int {
	return http.StatusInternalServerError
}
