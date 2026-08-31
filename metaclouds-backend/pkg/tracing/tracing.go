package tracing

import (
	"context"
	"fmt"
	"net/http"
	"sync/atomic"

	"github.com/gin-gonic/gin"
)

type Span interface {
	End()
	RecordError(err error)
	SetAttributes(keyValues ...interface{})
}

type noopSpan struct{}

func (n noopSpan) End()                    {}
func (n noopSpan) RecordError(err error)   {}
func (n noopSpan) SetAttributes(...interface{}) {}

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
	traceIDKey traceContextKey = "trace_id"
	spanIDKey  traceContextKey = "span_id"
)

func StartSpan(ctx context.Context, name string, opts ...interface{}) (context.Context, Span) {
	traceID := GetTraceID(ctx)
	if traceID == "" {
		traceID = generateTraceID()
	}
	spanID := generateSpanID()

	newCtx := context.WithValue(ctx, traceIDKey, traceID)
	newCtx = context.WithValue(newCtx, spanIDKey, spanID)

	return newCtx, noopSpan{}
}

func GetTraceID(ctx context.Context) string {
	if ctx == nil {
		return ""
	}
	traceID, _ := ctx.Value(traceIDKey).(string)
	return traceID
}

func GetSpanID(ctx context.Context) string {
	if ctx == nil {
		return ""
	}
	spanID, _ := ctx.Value(spanIDKey).(string)
	return spanID
}

func GinMiddleware(serviceName string) gin.HandlerFunc {
	return func(c *gin.Context) {
		traceID := c.GetHeader("X-Trace-ID")
		if traceID == "" {
			traceID = generateTraceID()
		}
		spanID := generateSpanID()

		ctx := context.WithValue(c.Request.Context(), traceIDKey, traceID)
		ctx = context.WithValue(ctx, spanIDKey, spanID)

		c.Request = c.Request.WithContext(ctx)
		c.Set("trace_id", traceID)

		c.Next()
	}
}

func RecordError(ctx *gin.Context, err error) {
}

type TraceMiddleware struct{}

func (t *TraceMiddleware) Handle(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		traceID := r.Header.Get("X-Trace-ID")
		if traceID == "" {
			traceID = generateTraceID()
		}

		ctx := context.WithValue(r.Context(), traceIDKey, traceID)
		r = r.WithContext(ctx)

		w.Header().Set("X-Trace-ID", traceID)
		next.ServeHTTP(w, r)
	})
}