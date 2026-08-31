package response

import (
	"math"
	"net/http"
	"strconv"
	"time"

	"github.com/gin-gonic/gin"
)

type Pagination struct {
	Page      int         `json:"page"`
	PageSize  int         `json:"page_size"`
	Total     int64      `json:"total"`
	TotalPage int         `json:"total_page"`
	Items     interface{} `json:"items"`
}

func Paginate(c *gin.Context, items interface{}, total int64) Pagination {
	page, _ := strconv.Atoi(c.DefaultQuery("page", "1"))
	pageSize, _ := strconv.Atoi(c.DefaultQuery("page_size", "10"))

	if page < 1 {
		page = 1
	}
	if pageSize < 1 {
		pageSize = 10
	}
	if pageSize > 100 {
		pageSize = 100
	}

	totalPage := int(math.Ceil(float64(total) / float64(pageSize)))

	return Pagination{
		Page:      page,
		PageSize:  pageSize,
		Total:     total,
		TotalPage: totalPage,
		Items:     items,
	}
}

func PaginatedSuccess(c *gin.Context, items interface{}, total int64) {
	pagination := Paginate(c, items, total)
	c.JSON(http.StatusOK, gin.H{
		"code":    0,
		"message": "success",
		"data":    pagination,
		"timestamp": time.Now().Unix(),
	})
}

type PageRequest struct {
	Page     int `json:"page" form:"page"`
	PageSize int `json:"page_size" form:"page_size"`
}

func GetPageRequest(c *gin.Context) PageRequest {
	page, _ := strconv.Atoi(c.DefaultQuery("page", "1"))
	pageSize, _ := strconv.Atoi(c.DefaultQuery("page_size", "10"))

	if page < 1 {
		page = 1
	}
	if pageSize < 1 {
		pageSize = 10
	}
	if pageSize > 100 {
		pageSize = 100
	}

	return PageRequest{
		Page:     page,
		PageSize: pageSize,
	}
}