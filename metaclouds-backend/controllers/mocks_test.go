package controllers

import (
	"metaclouds-backend/models"
	"metaclouds-backend/services"
)

// ===== Mock AuthService =====

type mockAuthService struct {
	loginFunc     func(req services.LoginRequest) (*services.LoginResponse, error)
	registerFunc  func(req services.RegisterRequest) (*models.UserResponse, error)
	refreshFunc   func(userID uint) (*services.RefreshResponse, error)
	getProfileFunc func(userID uint) (*models.UserResponse, error)
}

func (m *mockAuthService) Login(req services.LoginRequest) (*services.LoginResponse, error) {
	return m.loginFunc(req)
}
func (m *mockAuthService) Register(req services.RegisterRequest) (*models.UserResponse, error) {
	return m.registerFunc(req)
}
func (m *mockAuthService) Refresh(userID uint) (*services.RefreshResponse, error) {
	return m.refreshFunc(userID)
}
func (m *mockAuthService) GetProfile(userID uint) (*models.UserResponse, error) {
	return m.getProfileFunc(userID)
}

// ===== Mock ClusterService =====

type mockClusterService struct {
	getClustersFunc   func() ([]models.Cluster, error)
	getClusterFunc    func(id uint) (*models.Cluster, error)
	createClusterFunc func(req services.CreateClusterRequest) (*models.Cluster, error)
	updateClusterFunc func(id uint, req services.UpdateClusterRequest) (*models.Cluster, error)
	deleteClusterFunc func(id uint) error
}

func (m *mockClusterService) GetClusters() ([]models.Cluster, error) {
	return m.getClustersFunc()
}
func (m *mockClusterService) GetCluster(id uint) (*models.Cluster, error) {
	return m.getClusterFunc(id)
}
func (m *mockClusterService) CreateCluster(req services.CreateClusterRequest) (*models.Cluster, error) {
	return m.createClusterFunc(req)
}
func (m *mockClusterService) UpdateCluster(id uint, req services.UpdateClusterRequest) (*models.Cluster, error) {
	return m.updateClusterFunc(id, req)
}
func (m *mockClusterService) DeleteCluster(id uint) error {
	return m.deleteClusterFunc(id)
}

// ===== Mock ResourceService =====

type mockResourceService struct {
	getResourcesFunc   func() ([]models.Resource, error)
	getResourceFunc    func(id uint) (*models.Resource, error)
	updateResourceFunc func(id uint, req services.UpdateResourceRequest) (*models.Resource, error)
}

func (m *mockResourceService) GetResources() ([]models.Resource, error) {
	return m.getResourcesFunc()
}
func (m *mockResourceService) GetResource(id uint) (*models.Resource, error) {
	return m.getResourceFunc(id)
}
func (m *mockResourceService) UpdateResource(id uint, req services.UpdateResourceRequest) (*models.Resource, error) {
	return m.updateResourceFunc(id, req)
}

// ===== Mock JobService =====

type mockJobService struct {
	getJobsVisibleToFunc    func(tenantID uint, isAdmin bool) ([]models.Job, error)
	getJobVisibleToFunc     func(id, tenantID uint, isAdmin bool) (*models.Job, error)
	createJobForUserFunc    func(req services.CreateJobRequest, tenantID, userID uint, isAdmin bool) (*models.Job, error)
	updateJobForTenantFunc  func(id, tenantID uint, isAdmin bool, req services.UpdateJobRequest) (*models.Job, error)
	deleteJobForTenantFunc  func(id, tenantID uint, isAdmin bool) error
	cancelJobForTenantFunc  func(id, tenantID uint, isAdmin bool) (*models.Job, error)
}

func (m *mockJobService) GetJobsVisibleTo(tenantID uint, isAdmin bool) ([]models.Job, error) {
	return m.getJobsVisibleToFunc(tenantID, isAdmin)
}
func (m *mockJobService) GetJobVisibleTo(id, tenantID uint, isAdmin bool) (*models.Job, error) {
	return m.getJobVisibleToFunc(id, tenantID, isAdmin)
}
func (m *mockJobService) CreateJobForUser(req services.CreateJobRequest, tenantID, userID uint, isAdmin bool) (*models.Job, error) {
	return m.createJobForUserFunc(req, tenantID, userID, isAdmin)
}
func (m *mockJobService) UpdateJobForTenant(id, tenantID uint, isAdmin bool, req services.UpdateJobRequest) (*models.Job, error) {
	return m.updateJobForTenantFunc(id, tenantID, isAdmin, req)
}
func (m *mockJobService) DeleteJobForTenant(id, tenantID uint, isAdmin bool) error {
	return m.deleteJobForTenantFunc(id, tenantID, isAdmin)
}
func (m *mockJobService) CancelJobForTenant(id, tenantID uint, isAdmin bool) (*models.Job, error) {
	return m.cancelJobForTenantFunc(id, tenantID, isAdmin)
}

// ===== Mock K8SService =====

type mockK8SService struct {
	submitJobFunc       func(req services.SubmitJobRequest) (*services.JobStatusResponse, error)
	getJobStatusFunc    func(jobID uint) (*services.JobStatusResponse, error)
	cancelJobFunc       func(jobID uint) (*services.JobStatusResponse, error)
	getGPUResourcesFunc func() ([]services.GPUResource, error)
	getClusterStatusFunc func(clusterID uint) (*services.ClusterStatus, error)
}

func (m *mockK8SService) SubmitJob(req services.SubmitJobRequest) (*services.JobStatusResponse, error) {
	return m.submitJobFunc(req)
}
func (m *mockK8SService) GetJobStatus(jobID uint) (*services.JobStatusResponse, error) {
	return m.getJobStatusFunc(jobID)
}
func (m *mockK8SService) CancelJob(jobID uint) (*services.JobStatusResponse, error) {
	return m.cancelJobFunc(jobID)
}
func (m *mockK8SService) GetGPUResources() ([]services.GPUResource, error) {
	return m.getGPUResourcesFunc()
}
func (m *mockK8SService) GetClusterStatus(clusterID uint) (*services.ClusterStatus, error) {
	return m.getClusterStatusFunc(clusterID)
}

// ===== Mock MonitoringService =====

type mockMonitoringService struct {
	getMetricsFunc    func() (map[string]interface{}, error)
	getAlertsFunc     func() ([]models.Alert, error)
	resolveAlertFunc  func(id uint) (*models.Alert, error)
}

func (m *mockMonitoringService) GetMetrics() (map[string]interface{}, error) {
	return m.getMetricsFunc()
}
func (m *mockMonitoringService) GetAlerts() ([]models.Alert, error) {
	return m.getAlertsFunc()
}
func (m *mockMonitoringService) ResolveAlert(id uint) (*models.Alert, error) {
	return m.resolveAlertFunc(id)
}

// ===== Mock TenantService =====

type mockTenantService struct {
	getTenantsFunc   func() ([]models.Tenant, error)
	getTenantFunc    func(id uint) (*models.Tenant, error)
	createTenantFunc func(req services.CreateTenantRequest) (*models.Tenant, error)
	updateTenantFunc func(id uint, req services.UpdateTenantRequest) (*models.Tenant, error)
	deleteTenantFunc func(id uint) error
}

func (m *mockTenantService) GetTenants() ([]models.Tenant, error) {
	return m.getTenantsFunc()
}
func (m *mockTenantService) GetTenant(id uint) (*models.Tenant, error) {
	return m.getTenantFunc(id)
}
func (m *mockTenantService) CreateTenant(req services.CreateTenantRequest) (*models.Tenant, error) {
	return m.createTenantFunc(req)
}
func (m *mockTenantService) UpdateTenant(id uint, req services.UpdateTenantRequest) (*models.Tenant, error) {
	return m.updateTenantFunc(id, req)
}
func (m *mockTenantService) DeleteTenant(id uint) error {
	return m.deleteTenantFunc(id)
}

// ===== Mock AccelerationService =====

type mockAccelerationService struct {
	getAccelerationSuitesFunc   func() ([]models.AccelerationSuite, error)
	getAccelerationSuiteFunc    func(id uint) (*models.AccelerationSuite, error)
	createAccelerationSuiteFunc func(req services.CreateAccelerationSuiteRequest) (*models.AccelerationSuite, error)
	updateAccelerationSuiteFunc func(id uint, req services.UpdateAccelerationSuiteRequest) (*models.AccelerationSuite, error)
	deleteAccelerationSuiteFunc func(id uint) error
}

func (m *mockAccelerationService) GetAccelerationSuites() ([]models.AccelerationSuite, error) {
	return m.getAccelerationSuitesFunc()
}
func (m *mockAccelerationService) GetAccelerationSuite(id uint) (*models.AccelerationSuite, error) {
	return m.getAccelerationSuiteFunc(id)
}
func (m *mockAccelerationService) CreateAccelerationSuite(req services.CreateAccelerationSuiteRequest) (*models.AccelerationSuite, error) {
	return m.createAccelerationSuiteFunc(req)
}
func (m *mockAccelerationService) UpdateAccelerationSuite(id uint, req services.UpdateAccelerationSuiteRequest) (*models.AccelerationSuite, error) {
	return m.updateAccelerationSuiteFunc(id, req)
}
func (m *mockAccelerationService) DeleteAccelerationSuite(id uint) error {
	return m.deleteAccelerationSuiteFunc(id)
}

// ===== Mock SecurityService =====

type mockSecurityService struct {
	getSecurityPoliciesFunc   func() ([]models.SecurityPolicy, error)
	getSecurityPolicyFunc    func(id uint) (*models.SecurityPolicy, error)
	createSecurityPolicyFunc func(req services.CreateSecurityPolicyRequest) (*models.SecurityPolicy, error)
	updateSecurityPolicyFunc func(id uint, req services.UpdateSecurityPolicyRequest) (*models.SecurityPolicy, error)
	deleteSecurityPolicyFunc func(id uint) error
}

func (m *mockSecurityService) GetSecurityPolicies() ([]models.SecurityPolicy, error) {
	return m.getSecurityPoliciesFunc()
}
func (m *mockSecurityService) GetSecurityPolicy(id uint) (*models.SecurityPolicy, error) {
	return m.getSecurityPolicyFunc(id)
}
func (m *mockSecurityService) CreateSecurityPolicy(req services.CreateSecurityPolicyRequest) (*models.SecurityPolicy, error) {
	return m.createSecurityPolicyFunc(req)
}
func (m *mockSecurityService) UpdateSecurityPolicy(id uint, req services.UpdateSecurityPolicyRequest) (*models.SecurityPolicy, error) {
	return m.updateSecurityPolicyFunc(id, req)
}
func (m *mockSecurityService) DeleteSecurityPolicy(id uint) error {
	return m.deleteSecurityPolicyFunc(id)
}
