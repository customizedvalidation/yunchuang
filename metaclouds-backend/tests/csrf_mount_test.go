package tests

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/stretchr/testify/assert"

	"metaclouds-backend/api"
	"metaclouds-backend/config"
	"metaclouds-backend/controllers"
	"metaclouds-backend/models"
	"metaclouds-backend/pkg/jwttool"
	"metaclouds-backend/services"
)

// TestRouter_CSRFEnforcedOnProtectedRoutes 验证“生产路由装配”（api.SetupRouter + RegisterRoutes，
// 即 main.go 实际使用的装配）确实挂载了 CSRF 双提交令牌中间件。
//
// 历史背景：此前 main.go 自建路由、漏挂 CSRF，而单元测试走的是 api.SetupRouter，导致“测试有 CSRF、
// 生产无 CSRF”的漏防，CSRF 保护形同虚设。此测试直接对真实装配发请求，若 SetupRouter 再次漏挂 CSRF，
// 本测试会立即失败，从而把“安全中间件是否真的挂上”纳入回归防护。
func TestRouter_CSRFEnforcedOnProtectedRoutes(t *testing.T) {
	cfg := &config.Config{
		Environment:        "development",
		JWTSecret:          "test-secret-csrf-mount-guard-0123456789",
		JWTExpirationHours: 1,
		CookieSameSite:     "lax",
	}

	setTestBootstrapPasswords()
	db := models.MustNewMemoryStore()
	var redisClient interface{} = nil

	authService := services.NewAuthService(db, redisClient, cfg)
	clusterService := services.NewClusterService(db, cfg)
	resourceService := services.NewResourceService(db, cfg)
	monitoringService := services.NewMonitoringService(db, cfg)
	tenantService := services.NewTenantService(db, cfg)
	accelerationService := services.NewAccelerationService(db, cfg)
	securityService := services.NewSecurityService(db, cfg)
	k8sService := services.NewK8SService(db, cfg)
	jobService := services.NewJobService(db, cfg, k8sService)

	authController := controllers.NewAuthController(authService)
	clusterController := controllers.NewClusterController(clusterService)
	resourceController := controllers.NewResourceController(resourceService)
	jobController := controllers.NewJobController(jobService)
	monitoringController := controllers.NewMonitoringController(monitoringService)
	tenantController := controllers.NewTenantController(tenantService)
	accelerationController := controllers.NewAccelerationController(accelerationService)
	securityController := controllers.NewSecurityController(securityService)
	k8sController := controllers.NewK8SController(k8sService)

	// 与 main.go 完全一致的装配方式。
	r := api.SetupRouter(cfg)
	api.RegisterRoutes(r, cfg, authController, clusterController, resourceController, jobController, monitoringController, tenantController, accelerationController, securityController, k8sController)

	tg := jwttool.NewTokenGenerator(cfg.JWTSecret, 1)
	token, _, err := tg.GenerateToken(jwttool.TokenClaims{
		UserID:   1,
		Username: "admin",
		Email:    "admin@example.com",
		Role:     "admin",
		TenantID: 0,
	})
	assert.NoError(t, err)

	const csrfValue = "csrf-mount-guard-value-abc123"

	// authedReq 携带浏览器会话 Cookie（access_token httpOnly + csrf_token 非 httpOnly），
	// 并按需附加 X-CSRF-Token 请求头。
	authedReq := func(csrfHeader string) *http.Request {
		req, _ := http.NewRequest(http.MethodPost, "/api/v1/clusters", nil)
		req.AddCookie(&http.Cookie{Name: "access_token", Value: token, Path: "/", HttpOnly: true})
		req.AddCookie(&http.Cookie{Name: "csrf_token", Value: csrfValue, Path: "/"})
		if csrfHeader != "" {
			req.Header.Set("X-CSRF-Token", csrfHeader)
		}
		return req
	}

	// 1) 浏览器会话 + 缺 X-CSRF-Token → CSRF 必须拦截（403），不应到达业务层。
	w := httptest.NewRecorder()
	r.ServeHTTP(w, authedReq(""))
	assert.Equal(t, http.StatusForbidden, w.Code,
		"CSRF must block a browser-session POST that omits X-CSRF-Token")

	// 2) 浏览器会话 + 正确 X-CSRF-Token → 通过 CSRF（不应是 403/401；业务层结果与此无关）。
	w = httptest.NewRecorder()
	r.ServeHTTP(w, authedReq(csrfValue))
	assert.NotEqual(t, http.StatusForbidden, w.Code,
		"a valid CSRF token must pass the middleware")
	assert.NotEqual(t, http.StatusUnauthorized, w.Code,
		"a valid session cookie must authenticate the request")
}
