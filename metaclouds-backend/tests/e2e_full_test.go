package tests

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"
	"github.com/stretchr/testify/assert"

	"metaclouds-backend/api"
	"metaclouds-backend/config"
	"metaclouds-backend/controllers"
	"metaclouds-backend/models"
	"metaclouds-backend/services"
)

func setupFullTestServer(t *testing.T) (router *gin.Engine, token string) {
	cfg := &config.Config{
		JWTSecret:          "test-secret-key-for-testing-only-123456",
		JWTExpirationHours: 24,
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

func TestFullE2E_Workflow(t *testing.T) {
	t.Run("Test Login and Authentication", testLoginAndAuthentication)
	t.Run("Test Cluster Management", testClusterManagement)
	t.Run("Test Job Management", testJobManagement)
	t.Run("Test Resource Management", testResourceManagement)
	t.Run("Test Validation Error Handling", testValidationErrorHandling)
}

func testLoginAndAuthentication(t *testing.T) {
	router, token := setupFullTestServer(t)

	assert.NotEmpty(t, token, "Token should not be empty")

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/auth/profile", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
	assert.NotEmpty(t, response["data"])
}

func testClusterManagement(t *testing.T) {
	router, token := setupFullTestServer(t)

	createBody := []byte(`{"name":"Test Cluster","description":"E2E Test Cluster","status":"running","location":"us-west-2","node_count":5,"gpu_count":10,"cpu_count":40,"memory_gb":128,"storage_gb":1024}`)
	w := httptest.NewRecorder()
	req, _ := http.NewRequest("POST", "/api/v1/clusters", bytes.NewBuffer(createBody))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusCreated, w.Code)

	var createResponse map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &createResponse)
	assert.True(t, createResponse["success"].(bool))
	clusterID := int(createResponse["data"].(map[string]interface{})["id"].(float64))

	w = httptest.NewRecorder()
	req, _ = http.NewRequest("GET", "/api/v1/clusters", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
	var listResponse map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &listResponse)
	assert.True(t, listResponse["success"].(bool))
	clusters := listResponse["data"].([]interface{})
	assert.GreaterOrEqual(t, len(clusters), 1)

	updateBody := []byte(`{"name":"Updated Cluster","description":"Updated description"}`)
	w = httptest.NewRecorder()
	req, _ = http.NewRequest("PUT", fmt.Sprintf("/api/v1/clusters/%d", clusterID), bytes.NewBuffer(updateBody))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	w = httptest.NewRecorder()
	req, _ = http.NewRequest("DELETE", fmt.Sprintf("/api/v1/clusters/%d", clusterID), nil)
	req.Header.Set("Authorization", "Bearer "+token)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusNoContent, w.Code)
}

func testJobManagement(t *testing.T) {
	router, token := setupFullTestServer(t)

	createBody := []byte(`{"name":"E2E Test Job","description":"Job created during E2E testing","type":"training","priority":2,"gpus":1,"cpus":2,"memory":8,"duration":120,"cluster_id":1,"tenant_id":1,"user_id":1}`)
	w := httptest.NewRecorder()
	req, _ := http.NewRequest("POST", "/api/v1/jobs", bytes.NewBuffer(createBody))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusCreated, w.Code)

	var createResponse map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &createResponse)
	assert.True(t, createResponse["success"].(bool))
	jobID := int(createResponse["data"].(map[string]interface{})["id"].(float64))

	w = httptest.NewRecorder()
	req, _ = http.NewRequest("GET", "/api/v1/jobs", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
	var listResponse map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &listResponse)
	assert.True(t, listResponse["success"].(bool))
	jobs := listResponse["data"].([]interface{})
	assert.GreaterOrEqual(t, len(jobs), 1)

	updateBody := []byte(`{"priority":3}`)
	w = httptest.NewRecorder()
	req, _ = http.NewRequest("PUT", fmt.Sprintf("/api/v1/jobs/%d", jobID), bytes.NewBuffer(updateBody))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	w = httptest.NewRecorder()
	req, _ = http.NewRequest("DELETE", fmt.Sprintf("/api/v1/jobs/%d", jobID), nil)
	req.Header.Set("Authorization", "Bearer "+token)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusNoContent, w.Code)
}

func testResourceManagement(t *testing.T) {
	router, token := setupFullTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/resources", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.True(t, response["success"].(bool))
}

func testValidationErrorHandling(t *testing.T) {
	router, _ := setupFullTestServer(t)

	w := httptest.NewRecorder()
	reqBody := []byte(`{"username":"admin"}`)
	req, _ := http.NewRequest("POST", "/api/v1/auth/login", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusBadRequest, w.Code, "Missing password should return 400 Bad Request")

	var response map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.False(t, response["success"].(bool))

	w = httptest.NewRecorder()
	reqBody = []byte(`{"password":"password123"}`)
	req, _ = http.NewRequest("POST", "/api/v1/auth/login", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusBadRequest, w.Code, "Missing username should return 400 Bad Request")
}

func TestFullE2E_EdgeCases(t *testing.T) {
	t.Run("Test Unauthorized Access", testUnauthorizedAccess)
	t.Run("Test Invalid Credentials", testInvalidCredentials)
	t.Run("Test Duplicate Registration", testDuplicateRegistration)
}

func testUnauthorizedAccess(t *testing.T) {
	router, _ := setupFullTestServer(t)

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/auth/profile", nil)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusUnauthorized, w.Code)
}

func testInvalidCredentials(t *testing.T) {
	router, _ := setupFullTestServer(t)

	w := httptest.NewRecorder()
	reqBody := []byte(`{"username":"admin","password":"wrongpassword"}`)
	req, _ := http.NewRequest("POST", "/api/v1/auth/login", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusUnauthorized, w.Code)
}

func testDuplicateRegistration(t *testing.T) {
	router, token := setupFullTestServer(t)

	w := httptest.NewRecorder()
	reqBody := []byte(`{"username":"duplicateuser","email":"duplicate@example.com","password":"Password123!"}`)
	req, _ := http.NewRequest("POST", "/api/v1/auth/register", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusCreated, w.Code)

	w = httptest.NewRecorder()
	req, _ = http.NewRequest("POST", "/api/v1/auth/register", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)
	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusConflict, w.Code, "Duplicate registration should return 409 Conflict")
}
