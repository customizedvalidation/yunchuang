package middleware

import (
	"sync"
	"time"

	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/response"

	"github.com/gin-gonic/gin"
)

type CircuitState int

const (
	Closed CircuitState = iota
	Open
	HalfOpen
)

// breakerState 是单个路由的熔断状态。
// 原先熔断器是全局单实例：任意一个接口返回 5xx 就会把整站流量切到 503，
// 单个故障点即可造成全系统不可用。改为按路由维护独立状态后，故障被隔离在
// 出问题的接口上，其余接口不受影响。
type breakerState struct {
	state        CircuitState
	failureCount int
	successCount int
	lastFailure  time.Time
}

type CircuitBreaker struct {
	mu               sync.Mutex
	routes           map[string]*breakerState
	failureThreshold int
	successThreshold int
	timeout          time.Duration
}

func NewCircuitBreaker(failureThreshold, successThreshold int, timeout time.Duration) *CircuitBreaker {
	return &CircuitBreaker{
		routes:           make(map[string]*breakerState),
		failureThreshold: failureThreshold,
		successThreshold: successThreshold,
		timeout:          timeout,
	}
}

func (cb *CircuitBreaker) state(key string) *breakerState {
	s, ok := cb.routes[key]
	if !ok {
		s = &breakerState{state: Closed}
		cb.routes[key] = s
	}
	return s
}

func (cb *CircuitBreaker) Middleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		// 未匹配到注册路由（如静态资源、SPA 兜底）直接放行，不做熔断。
		key := c.FullPath()
		if key == "" {
			c.Next()
			return
		}

		cb.mu.Lock()
		s := cb.state(key)
		if s.state == Open {
			if time.Since(s.lastFailure) >= cb.timeout {
				s.state = HalfOpen
			} else {
				cb.mu.Unlock()
				response.Error(c, errors.NewServiceUnavailableError("Service temporarily unavailable"))
				c.Abort()
				return
			}
		}
		cb.mu.Unlock()

		// 关键路径必须在锁外执行，否则所有请求会被串行化，吞吐骤降。
		c.Next()

		cb.mu.Lock()
		defer cb.mu.Unlock()

		// 仅对 5xx 服务端错误计数；4xx 客户端错误（含 401/403/429）不算失败。
		if c.Writer.Status() >= 500 {
			cb.onFailure(s)
		} else {
			cb.onSuccess(s)
		}
	}
}

func (cb *CircuitBreaker) onFailure(s *breakerState) {
	switch s.state {
	case Closed:
		s.failureCount++
		if s.failureCount >= cb.failureThreshold {
			s.state = Open
			s.lastFailure = time.Now()
			s.failureCount = 0
		}
	case HalfOpen:
		s.state = Open
		s.lastFailure = time.Now()
		s.successCount = 0
	}
}

func (cb *CircuitBreaker) onSuccess(s *breakerState) {
	switch s.state {
	case Closed:
		s.failureCount = 0
	case HalfOpen:
		s.successCount++
		if s.successCount >= cb.successThreshold {
			s.state = Closed
			s.successCount = 0
		}
	}
}

// State 返回指定路由当前的熔断状态，供运维/测试查询。
func (cb *CircuitBreaker) State(route string) CircuitState {
	cb.mu.Lock()
	defer cb.mu.Unlock()
	if s, ok := cb.routes[route]; ok {
		return s.state
	}
	return Closed
}

// Reset 清除指定路由（或全部，route 为空）的熔断状态。
func (cb *CircuitBreaker) Reset(route string) {
	cb.mu.Lock()
	defer cb.mu.Unlock()
	if route == "" {
		cb.routes = make(map[string]*breakerState)
		return
	}
	delete(cb.routes, route)
}
