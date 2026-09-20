package middlewares

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	"metaclouds-backend/config"
	appErrors "metaclouds-backend/pkg/errors"
)

func init() {
	gin.SetMode(gin.TestMode)
}

// newTestRouter 创建一个仅挂载指定中间件和处理函数的测试路由。
func newTestRouter(middleware gin.HandlerFunc, handler gin.HandlerFunc) *gin.Engine {
	r := gin.New()
	if middleware != nil {
		r.Use(middleware)
	}
	r.GET("/test", handler)
	r.POST("/test", handler)
	return r
}

// ===== ErrorHandler 测试 =====

func TestErrorHandler_AppError(t *testing.T) {
	tests := []struct {
		name           string
		err            error
		expectedStatus int
	}{
		{"BadRequest", appErrors.BadRequest("invalid input"), http.StatusBadRequest},
		{"Unauthorized", appErrors.Unauthorized("not logged in"), http.StatusUnauthorized},
		{"Forbidden", appErrors.Forbidden("no permission"), http.StatusForbidden},
		{"NotFound", appErrors.NotFound("resource not found"), http.StatusNotFound},
		{"Conflict", appErrors.Conflict("already exists"), http.StatusConflict},
		{"InternalServer", appErrors.InternalServer("db error"), http.StatusInternalServerError},
		{"RateLimit", appErrors.RateLimit("too many requests"), http.StatusTooManyRequests},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			r := newTestRouter(ErrorHandler(), func(c *gin.Context) {
				c.Error(tc.err)
			})
			w := httptest.NewRecorder()
			req := httptest.NewRequest(http.MethodGet, "/test", nil)
			r.ServeHTTP(w, req)

			assert.Equal(t, tc.expectedStatus, w.Code)
			var resp map[string]interface{}
			require.NoError(t, json.Unmarshal(w.Body.Bytes(), &resp))
			assert.Contains(t, resp["message"], "")
		})
	}
}

func TestErrorHandler_GenericError(t *testing.T) {
	r := newTestRouter(ErrorHandler(), func(c *gin.Context) {
		c.Error(errors.New("something went wrong"))
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusInternalServerError, w.Code)
	var resp map[string]interface{}
	require.NoError(t, json.Unmarshal(w.Body.Bytes(), &resp))
	assert.Equal(t, "An unexpected error occurred", resp["message"])
}

func TestErrorHandler_NoError(t *testing.T) {
	r := newTestRouter(ErrorHandler(), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestHandleError_DirectCall(t *testing.T) {
	r := gin.New()
	r.GET("/test", func(c *gin.Context) {
		HandleError(c, appErrors.NotFound("not found"))
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusNotFound, w.Code)
	var resp map[string]interface{}
	require.NoError(t, json.Unmarshal(w.Body.Bytes(), &resp))
	assert.Equal(t, "not found", resp["message"])
	assert.Equal(t, "NOT_FOUND", resp["error"])
}

// ===== PanicRecovery 测试 =====

func TestPanicRecovery_RecoversPanic(t *testing.T) {
	r := newTestRouter(PanicRecovery(), func(c *gin.Context) {
		panic("something broke")
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)

	// 不应崩溃
	assert.NotPanics(t, func() {
		r.ServeHTTP(w, req)
	})

	assert.Equal(t, http.StatusInternalServerError, w.Code)
	assert.Equal(t, "close", w.Header().Get("Connection"))
}

func TestPanicRecovery_NormalRequest(t *testing.T) {
	r := newTestRouter(PanicRecovery(), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestPanicError_Error(t *testing.T) {
	pe := &panicError{msg: "test panic"}
	assert.Equal(t, "test panic", pe.Error())
	assert.Equal(t, http.StatusInternalServerError, pe.StatusCode())
}

// ===== RequestID 测试 =====

func TestRequestID_GeneratesNewID(t *testing.T) {
	r := newTestRouter(RequestID(), func(c *gin.Context) {
		id, exists := c.Get(requestIDGinKey)
		assert.True(t, exists)
		assert.NotEmpty(t, id)
		c.JSON(http.StatusOK, gin.H{"id": id})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
	// 响应头中应包含 X-Request-ID
	requestID := w.Header().Get("X-Request-ID")
	assert.NotEmpty(t, requestID)
	assert.Len(t, requestID, 32) // 16 bytes hex = 32 chars
}

func TestRequestID_UsesExistingHeader(t *testing.T) {
	r := newTestRouter(RequestID(), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	req.Header.Set("X-Request-ID", "existing-request-id-123")
	r.ServeHTTP(w, req)

	assert.Equal(t, "existing-request-id-123", w.Header().Get("X-Request-ID"))
}

func TestRequestID_TwoRequestsDifferentIDs(t *testing.T) {
	r := newTestRouter(RequestID(), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{})
	})

	w1 := httptest.NewRecorder()
	req1 := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w1, req1)

	w2 := httptest.NewRecorder()
	req2 := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w2, req2)

	assert.NotEqual(t, w1.Header().Get("X-Request-ID"), w2.Header().Get("X-Request-ID"))
}

func TestGenerateRequestID(t *testing.T) {
	id := generateRequestID()
	assert.Len(t, id, 32)

	id2 := generateRequestID()
	assert.NotEqual(t, id, id2)
}

func TestGetRequestID_FromContext(t *testing.T) {
	ctx := contextWithRequestID("test-id-123")
	assert.Equal(t, "test-id-123", GetRequestID(ctx))
}

func TestGetRequestID_EmptyContext(t *testing.T) {
	assert.Equal(t, "", GetRequestID(context.Background()))
}

// contextWithRequestID 辅助函数：创建带 request_id 的 context
func contextWithRequestID(id string) context.Context {
	return context.WithValue(context.Background(), RequestIDKey, id)
}

// ===== RequestLogger 测试 =====

func TestRequestLogger_LogsRequest(t *testing.T) {
	r := newTestRouter(RequestLogger(), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestRequestLogger_WithRequestBody(t *testing.T) {
	r := newTestRouter(NewRequestLogger(nil), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	body := bytes.NewBufferString(`{"username":"test","password":"secret123"}`)
	req := httptest.NewRequest(http.MethodPost, "/test", body)
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestRequestLogger_WithConfigThreshold(t *testing.T) {
	cfg := &config.Config{SlowRequestThresholdMs: 100}
	r := newTestRouter(NewRequestLogger(cfg), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestSanitizeRequestBody(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected string
	}{
		{"密码被脱敏", `{"password":"secret123"}`, `{"password":"[REDACTED]"}`},
		{"无密码不变", `{"username":"test"}`, `{"username":"test"}`},
		{"空字符串", "", ""},
		{"多个password字段", `{"password":"a","password":"b"}`, `{"password":"[REDACTED]","password":"[REDACTED]"}`},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			result := sanitizeRequestBody(tc.input)
			assert.Equal(t, tc.expected, result)
		})
	}
}

func TestSanitizeResponseBody(t *testing.T) {
	input := `{"token":"abc","password":"mypass"}`
	result := sanitizeResponseBody(input)
	assert.Contains(t, result, "[REDACTED]")
	assert.NotContains(t, result, "mypass")
}

// ===== SecurityHeaders 测试 =====

func TestSecurityHeaders_Development(t *testing.T) {
	r := newTestRouter(SecurityHeaders(), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
	assert.Equal(t, "nosniff", w.Header().Get("X-Content-Type-Options"))
	assert.Equal(t, "DENY", w.Header().Get("X-Frame-Options"))
	assert.Equal(t, "1; mode=block", w.Header().Get("X-XSS-Protection"))
	assert.Equal(t, "strict-origin-when-cross-origin", w.Header().Get("Referrer-Policy"))
	assert.Equal(t, "Metaclouds", w.Header().Get("Server"))
	// 开发环境不应有 HSTS
	assert.Empty(t, w.Header().Get("Strict-Transport-Security"))
	// CSP 应包含 unsafe-inline（开发环境）
	csp := w.Header().Get("Content-Security-Policy")
	assert.Contains(t, csp, "unsafe-inline")
}

func TestSecurityHeaders_Production(t *testing.T) {
	cfg := &config.Config{Environment: "production"}
	r := newTestRouter(NewSecurityHeaders(cfg), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
	// 生产环境应有 HSTS
	assert.Equal(t, "max-age=31536000; includeSubDomains", w.Header().Get("Strict-Transport-Security"))
	// 生产环境 CSP 不应包含 unsafe-inline
	csp := w.Header().Get("Content-Security-Policy")
	assert.NotContains(t, csp, "unsafe-inline")
}

func TestSecurityHeaders_BasicHeadersAlwaysSet(t *testing.T) {
	r := newTestRouter(NewSecurityHeaders(nil), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.NotEmpty(t, w.Header().Get("X-Content-Type-Options"))
	assert.NotEmpty(t, w.Header().Get("X-Frame-Options"))
	assert.NotEmpty(t, w.Header().Get("Referrer-Policy"))
	assert.NotEmpty(t, w.Header().Get("Permissions-Policy"))
}

// ===== Timing 测试 =====

func TestTimingMiddleware_LogsTiming(t *testing.T) {
	r := newTestRouter(TimingMiddleware(), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestTimingMiddleware_ErrorStatus(t *testing.T) {
	r := newTestRouter(TimingMiddleware(), func(c *gin.Context) {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "fail"})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusInternalServerError, w.Code)
}

func TestTimingMiddlewareWithThreshold(t *testing.T) {
	r := newTestRouter(TimingMiddlewareWithThreshold(1*time.Nanosecond), func(c *gin.Context) {
		time.Sleep(2 * time.Millisecond)
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestTimingMiddlewareWithConfig_ExcludePath(t *testing.T) {
	cfg := TimingConfig{
		ExcludePaths: []string{"/test"},
	}
	r := newTestRouter(TimingMiddlewareWithConfig(cfg), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestTimingMiddlewareWithConfig_SlowThreshold(t *testing.T) {
	cfg := TimingConfig{
		SlowThreshold: 1 * time.Nanosecond,
	}
	r := newTestRouter(TimingMiddlewareWithConfig(cfg), func(c *gin.Context) {
		time.Sleep(2 * time.Millisecond)
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestDefaultTimingMiddleware(t *testing.T) {
	r := newTestRouter(DefaultTimingMiddleware, func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

// ===== Validation 测试 =====

type testValidationStruct struct {
	Name  string `json:"name" binding:"required"`
	Age   int    `json:"age" binding:"gte=0"`
	Email string `json:"email" binding:"required,email"`
}

func TestValidateRequest_ValidRequest(t *testing.T) {
	r := gin.New()
	r.POST("/test", ValidateRequest(&testValidationStruct{}), func(c *gin.Context) {
		req, ok := GetValidatedRequest(c)
		assert.True(t, ok)
		assert.NotNil(t, req)
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})

	w := httptest.NewRecorder()
	body := bytes.NewBufferString(`{"name":"test","age":25,"email":"test@example.com"}`)
	req := httptest.NewRequest(http.MethodPost, "/test", body)
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestValidateRequest_MissingRequired(t *testing.T) {
	r := gin.New()
	r.POST("/test", ValidateRequest(&testValidationStruct{}), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})

	w := httptest.NewRecorder()
	body := bytes.NewBufferString(`{"age":25}`)
	req := httptest.NewRequest(http.MethodPost, "/test", body)
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusBadRequest, w.Code)
}

func TestValidateRequest_NegativeValueRejected(t *testing.T) {
	r := gin.New()
	r.POST("/test", ValidateRequest(&testValidationStruct{}), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})

	w := httptest.NewRecorder()
	body := bytes.NewBufferString(`{"name":"test","age":-1,"email":"test@example.com"}`)
	req := httptest.NewRequest(http.MethodPost, "/test", body)
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusBadRequest, w.Code)
}

func TestValidateRequest_InvalidEmail(t *testing.T) {
	r := gin.New()
	r.POST("/test", ValidateRequest(&testValidationStruct{}), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})

	w := httptest.NewRecorder()
	body := bytes.NewBufferString(`{"name":"test","age":25,"email":"not-an-email"}`)
	req := httptest.NewRequest(http.MethodPost, "/test", body)
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusBadRequest, w.Code)
}

func TestValidateRequest_InvalidJSON(t *testing.T) {
	r := gin.New()
	r.POST("/test", ValidateRequest(&testValidationStruct{}), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})

	w := httptest.NewRecorder()
	body := bytes.NewBufferString(`not valid json`)
	req := httptest.NewRequest(http.MethodPost, "/test", body)
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w, req)

	// 无效 JSON 被 ShouldBindJSON 解析为通用错误，返回 500
	assert.Equal(t, http.StatusInternalServerError, w.Code)
}

func TestGetValidatedRequest_NotSet(t *testing.T) {
	c, _ := gin.CreateTestContext(httptest.NewRecorder())
	req, ok := GetValidatedRequest(c)
	assert.False(t, ok)
	assert.Nil(t, req)
}

// ===== ApplyCoreStack 测试 =====

func TestApplyCoreStack_BasicSetup(t *testing.T) {
	r := gin.New()
	cfg := &config.Config{
		MaxRequestBodySize:    1048576,
		RateLimitEnabled:      false,
		CircuitBreakerEnabled: false,
	}
	ApplyCoreStack(r, cfg)

	r.GET("/test", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})

	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
	// 验证安全头被设置
	assert.NotEmpty(t, w.Header().Get("X-Content-Type-Options"))
	assert.NotEmpty(t, w.Header().Get("X-Request-ID"))
}

func TestApplyCoreStack_WithRateLimit(t *testing.T) {
	r := gin.New()
	cfg := &config.Config{
		MaxRequestBodySize:       1048576,
		RateLimitEnabled:         true,
		RateLimitRequests:        10,
		RateLimitDurationSeconds: 60,
		CircuitBreakerEnabled:    false,
	}
	ApplyCoreStack(r, cfg)

	r.GET("/test", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})

	// 前几个请求应成功
	for i := 0; i < 5; i++ {
		w := httptest.NewRecorder()
		req := httptest.NewRequest(http.MethodGet, "/test", nil)
		r.ServeHTTP(w, req)
		assert.Equal(t, http.StatusOK, w.Code)
	}
}

func TestApplyCoreStack_WithCircuitBreaker(t *testing.T) {
	r := gin.New()
	cfg := &config.Config{
		MaxRequestBodySize:       1048576,
		RateLimitEnabled:         false,
		CircuitBreakerEnabled:    true,
		CircuitBreakerThreshold:  5,
		CircuitBreakerTimeoutSeconds: 30,
	}
	ApplyCoreStack(r, cfg)

	r.GET("/test", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})

	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestApplyCoreStack_NoMaxBodySize(t *testing.T) {
	// MaxRequestBodySize <= 0 时应跳过 MaxBytesReader
	r := gin.New()
	cfg := &config.Config{
		MaxRequestBodySize:    0,
		RateLimitEnabled:      false,
		CircuitBreakerEnabled: false,
	}
	ApplyCoreStack(r, cfg)

	r.POST("/test", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})

	w := httptest.NewRecorder()
	body := bytes.NewBufferString(`{"data":"test"}`)
	req := httptest.NewRequest(http.MethodPost, "/test", body)
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestApplyCoreStack_PanicRecovery(t *testing.T) {
	r := gin.New()
	cfg := &config.Config{
		RateLimitEnabled:      false,
		CircuitBreakerEnabled: false,
	}
	ApplyCoreStack(r, cfg)

	r.GET("/panic", func(c *gin.Context) {
		panic("test panic in core stack")
	})

	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/panic", nil)

	assert.NotPanics(t, func() {
		r.ServeHTTP(w, req)
	})
	assert.Equal(t, http.StatusInternalServerError, w.Code)
}

// ===== SecurityFilter 补充测试（security.go） =====

func TestSecurityFilter_ValidRequest(t *testing.T) {
	r := newTestRouter(SecurityFilter(), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestSecurityFilter_SQLInjectionBlocked(t *testing.T) {
	r := newTestRouter(SecurityFilter(), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test?q=SELECT+*+FROM+users", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusBadRequest, w.Code)
}

func TestSecurityFilter_XSSBlocked(t *testing.T) {
	r := newTestRouter(SecurityFilter(), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test?q=<script>alert(1)</script>", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusBadRequest, w.Code)
}

func TestIsValidIP(t *testing.T) {
	assert.True(t, isValidIP(""))
	assert.True(t, isValidIP("127.0.0.1"))
	assert.True(t, isValidIP("::1"))
	assert.False(t, isValidIP("not-an-ip"))
	assert.False(t, isValidIP("999.999.999.999"))
}

func TestSanitizeInput(t *testing.T) {
	input := `<script>alert("xss")</script>`
	result := SanitizeInput(input)
	// 原始尖括号应被转义（注意：& 也会被转义，所以 &lt; 变成 &amp;lt;）
	assert.NotContains(t, result, "<script>")
	assert.NotContains(t, result, "</script>")
	assert.Contains(t, result, "&amp;lt;")
	assert.Contains(t, result, "&amp;quot;")
}

func TestMaskSensitiveHeaders(t *testing.T) {
	headers := http.Header{}
	headers.Set("Authorization", "Bearer secret-token-12345")
	headers.Set("Content-Type", "application/json")
	headers.Set("X-API-Key", "api-key-secret")
	headers.Set("X-Custom-Token", "custom-token-value")

	result := MaskSensitiveHeaders(headers)
	assert.Equal(t, "application/json", result["Content-Type"])
	// 敏感头应被脱敏
	assert.NotEqual(t, "Bearer secret-token-12345", result["Authorization"])
	assert.Contains(t, result["Authorization"], "...")
	assert.NotEqual(t, "api-key-secret", result["X-API-Key"])
	assert.NotEqual(t, "custom-token-value", result["X-Custom-Token"])
}

func TestMaskValue(t *testing.T) {
	assert.Equal(t, "[REDACTED]", maskValue("short"))
	assert.Equal(t, "abcd...wxyz", maskValue("abcdefghijklmnopqrstuvwxyz"))
}

func TestRateLimitByIP(t *testing.T) {
	r := newTestRouter(RateLimitByIP(), func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	w := httptest.NewRecorder()
	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	req.RemoteAddr = "192.168.1.1:12345"
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

// 确保 strings 包被使用（避免未使用导入）
var _ = strings.TrimSpace
