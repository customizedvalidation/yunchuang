package middlewares

import (
	"net/http"
	"time"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/config"
	"metaclouds-backend/pkg/logger"
	"metaclouds-backend/pkg/middleware"
)

// ApplyCoreStack 注册生产环境与测试环境共用的核心中间件。
//
// 此前测试走 api.SetupRouter、生产走 main.go 自建路由，两套中间件不一致：
// 测试路径缺少 MaxBytesReader、PanicRecovery、SecurityFilter、RequestLogger、
// GinMiddleware，导致测试覆盖无法反映生产行为。统一到本函数后，两者仅差 CORS
// 来源（由调用方各自配置）与可选的限流/熔断。
//
// 注意：请求体大小限制必须排在任何会读取 body 的中间件之前（SecurityFilter 会
// 完整读取 body，但其内部已用 io.NopCloser 还原，不会耗尽）。
func ApplyCoreStack(r *gin.Engine, cfg *config.Config) {
	// 请求体大小限制（最先注册：在任何读取 body 的中间件之前）。
	// MaxRequestBodySize <= 0 视为未配置，跳过以避免吞掉合法请求（如测试内联 cfg）。
	if cfg.MaxRequestBodySize > 0 {
		limit := cfg.MaxRequestBodySize
		r.Use(func(c *gin.Context) {
			c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, limit)
			c.Next()
		})
	}

	r.Use(PanicRecovery())
	r.Use(RequestID())
	r.Use(SecurityHeaders())
	r.Use(SecurityFilter())
	r.Use(RequestLogger())
	r.Use(ErrorHandler())
	r.Use(DefaultTimingMiddleware)
	r.Use(logger.GinMiddleware(logger.Global))

	if cfg.RateLimitEnabled {
		rateLimiter := middleware.NewSlidingWindowLimiter(cfg.RateLimitRequests, time.Duration(cfg.RateLimitDurationSeconds)*time.Second)
		r.Use(rateLimiter.Middleware())
	}

	if cfg.CircuitBreakerEnabled {
		circuitBreaker := middleware.NewCircuitBreaker(cfg.CircuitBreakerThreshold, 5, time.Duration(cfg.CircuitBreakerTimeoutSeconds)*time.Second)
		r.Use(circuitBreaker.Middleware())
	}
}
