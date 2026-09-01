package tests

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"os"
	"testing"

	"github.com/gin-gonic/gin"
	"github.com/stretchr/testify/assert"

	"metaclouds-backend/api"
	"metaclouds-backend/config"
	"metaclouds-backend/controllers"
	"metaclouds-backend/models"
	"metaclouds-backend/services"
)

// 测试专用引导口令。这些不是真实凭证，仅用于让内存存储使用可预测的
// 管理员/用户口令，从而摆脱对仓库中已泄露的硬编码默认值 Admin@123! 的依赖。
// 必须在创建内存存储前通过环境变量注入，因为 bootstrapPassword 在生产环境
// 缺失对应变量时会拒绝启动。
const (
	testAdminPassword = "Th1sIsA-T3st-Adm1n-Pw-2026"
	testUserPassword  = "Th1sIsA-T3st-Us3r-Pw-2026"
)

// setTestBootstrapPasswords 在创建内存存储前注入可知的测试口令。
func setTestBootstrapPasswords() {
	_ = os.Setenv("DEFAULT_ADMIN_PASSWORD", testAdminPassword)
	_ = os.Setenv("DEFAULT_USER_PASSWORD", testUserPassword)
}

// adminLoginBody 构造使用测试管理员口令的登录请求体。
func adminLoginBody() []byte {
	return []byte(`{"username":"admin","password":"` + testAdminPassword + `"}`)
}

func setupTestServer(t *testing.T) (router *gin.Engine, token string) {
	cfg := &config.Config{
		JWTSecret:          "test-secret-key-for-testing-only-123456",
		JWTExpirationHours: 24,
		// 开放注册默认关闭，注册相关用例需显式开启。
		AllowPublicRegistration: true,
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

	r := api.SetupRouter(cfg)
	api.RegisterRoutes(r, cfg, authController, clusterController, resourceController, jobController, monitoringController, tenantController, accelerationController, securityController, k8sController)

	reqBody := adminLoginBody()
	req, _ := http.NewRequest("POST", "/api/v1/auth/login", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")

	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var loginResponse map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &loginResponse)
	token = loginResponse["data"].(map[string]interface{})["token"].(string)

	return r, token
}

func TestE2E_HealthCheck(t *testing.T) {
	r, _ := setupTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/health", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]string
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.Equal(t, "healthy", response["status"])
}

func TestE2E_LoginSuccess(t *testing.T) {
	r, _ := setupTestServer(t)

	reqBody := adminLoginBody()
	req, _ := http.NewRequest("POST", "/api/v1/auth/login", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")

	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
	assert.NotEmpty(t, response["data"])
	assert.NotEmpty(t, response["data"].(map[string]interface{})["token"])
}

func TestE2E_LoginFailure_InvalidCredentials(t *testing.T) {
	r, _ := setupTestServer(t)

	reqBody := []byte(`{"username":"admin","password":"wrongpassword"}`)
	req, _ := http.NewRequest("POST", "/api/v1/auth/login", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")

	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusUnauthorized, w.Code)
}

func TestE2E_LoginFailure_MissingPassword(t *testing.T) {
	r, _ := setupTestServer(t)

	reqBody := []byte(`{"username":"admin"}`)
	req, _ := http.NewRequest("POST", "/api/v1/auth/login", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")

	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusBadRequest, w.Code)
}

func TestE2E_GetProfile(t *testing.T) {
	r, token := setupTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/auth/profile", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
	assert.NotEmpty(t, response["data"])

	// profile 的响应形状必须与 login/refresh 的 user 字段一致（models.UserResponse）。
	// 这里把契约钉死，防止再退化成扁平的 {user_id, ...}。
	data := response["data"].(map[string]interface{})
	assert.Equal(t, "admin", data["username"])
	assert.Equal(t, "admin", data["role"])
	for _, field := range []string{"id", "username", "email", "role", "tenant_id", "created_at", "updated_at"} {
		assert.Contains(t, data, field)
	}
	// 旧契约的字段名不应再出现
	assert.NotContains(t, data, "user_id")
}

func TestE2E_GetProfile_NoToken(t *testing.T) {
	r, _ := setupTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/auth/profile", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusUnauthorized, w.Code)
}

func TestE2E_GetProfile_InvalidToken(t *testing.T) {
	r, _ := setupTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/auth/profile", nil)
	req.Header.Set("Authorization", "Bearer invalid-token-12345")
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusUnauthorized, w.Code)
}

func TestE2E_GetClusters(t *testing.T) {
	r, token := setupTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/clusters", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
	assert.IsType(t, []interface{}{}, response["data"])
}

func TestE2E_CreateCluster(t *testing.T) {
	r, token := setupTestServer(t)

	reqBody := []byte(`{"name":"Test Cluster","description":"Test cluster for E2E testing","status":"running","location":"us-west-2","node_count":5,"gpu_count":10,"cpu_count":40,"memory_gb":128,"storage_gb":1024}`)
	req, _ := http.NewRequest("POST", "/api/v1/clusters", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)

	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusCreated, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
	assert.NotEmpty(t, response["data"])
	assert.Equal(t, "Test Cluster", response["data"].(map[string]interface{})["name"])
}

func TestE2E_GetResources(t *testing.T) {
	r, token := setupTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/resources", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
}

func TestE2E_CreateJob(t *testing.T) {
	r, token := setupTestServer(t)

	reqBody := []byte(`{"name":"E2E Test Job","description":"Job created during E2E testing","type":"training","priority":2,"gpus":1,"cpus":2,"memory":8,"duration":120,"cluster_id":1,"tenant_id":1,"user_id":1}`)
	req, _ := http.NewRequest("POST", "/api/v1/jobs", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)

	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusCreated, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
	assert.NotEmpty(t, response["data"])
}

func TestE2E_GetJobs(t *testing.T) {
	r, token := setupTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/jobs", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
	assert.IsType(t, []interface{}{}, response["data"])
}

func TestE2E_GetTenants(t *testing.T) {
	r, token := setupTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/tenants", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
}

func TestE2E_GetSecurityPolicies(t *testing.T) {
	r, token := setupTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/security/policies", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
}

func TestE2E_GetAccelerationSuites(t *testing.T) {
	r, token := setupTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/acceleration", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
}

func TestE2E_GetMonitoringMetrics(t *testing.T) {
	r, token := setupTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/monitoring/metrics", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
}

func TestE2E_GetMonitoringAlerts(t *testing.T) {
	r, token := setupTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/monitoring/alerts", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}

func TestE2E_RegisterUser(t *testing.T) {
	r, _ := setupTestServer(t)

	reqBody := []byte(`{"username":"testuser","email":"test@example.com","password":"Test@123!","role":"user"}`)
	req, _ := http.NewRequest("POST", "/api/v1/auth/register", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")

	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusCreated, w.Code)
}

func TestE2E_RegisterUser_InvalidEmail(t *testing.T) {
	r, _ := setupTestServer(t)

	reqBody := []byte(`{"username":"testuser","email":"invalid-email","password":"Test@123!","role":"user"}`)
	req, _ := http.NewRequest("POST", "/api/v1/auth/register", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")

	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	assert.NotEqual(t, http.StatusOK, w.Code)
}
