package tests

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/stretchr/testify/assert"

	"metaclouds-backend/api"
	"metaclouds-backend/config"
	"metaclouds-backend/controllers"
	"metaclouds-backend/models"
	"metaclouds-backend/services"
)

func TestAPI_HealthCheck(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
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

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/health", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var response map[string]string
	json.Unmarshal(w.Body.Bytes(), &response)
	assert.Equal(t, "healthy", response["status"])
}

func TestAPI_Login(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
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
}

func TestAPI_ProtectedEndpoint(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
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

	w := httptest.NewRecorder()
	req, _ := http.NewRequest("GET", "/api/v1/clusters", nil)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusUnauthorized, w.Code)
}

func TestAPI_JobPriorityScheduling(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
		JWTExpirationHours: 24,
	}

	setTestBootstrapPasswords()
	db := models.MustNewMemoryStore()
	var redisClient interface{} = nil

	authService := services.NewAuthService(db, redisClient, cfg)
	clusterService := services.NewClusterService(db, cfg)
	resourceService := services.NewResourceService(db, cfg)
	jobService := services.NewJobService(db, cfg, nil)
	monitoringService := services.NewMonitoringService(db, cfg)
	tenantService := services.NewTenantService(db, cfg)
	accelerationService := services.NewAccelerationService(db, cfg)
	securityService := services.NewSecurityService(db, cfg)
	k8sService := services.NewK8SService(db, cfg)

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

	w := httptest.NewRecorder()
	reqBody := adminLoginBody()
	req, _ := http.NewRequest("POST", "/api/v1/auth/login", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var loginResponse map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &loginResponse)
	token := loginResponse["data"].(map[string]interface{})["token"].(string)

	w = httptest.NewRecorder()
	reqBody = []byte(`{"name":"Priority Test Job","description":"Test job with priority","type":"training","priority":3,"gpus":1,"cpus":2,"memory":8,"duration":60,"cluster_id":1,"tenant_id":1,"user_id":1}`)
	req, _ = http.NewRequest("POST", "/api/v1/jobs", bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusCreated, w.Code)

	var jobResponse map[string]interface{}
	json.Unmarshal(w.Body.Bytes(), &jobResponse)
	jobID := uint(jobResponse["data"].(map[string]interface{})["id"].(float64))
	priority := int(jobResponse["data"].(map[string]interface{})["priority"].(float64))
	assert.Equal(t, 3, priority)

	w = httptest.NewRecorder()
	reqBody = []byte(`{"priority":1}`)
	req, _ = http.NewRequest("PUT", "/api/v1/jobs/"+string(rune(jobID+'0')), bytes.NewBuffer(reqBody))
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	json.Unmarshal(w.Body.Bytes(), &jobResponse)
	priority = int(jobResponse["data"].(map[string]interface{})["priority"].(float64))
	assert.Equal(t, 1, priority)

	w = httptest.NewRecorder()
	req, _ = http.NewRequest("GET", "/api/v1/jobs", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	r.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)
}
