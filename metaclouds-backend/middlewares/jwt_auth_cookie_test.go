package middlewares

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/jwttool"
)

func TestJWTAuth_CookieFallback(t *testing.T) {
	gin.SetMode(gin.TestMode)

	secret := "test-secret-for-cookie-fallback-unit-test-0123456789"
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

	cfg := &JWTAuthConfig{Secret: secret, ExpirationHours: 1}
	mw := jwtAuthHandler(cfg)

	r := gin.New()
	r.GET("/ping", mw, func(c *gin.Context) {
		uid, _ := c.Get("user_id")
		c.JSON(http.StatusOK, gin.H{"user_id": uid})
	})

	// 1) 仅带 Cookie，无 Authorization 头 → 应通过鉴权。
	w := httptest.NewRecorder()
	req, _ := http.NewRequest(http.MethodGet, "/ping", nil)
	req.AddCookie(&http.Cookie{Name: "access_token", Value: token})
	r.ServeHTTP(w, req)
	if w.Code != http.StatusOK {
		t.Fatalf("cookie path: want 200 got %d body=%s", w.Code, w.Body.String())
	}

	// 2) 既无 Authorization 头也无 Cookie → 401。
	w2 := httptest.NewRecorder()
	req2, _ := http.NewRequest(http.MethodGet, "/ping", nil)
	r.ServeHTTP(w2, req2)
	if w2.Code != http.StatusUnauthorized {
		t.Fatalf("missing creds: want 401 got %d", w2.Code)
	}

	// 3) Cookie 中是无效令牌 → 401。
	w3 := httptest.NewRecorder()
	req3, _ := http.NewRequest(http.MethodGet, "/ping", nil)
	req3.AddCookie(&http.Cookie{Name: "access_token", Value: "not-a-valid-token"})
	r.ServeHTTP(w3, req3)
	if w3.Code != http.StatusUnauthorized {
		t.Fatalf("bad cookie token: want 401 got %d", w3.Code)
	}
}
