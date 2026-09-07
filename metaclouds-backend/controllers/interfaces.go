package controllers

import (
	"metaclouds-backend/models"
	"metaclouds-backend/services"
)

// 以下接口定义了各控制器依赖的服务契约。
// 使用接口而非具体结构体，便于在单元测试中用 mock 替代真实服务层。
// 现有 services 包中的具体类型均已隐式实现对应接口，无需修改服务层代码。

// AuthServiceInterface 定义认证服务的接口。
type AuthServiceInterface interface {
	Login(req services.LoginRequest) (*services.LoginResponse, error)
	Register(req services.RegisterRequest) (*models.UserResponse, error)
	Refresh(userID uint) (*services.RefreshResponse, error)
	GetProfile(userID uint) (*models.UserResponse, error)
}

// ClusterServiceInterface 定义集群服务的接口。
type ClusterServiceInterface interface {
	GetClusters() ([]models.Cluster, error)
	GetCluster(id uint) (*models.Cluster, error)
	CreateCluster(req services.CreateClusterRequest) (*models.Cluster, error)
	UpdateCluster(id uint, req services.UpdateClusterRequest) (*models.Cluster, error)
	DeleteCluster(id uint) error
}

// ResourceServiceInterface 定义资源服务的接口。
type ResourceServiceInterface interface {
	GetResources() ([]models.Resource, error)
	GetResource(id uint) (*models.Resource, error)
	UpdateResource(id uint, req services.UpdateResourceRequest) (*models.Resource, error)
}

// JobServiceInterface 定义作业服务的接口。
type JobServiceInterface interface {
	GetJobsVisibleTo(tenantID uint, isAdmin bool) ([]models.Job, error)
	GetJobVisibleTo(id, tenantID uint, isAdmin bool) (*models.Job, error)
	CreateJobForUser(req services.CreateJobRequest, tenantID, userID uint, isAdmin bool) (*models.Job, error)
	UpdateJobForTenant(id, tenantID uint, isAdmin bool, req services.UpdateJobRequest) (*models.Job, error)
	DeleteJobForTenant(id, tenantID uint, isAdmin bool) error
	CancelJobForTenant(id, tenantID uint, isAdmin bool) (*models.Job, error)
}

// K8SServiceInterface 定义 K8S 服务的接口。
type K8SServiceInterface interface {
	SubmitJob(req services.SubmitJobRequest) (*services.JobStatusResponse, error)
	GetJobStatus(jobID uint) (*services.JobStatusResponse, error)
	CancelJob(jobID uint) (*services.JobStatusResponse, error)
	GetGPUResources() ([]services.GPUResource, error)
	GetClusterStatus(clusterID uint) (*services.ClusterStatus, error)
}

// MonitoringServiceInterface 定义监控服务的接口。
type MonitoringServiceInterface interface {
	GetMetrics() (map[string]interface{}, error)
	GetAlerts() ([]models.Alert, error)
	ResolveAlert(id uint) (*models.Alert, error)
}

// TenantServiceInterface 定义租户服务的接口。
type TenantServiceInterface interface {
	GetTenants() ([]models.Tenant, error)
	GetTenant(id uint) (*models.Tenant, error)
	CreateTenant(req services.CreateTenantRequest) (*models.Tenant, error)
	UpdateTenant(id uint, req services.UpdateTenantRequest) (*models.Tenant, error)
	DeleteTenant(id uint) error
}

// AccelerationServiceInterface 定义加速套件服务的接口。
type AccelerationServiceInterface interface {
	GetAccelerationSuites() ([]models.AccelerationSuite, error)
	GetAccelerationSuite(id uint) (*models.AccelerationSuite, error)
	CreateAccelerationSuite(req services.CreateAccelerationSuiteRequest) (*models.AccelerationSuite, error)
	UpdateAccelerationSuite(id uint, req services.UpdateAccelerationSuiteRequest) (*models.AccelerationSuite, error)
	DeleteAccelerationSuite(id uint) error
}

// SecurityServiceInterface 定义安全策略服务的接口。
type SecurityServiceInterface interface {
	GetSecurityPolicies() ([]models.SecurityPolicy, error)
	GetSecurityPolicy(id uint) (*models.SecurityPolicy, error)
	CreateSecurityPolicy(req services.CreateSecurityPolicyRequest) (*models.SecurityPolicy, error)
	UpdateSecurityPolicy(id uint, req services.UpdateSecurityPolicyRequest) (*models.SecurityPolicy, error)
	DeleteSecurityPolicy(id uint) error
}
