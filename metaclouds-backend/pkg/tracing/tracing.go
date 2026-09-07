package tracing

import (
	"context"
	"fmt"
	"net/http"
	"sync/atomic"
	"time"

	"github.com/gin-gonic/gin"
)

// Span 定义追踪跨度接口。
// 当前实现为轻量级本地 span（不依赖 OpenTelemetry SDK），
// 记录 trace_id/span_id 及属性，供日志关联和后续接入 Jaeger 使用。
type Span interface {
	End()
	RecordError(err error)
	SetAttributes(keyValues ...interface{})
	TraceID() string
	SpanID() string
}

// span 是 Span 的本地实现，记录开始时间和属性。
type span struct {
	traceID    string
	spanID     string
	name       string
	startTime  time.Time
	attributes map[string]interface{}
	err        error
}

func (s *span) End() {
	// 当前为本地实现，End 仅标记结束；接入 OTel 后在此处导出 span。
}

func (s *span) RecordError(err error) {
	s.err = err
}

func (s *span) SetAttributes(keyValues ...interface{}) {
	for i := 0; i+1 < len(keyValues); i += 2 {
		if key, ok := keyValues[i].(string); ok {
			s.attributes[key] = keyValues[i+1]
		}
	}
}

func (s *span) TraceID() string { return s.traceID }
func (s *span) SpanID() string  { return s.spanID }

// noopSpan 用于未启用追踪时的空实现。
type noopSpan struct{}

func (n noopSpan) End()                       {}
func (n noopSpan) RecordError(err error)      {}
func (n noopSpan) SetAttributes(...interface{}) {}
func (n noopSpan) TraceID() string            { return "" }
func (n noopSpan) SpanID() string             { return "" }

var (
	traceIDCounter uint64
	spanIDCounter  uint64
)

func generateTraceID() string {
	return fmt.Sprintf("%016x", atomic.AddUint64(&traceIDCounter, 1))
}

func generateSpanID() string {
	return fmt.Sprintf("%016x", atomic.AddUint64(&spanIDCounter, 1))
}

type traceContextKey string

const (
	// TraceIDKey 是 context 中 trace_id 的键。
	TraceIDKey traceContextKey = "trace_id"
	// SpanIDKey 是 context 中 span_id 的键。
	SpanIDKey traceContextKey = "span_id"
	// SpanKey 是 context 中当前 Span 的键。
	SpanKey traceContextKey = "span"
)

// StartSpan 在给定 context 中启动一个新 span。
// 如果 context 中已有 trace_id，则复用；否则生成新的 trace_id。
func StartSpan(ctx context.Context, name string, opts ...interface{}) (context.Context, Span) {
	traceID := GetTraceID(ctx)
	if traceID == "" {
		traceID = generateTraceID()
	}
	spanID := generateSpanID()

	s := &span{
		traceID:    traceID,
		spanID:     spanID,
		name:       name,
		startTime:  time.Now(),
		attributes: make(map[string]interface{}),
	}

	newCtx := context.WithValue(ctx, TraceIDKey, traceID)
	newCtx = context.WithValue(newCtx, SpanIDKey, spanID)
	newCtx = context.WithValue(newCtx, SpanKey, s)

	return newCtx, s
}

// GetTraceID 从 context 中提取 trace_id。
func GetTraceID(ctx context.Context) string {
	if ctx == nil {
		return ""
	}
	traceID, _ := ctx.Value(TraceIDKey).(string)
	return traceID
}

// GetSpanID 从 context 中提取 span_id。
func GetSpanID(ctx context.Context) string {
	if ctx == nil {
		return ""
	}
	spanID, _ := ctx.Value(SpanIDKey).(string)
	return spanID
}

// GetSpan 从 context 中提取当前 Span。
func GetSpan(ctx context.Context) Span {
	if ctx == nil {
		return noopSpan{}
	}
	if s, ok := ctx.Value(SpanKey).(Span); ok {
		return s
	}
	return noopSpan{}
}

// GinMiddleware 是 Gin 追踪中间件。
// 为每个 HTTP 请求创建一个 span，注入 trace_id/span_id 到 context，
// 并在响应头中返回 X-Trace-ID，便于客户端关联日志。
func GinMiddleware(serviceName string) gin.HandlerFunc {
	return func(c *gin.Context) {
		// 优先从请求头中提取上游传入的 trace_id，实现跨服务追踪关联。
		traceID := c.GetHeader("X-Trace-ID")
		if traceID == "" {
			traceID = generateTraceID()
		}
		spanID := generateSpanID()

		start := time.Now()

		// 创建请求级 span
		s := &span{
			traceID:    traceID,
			spanID:     spanID,
			name:       fmt.Sprintf("%s %s", c.Request.Method, c.Request.URL.Path),
			startTime:  start,
			attributes: make(map[string]interface{}),
		}
		s.SetAttributes(
			"http.method", c.Request.Method,
			"http.path", c.Request.URL.Path,
			"http.host", c.Request.Host,
			"service.name", serviceName,
		)

		// 将 trace_id、span_id 和 span 注入请求 context
		ctx := context.WithValue(c.Request.Context(), TraceIDKey, traceID)
		ctx = context.WithValue(ctx, SpanIDKey, spanID)
		ctx = context.WithValue(ctx, SpanKey, s)
		c.Request = c.Request.WithContext(ctx)

		// 在 Gin context 中也设置 trace_id，供 logger 等中间件读取
		c.Set("trace_id", traceID)
		c.Set("span_id", spanID)

		// 响应头返回 trace_id，便于客户端日志关联
		c.Header("X-Trace-ID", traceID)

		c.Next()

		// 请求结束后补充状态码和耗时属性
		duration := time.Since(start)
		s.SetAttributes(
			"http.status_code", c.Writer.Status(),
			"http.duration_ms", duration.Milliseconds(),
			"http.client_ip", c.ClientIP(),
		)
		if c.Writer.Status() >= 500 {
			s.RecordError(fmt.Errorf("HTTP %d", c.Writer.Status()))
		}
		s.End()
	}
}

// RecordError 在当前 span 中记录错误。
func RecordError(ctx *gin.Context, err error) {
	if s := GetSpan(ctx.Request.Context()); s != nil {
		s.RecordError(err)
	}
}

// TraceMiddleware 是标准 net/http 中间件（供非 Gin 场景使用）。
type TraceMiddleware struct{}

func (t *TraceMiddleware) Handle(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		traceID := r.Header.Get("X-Trace-ID")
		if traceID == "" {
			traceID = generateTraceID()
		}

		ctx := context.WithValue(r.Context(), TraceIDKey, traceID)
		r = r.WithContext(ctx)

		w.Header().Set("X-Trace-ID", traceID)
		next.ServeHTTP(w, r)
	})
}
