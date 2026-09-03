package middlewares

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/config"
)

// newCsrfTestRouter 构造一个仅挂载 CSRF 中间件的测试路由，便于隔离验证双提交令牌逻辑。
func newCsrfTestRouter() *gin.Engine {
	gin.SetMode(gin.TestMode)
	r := gin.New()
	r.Use(NewCSRFProtect(&config.Config{Environment: "development", CookieSameSite: "lax"}))
	r.POST("/api/v1/things", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	r.GET("/api/v1/things", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	return r
}

func doCsrfRequest(method, csrfHeader, cookie string) *httptest.ResponseRecorder {
	req := httptest.NewRequest(method, "/api/v1/things", nil)
	if cookie != "" {
		req.Header.Set("Cookie", cookie)
	}
	if csrfHeader != "" {
		req.Header.Set(csrfHeaderName, csrfHeader)
	}
	w := httptest.NewRecorder()
	newCsrfTestRouter().ServeHTTP(w, req)
	return w
}

func TestCSRF_BrowserSession_ValidHeader_Pass(t *testing.T) {
	w := doCsrfRequest(http.MethodPost, "xyz", "access_token=abc; csrf_token=xyz")
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
}

func TestCSRF_BrowserSession_MissingHeader_Forbidden(t *testing.T) {
	// 有 access_token Cookie（浏览器会话）但无 X-CSRF-Token 头。
	w := doCsrfRequest(http.MethodPost, "", "access_token=abc; csrf_token=xyz")
	if w.Code != http.StatusForbidden {
		t.Fatalf("expected 403, got %d", w.Code)
	}
}

func TestCSRF_BrowserSession_MismatchHeader_Forbidden(t *testing.T) {
	// 头值与 Cookie 值不一致。
	w := doCsrfRequest(http.MethodPost, "wrong", "access_token=abc; csrf_token=xyz")
	if w.Code != http.StatusForbidden {
		t.Fatalf("expected 403, got %d", w.Code)
	}
}

func TestCSRF_BearerChannel_Skip(t *testing.T) {
	// 非浏览器客户端（仅 Bearer 头，无 access_token Cookie）：跳过 CSRF 校验。
	req := httptest.NewRequest(http.MethodPost, "/api/v1/things", nil)
	req.Header.Set("Authorization", "Bearer sometoken")
	w := httptest.NewRecorder()
	newCsrfTestRouter().ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
}

func TestCSRF_GetMethod_NoCheck(t *testing.T) {
	// GET 为幂等读取方法，即使无 CSRF 头也应放行。
	w := doCsrfRequest(http.MethodGet, "", "access_token=abc; csrf_token=xyz")
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
}

func TestCSRF_NoSessionCookie_Skip(t *testing.T) {
	// 既无 access_token Cookie 也无 Bearer：视为匿名状态变更（如未登录的 POST），
	// 不强制 CSRF（匿名 POST 由具体接口鉴权拦截）。
	w := doCsrfRequest(http.MethodPost, "", "")
	if w.Code != http.StatusOK {
		t.Fatalf("expected 200, got %d", w.Code)
	}
}
