package config

import (
	"net/http"
	"strings"
	"testing"
)

// baseValidConfig 返回一个在生产环境校验可顺利通过的最小合法配置。
func baseValidConfig() *Config {
	return &Config{
		ServerPort:          "8000",
		JWTSecret:           strings.Repeat("a", 32),
		JWTExpirationHours:  24,
		Environment:         "production",
		UseSQLite:           false,
		MemoryStoreEnabled:  false,
		AllowPublicRegistration: false,
		DatabaseSSLMode:     "require",
		AllowedOrigins:      []string{"https://example.com"},
	}
}

func TestValidate_CookieSameSiteNoneRequiresProduction(t *testing.T) {
	// 开发环境设 none：SameSite=None 需 Secure（https），http 下浏览器会拒存 Cookie。
	cfg := baseValidConfig()
	cfg.Environment = "development"
	cfg.UseSQLite = true
	cfg.CookieSameSite = "none"
	if err := cfg.Validate(); err == nil {
		t.Fatal("expected error when COOKIE_SAME_SITE=none in development")
	}

	// 生产环境设 none：应校验通过。
	cfg.Environment = "production"
	cfg.UseSQLite = false
	if err := cfg.Validate(); err != nil {
		t.Fatalf("unexpected error for none in production: %v", err)
	}
}

func TestCookieSameSiteMode(t *testing.T) {
	cases := map[string]struct {
		in   string
		want http.SameSite
	}{
		"default": {"", http.SameSiteLaxMode},
		"lax":     {"lax", http.SameSiteLaxMode},
		"LAX":     {"LAX", http.SameSiteLaxMode},
		"strict":  {"strict", http.SameSiteStrictMode},
		"none":    {"none", http.SameSiteNoneMode},
		"garbage": {"bogus", http.SameSiteLaxMode},
	}
	for name, tc := range cases {
		c := &Config{CookieSameSite: tc.in}
		if got := c.CookieSameSiteMode(); got != tc.want {
			t.Fatalf("%s: expected %v, got %v", name, tc.want, got)
		}
	}
}
