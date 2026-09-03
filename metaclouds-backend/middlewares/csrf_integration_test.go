package middlewares

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/config"
	"metaclouds-backend/pkg/jwttool"
)

// TestCSRF_FullLoginFlow 通过真实 JWTAuth + CSRFProtect 中间件，模拟「登录 → 已登录会话发状态变更请求」
// 的端到端（HTTP 层）链路，验证 CSRF 双提交令牌与 Cookie 会话鉴权协同工作。
func TestCSRF_FullLoginFlow(t *testing.T) {
	gin.SetMode(gin.TestMode)

	secret := "test-secret-csrf-full-flow-integration-0123456789"
	cfg := &config.Config{
		Environment:       "development",
		JWTSecret:         secret,
		JWTExpirationHours: 1,
		CookieSameSite:    "lax",
	}

	tg := jwttool.NewTokenGenerator(secret, 1)
	token, _, err := tg.GenerateToken(jwttool.TokenClaims{
		UserID:   1,
		Username: "alice",
		Email:    "alice@example.com",
		Role:     "admin",
		TenantID: 0,
	})
	if err != nil {
		t.Fatalf("generate token: %v", err)
	}

	const csrfValue = "csrf-secret-value-abc123"

	r := gin.New()
	// CSRF 全局注册（与真实路由一致）。
	r.Use(NewCSRFProtect(cfg))

	// 登录端点：模拟真实 Login 写入 access_token(httpOnly) 与 csrf_token(非 httpOnly)。
	r.POST("/login", func(c *gin.Context) {
		c.SetSameSite(cfg.CookieSameSiteMode())
		c.SetCookie("access_token", token, 3600, "/", "", false, true)
		c.SetCookie("csrf_token", csrfValue, 3600, "/", "", false, false)
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})

	// 受保护路由：仅这些挂在 JWTAuth 之后，登录态通过 access_token Cookie 持有。
	protected := r.Group("")
	protected.Use(NewJWTAuth(cfg))
	protected.GET("/ping", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})
	protected.POST("/data", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{"ok": true})
	})

	// 1) 登录：公开 POST，无 access_token Cookie → CSRF 跳过，成功写入两个 Cookie。
	loginW := httptest.NewRecorder()
	loginReq, _ := http.NewRequest(http.MethodPost, "/login", nil)
	r.ServeHTTP(loginW, loginReq)
	if loginW.Code != http.StatusOK {
		t.Fatalf("login: want 200 got %d body=%s", loginW.Code, loginW.Body.String())
	}
	cookies := loginW.Result().Cookies()
	var hasAccessToken, hasCsrf bool
	for _, ck := range cookies {
		if ck.Name == "access_token" && ck.HttpOnly {
			hasAccessToken = true
		}
		if ck.Name == "csrf_token" && !ck.HttpOnly {
			hasCsrf = true
		}
	}
	if !hasAccessToken {
		t.Fatal("login response missing httpOnly access_token cookie")
	}
	if !hasCsrf {
		t.Fatal("login response missing non-httpOnly csrf_token cookie")
	}

	// 带登录 Cookie 的后续请求构造器。
	authedReq := func(method, path, csrfHeader string) *http.Request {
		req, _ := http.NewRequest(method, path, nil)
		for _, ck := range cookies {
			req.AddCookie(ck)
		}
		if csrfHeader != "" {
			req.Header.Set(csrfHeaderName, csrfHeader)
		}
		return req
	}

	// 2) GET /ping 带 access_token Cookie → 200（Cookie 会话鉴权生效）。
	w := httptest.NewRecorder()
	r.ServeHTTP(w, authedReq(http.MethodGet, "/ping", ""))
	if w.Code != http.StatusOK {
		t.Fatalf("authed GET: want 200 got %d", w.Code)
	}

	// 3) POST /data 带 Cookie + 正确 X-CSRF-Token → 200（双提交令牌校验通过）。
	w = httptest.NewRecorder()
	r.ServeHTTP(w, authedReq(http.MethodPost, "/data", csrfValue))
	if w.Code != http.StatusOK {
		t.Fatalf("authed POST with valid CSRF: want 200 got %d body=%s", w.Code, w.Body.String())
	}

	// 4) POST /data 带 Cookie 但缺 X-CSRF-Token → 403（CSRF 拦截）。
	w = httptest.NewRecorder()
	r.ServeHTTP(w, authedReq(http.MethodPost, "/data", ""))
	if w.Code != http.StatusForbidden {
		t.Fatalf("authed POST missing CSRF: want 403 got %d", w.Code)
	}

	// 5) POST /data 带 Cookie 但 X-CSRF-Token 不匹配 → 403。
	w = httptest.NewRecorder()
	r.ServeHTTP(w, authedReq(http.MethodPost, "/data", "wrong-value"))
	if w.Code != http.StatusForbidden {
		t.Fatalf("authed POST mismatched CSRF: want 403 got %d", w.Code)
	}

	// 6) POST /data 走 Bearer 头（无 Cookie）→ 200（非浏览器通道跳过 CSRF）。
	bearerW := httptest.NewRecorder()
	bearerReq, _ := http.NewRequest(http.MethodPost, "/data", nil)
	bearerReq.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(bearerW, bearerReq)
	if bearerW.Code != http.StatusOK {
		t.Fatalf("Bearer POST (no cookie): want 200 got %d", bearerW.Code)
	}
}
