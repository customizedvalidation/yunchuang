package middlewares

import (
	"time"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/logger"
)

func ErrorHandler() gin.HandlerFunc {
	return func(c *gin.Context) {
		c.Next()

		if len(c.Errors) > 0 {
			err := c.Errors[0].Err

			var statusCode int
			var message string
			var details interface{}

			if appErr, ok := errors.GetAppError(err); ok {
				statusCode = appErr.Code.HTTPStatus()
				message = appErr.Message
				details = appErr.Details
			} else {
				statusCode = 500
				message = "An unexpected error occurred"
			}

			logger.ErrorWithCtx(c.Request.Context(), "Request error", err, "status", statusCode)

			response := gin.H{
				"message":   message,
				"timestamp": time.Now().Unix(),
			}

			if details != nil {
				response["details"] = details
			}

			c.JSON(statusCode, response)
		}
	}
}

func HandleError(c *gin.Context, err error) {
	appErr := errors.FromError(err)

	logger.ErrorWithCtx(c.Request.Context(), "API error", err)

	c.JSON(appErr.HTTPStatus(), gin.H{
		"code":      appErr.HTTPStatus(),
		"error":     appErr.Code.String(),
		"message":   appErr.Message,
		"details":   appErr.Details,
		"timestamp": time.Now().Unix(),
	})
}
