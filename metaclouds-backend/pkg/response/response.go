package response

import (
	"net/http"
	"time"

	"metaclouds-backend/pkg/errors"

	"github.com/gin-gonic/gin"
)

type Response struct {
	Success   bool        `json:"success"`
	Data      interface{} `json:"data,omitempty"`
	Message   string      `json:"message,omitempty"`
	Code      string      `json:"code,omitempty"`
	Timestamp int64       `json:"timestamp"`
}

type PaginatedResponse struct {
	Data       interface{} `json:"data"`
	Total      int64       `json:"total"`
	Page       int         `json:"page"`
	PageSize   int         `json:"page_size"`
	TotalPages int         `json:"total_pages"`
}

func Success(c *gin.Context, data interface{}) {
	c.JSON(http.StatusOK, Response{
		Success:   true,
		Data:      data,
		Timestamp: getTimestamp(),
	})
}

func SuccessWithMessage(c *gin.Context, data interface{}, message string) {
	c.JSON(http.StatusOK, Response{
		Success:   true,
		Data:      data,
		Message:   message,
		Timestamp: getTimestamp(),
	})
}

func Created(c *gin.Context, data interface{}) {
	c.JSON(http.StatusCreated, Response{
		Success:   true,
		Data:      data,
		Timestamp: getTimestamp(),
	})
}

func NoContent(c *gin.Context) {
	c.Status(http.StatusNoContent)
}

func Error(c *gin.Context, err error) {
	appErr := errors.FromError(err)
	c.JSON(appErr.HTTPStatus(), Response{
		Success:   false,
		Message:   appErr.Message,
		Code:      appErr.Code.String(),
		Timestamp: getTimestamp(),
	})
}

func Paginated(c *gin.Context, data interface{}, total int64, page, pageSize int) {
	totalPages := int(total) / pageSize
	if int(total)%pageSize != 0 {
		totalPages++
	}
	c.JSON(http.StatusOK, Response{
		Success: true,
		Data: PaginatedResponse{
			Data:       data,
			Total:      total,
			Page:       page,
			PageSize:   pageSize,
			TotalPages: totalPages,
		},
		Timestamp: getTimestamp(),
	})
}

func getTimestamp() int64 {
	return time.Now().Unix()
}
