package middlewares

import (
	"metaclouds-backend/pkg/response"

	"github.com/gin-gonic/gin"
	"github.com/go-playground/validator/v10"
)

var validate *validator.Validate

func init() {
	validate = validator.New()
}

func ValidateRequest(s interface{}) gin.HandlerFunc {
	return func(c *gin.Context) {
		if err := c.ShouldBindJSON(s); err != nil {
			response.Error(c, err)
			c.Abort()
			return
		}

		if err := validate.Struct(s); err != nil {
			response.Error(c, err)
			c.Abort()
			return
		}

		c.Set("validated_request", s)
		c.Next()
	}
}

func GetValidatedRequest(c *gin.Context) (interface{}, bool) {
	return c.Get("validated_request")
}
