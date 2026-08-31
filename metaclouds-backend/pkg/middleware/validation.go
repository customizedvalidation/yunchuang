package middleware

import (
	"fmt"

	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/response"

	"github.com/gin-gonic/gin"
	"github.com/go-playground/validator/v10"
)

var validate = validator.New()

func ValidationMiddleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		c.Next()
	}
}

func ValidateStruct(s interface{}) error {
	err := validate.Struct(s)
	if err != nil {
		var validationErrors []string
		for _, e := range err.(validator.ValidationErrors) {
			field := e.Field()
			tag := e.Tag()
			param := e.Param()
			validationErrors = append(validationErrors, fmt.Sprintf("%s %s %s", field, tag, param))
		}
		return errors.NewWithDetails(errors.ErrValidation, "Validation failed", validationErrors)
	}
	return nil
}

func BindAndValidate(c *gin.Context, obj interface{}) error {
	if err := c.ShouldBindJSON(obj); err != nil {
		return errors.BadRequest("Invalid request body")
	}
	if err := ValidateStruct(obj); err != nil {
		return err
	}
	return nil
}

func MustBindAndValidate(c *gin.Context, obj interface{}) {
	if err := BindAndValidate(c, obj); err != nil {
		response.Error(c, err)
		c.Abort()
	}
}
