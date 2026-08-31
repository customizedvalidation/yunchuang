package models

import (
	"fmt"
	"strings"
	"sync"
	"time"

	"metaclouds-backend/config"
	"metaclouds-backend/pkg/logger"

	"golang.org/x/crypto/bcrypt"
	"gorm.io/driver/postgres"
	"gorm.io/driver/sqlite"
	"gorm.io/gorm"
)

var (
	memoryStore     *MemoryStore
	memoryStoreOnce sync.Once
)

func MustNewMemoryStore() *MemoryStore {
	db, err := NewMemoryStore()
	if err != nil {
		panic(fmt.Sprintf("Failed to create memory store: %v", err))
	}
	return db
}

type MemoryStore struct {
	Mu                 sync.RWMutex
	Users              map[uint]*User
	UsersByUsername    map[string]*User
	UsersByEmail       map[string]*User
	Clusters           map[uint]*Cluster
	Resources          map[uint]*Resource
	ResourcesByStatus  map[string]map[uint]*Resource
	ResourcesByCluster map[uint]map[uint]*Resource
	ResourcesByType    map[string]map[uint]*Resource
	Jobs               map[uint]*Job
	JobsByStatus       map[string]map[uint]*Job
	JobsByPriority     map[int]map[uint]*Job
	JobsByTenant       map[uint]map[uint]*Job
	Tenants            map[uint]*Tenant
	AccelerationSuites map[uint]*AccelerationSuite
	SecurityPolicies   map[uint]*SecurityPolicy
	Alerts             map[uint]*Alert

	UserSeq              uint
	ClusterSeq           uint
	ResourceSeq          uint
	JobSeq               uint
	TenantSeq            uint
	AccelerationSuiteSeq uint
	SecurityPolicySeq    uint
	AlertSeq             uint

	PriorityChanged chan uint
}

func GetMemoryStore() (*MemoryStore, error) {
	var initErr error
	memoryStoreOnce.Do(func() {
		var err error
		memoryStore, err = NewMemoryStore()
		initErr = err
	})
	if initErr != nil {
		return nil, initErr
	}
	return memoryStore, nil
}

func NewMemoryStore() (*MemoryStore, error) {
	store := &MemoryStore{
		Users:              make(map[uint]*User),
		UsersByUsername:    make(map[string]*User),
		UsersByEmail:       make(map[string]*User),
		Clusters:           make(map[uint]*Cluster),
		Resources:          make(map[uint]*Resource),
		ResourcesByStatus:  make(map[string]map[uint]*Resource),
		ResourcesByCluster: make(map[uint]map[uint]*Resource),
		ResourcesByType:    make(map[string]map[uint]*Resource),
		Jobs:               make(map[uint]*Job),
		JobsByStatus:       make(map[string]map[uint]*Job),
		JobsByPriority:     make(map[int]map[uint]*Job),
		JobsByTenant:       make(map[uint]map[uint]*Job),
		Tenants:            make(map[uint]*Tenant),
		AccelerationSuites: make(map[uint]*AccelerationSuite),
		SecurityPolicies:   make(map[uint]*SecurityPolicy),
		Alerts:             make(map[uint]*Alert),
		PriorityChanged:    make(chan uint, 100),
	}

	if err := store.initDefaultData(); err != nil {
		return nil, err
	}
	return store, nil
}

func (s *MemoryStore) initDefaultData() error {
	// 引导口令必须来自环境变量。生产环境缺失时拒绝启动，
	// 非生产环境生成一次性随机口令，不再回退到硬编码常量。
	defaultAdminPassword, err := bootstrapPassword("DEFAULT_ADMIN_PASSWORD", "admin")
	if err != nil {
		return err
	}

	hashedPassword, err := bcrypt.GenerateFromPassword([]byte(defaultAdminPassword), bcrypt.DefaultCost)
	if err != nil {
		logger.ErrorWithCtx(nil, "Failed to hash default password", err)
		return fmt.Errorf("failed to hash default password: %w", err)
	}
	adminUser := &User{
		ID:        1,
		Username:  "admin",
		Email:     "admin@metaclouds.com",
		Password:  string(hashedPassword),
		Role:      "admin",
		TenantID:  0,
		CreatedAt: time.Now(),
		UpdatedAt: time.Now(),
	}
	s.AddUserWithIndex(adminUser)
	s.UserSeq = 2

	defaultTenant := &Tenant{
		ID:           1,
		Name:         "Default Tenant",
		Description:  "Default system tenant",
		Status:       "active",
		GPUQuota:     100,
		CPUQuota:     1000,
		MemoryQuota:  10000,
		StorageQuota: 10000,
		CreatedAt:    time.Now(),
		UpdatedAt:    time.Now(),
	}
	s.Tenants[1] = defaultTenant
	s.TenantSeq = 2

	cluster1 := &Cluster{
		ID:          1,
		Name:        "GPU-Cluster-1",
		Description: "Primary GPU cluster for deep learning",
		Status:      "active",
		Nodes:       5,
		GPUs:        40,
		CPUs:        256,
		Memory:      2048,
		Storage:     100,
		NetworkType: "InfiniBand",
		Location:    "US-West",
		CreatedAt:   time.Now(),
		UpdatedAt:   time.Now(),
	}
	s.Clusters[1] = cluster1

	cluster2 := &Cluster{
		ID:          2,
		Name:        "CPU-Cluster-1",
		Description: "CPU cluster for general computing",
		Status:      "active",
		Nodes:       10,
		GPUs:        0,
		CPUs:        512,
		Memory:      4096,
		Storage:     200,
		NetworkType: "Ethernet",
		Location:    "US-East",
		CreatedAt:   time.Now(),
		UpdatedAt:   time.Now(),
	}
	s.Clusters[2] = cluster2
	s.ClusterSeq = 3

	resource1 := &Resource{
		ID:          1,
		ClusterID:   1,
		Name:        "NVIDIA-A100-0",
		Type:        "gpu",
		Status:      "available",
		Total:       1,
		Used:        0,
		Available:   1,
		Utilization: 0,
		Details:     "NVIDIA A100 80GB",
		CreatedAt:   time.Now(),
		UpdatedAt:   time.Now(),
	}
	s.AddResourceWithIndex(resource1)
	s.ResourceSeq = 2

	for i := 2; i <= 40; i++ {
		s.AddResourceWithIndex(&Resource{
			ID:          uint(i),
			ClusterID:   1,
			Name:        fmt.Sprintf("NVIDIA-A100-%d", i-1),
			Type:        "gpu",
			Status:      "available",
			Total:       1,
			Used:        0,
			Available:   1,
			Utilization: 0,
			Details:     "NVIDIA A100 80GB",
			CreatedAt:   time.Now(),
			UpdatedAt:   time.Now(),
		})
	}

	job1 := &Job{
		ID:          1,
		Name:        "training-job-1",
		Description: "Deep learning training job",
		Type:        "training",
		Status:      "running",
		Priority:    1,
		GPUs:        4,
		CPUs:        32,
		Memory:      128,
		Duration:    3600,
		ClusterID:   1,
		TenantID:    1,
		UserID:      1,
		Progress:    65,
		CreatedAt:   time.Now(),
		UpdatedAt:   time.Now(),
	}
	s.AddJobWithIndex(job1)

	job2 := &Job{
		ID:          2,
		Name:        "inference-job-1",
		Description: "Model inference job",
		Type:        "inference",
		Status:      "completed",
		Priority:    0,
		GPUs:        1,
		CPUs:        8,
		Memory:      32,
		Duration:    1800,
		ClusterID:   1,
		TenantID:    1,
		UserID:      1,
		Progress:    100,
		CreatedAt:   time.Now().Add(-time.Hour),
		UpdatedAt:   time.Now(),
	}
	s.AddJobWithIndex(job2)
	s.JobSeq = 3

	accelerationSuite1 := &AccelerationSuite{
		ID:          1,
		Name:        "CUDA-11.7",
		Description: "NVIDIA CUDA 11.7 toolkit",
		Type:        "cuda",
		Version:     "11.7.1",
		Status:      "active",
		Enabled:     true,
		Details:     "{\"framework\": \"PyTorch\", \"compute_capability\": \"8.0\"}",
		CreatedAt:   time.Now(),
		UpdatedAt:   time.Now(),
	}
	s.AccelerationSuites[1] = accelerationSuite1
	s.AccelerationSuiteSeq = 2

	securityPolicy1 := &SecurityPolicy{
		ID:          1,
		Name:        "network-isolation",
		Description: "Enable network isolation between tenants",
		Type:        "network",
		Status:      "active",
		Enabled:     true,
		Rules:       "{\"ingress\": \"deny-all\", \"egress\": \"allow-local\"}",
		Details:     "Network isolation policy",
		CreatedAt:   time.Now(),
		UpdatedAt:   time.Now(),
	}
	s.SecurityPolicies[1] = securityPolicy1
	s.SecurityPolicySeq = 2

	alert1 := &Alert{
		ID:        1,
		Type:      "resource",
		Level:     "warning",
		Status:    "active",
		Message:   "GPU utilization exceeds 90%",
		Details:   "{\"gpu_id\": \"NVIDIA-A100-0\", \"utilization\": 95}",
		CreatedAt: time.Now(),
		UpdatedAt: time.Now(),
	}
	s.Alerts[1] = alert1
	s.AlertSeq = 2

	return nil
}

func InitDB(cfg *config.Config) (interface{}, error) {
	start := time.Now()
	logger.InfoWithCtx(nil, "Database initialization started",
		"memory_store_enabled", cfg.MemoryStoreEnabled,
		"use_sqlite", cfg.UseSQLite,
		"database_host", cfg.DatabaseHost,
		"database_port", cfg.DatabasePort,
		"database_name", cfg.DatabaseName)

	if cfg.MemoryStoreEnabled {
		logger.InfoWithCtx(nil, "Database initialization - Using in-memory store (no CGO required)",
			"duration", time.Since(start))
		return GetMemoryStore()
	}

	if cfg.UseSQLite {
		logger.InfoWithCtx(nil, "Database initialization - Using SQLite database",
			"duration", time.Since(start))
		return initSQLite(cfg)
	}

	logger.InfoWithCtx(nil, "Database initialization - Using PostgreSQL database",
		"duration", time.Since(start))
	return initPostgreSQL(cfg)
}

func initSQLite(cfg *config.Config) (*gorm.DB, error) {
	start := time.Now()
	logger.DebugWithCtx(nil, "SQLite initialization started",
		"database_file", "metaclouds.db")

	db, err := gorm.Open(sqlite.Open("metaclouds.db"), &gorm.Config{})
	if err != nil {
		logger.ErrorWithCtx(nil, "SQLite initialization failed - failed to open database", err,
			"duration", time.Since(start))
		return nil, fmt.Errorf("failed to open SQLite database: %w", err)
	}

	logger.DebugWithCtx(nil, "SQLite connection established",
		"duration", time.Since(start))

	migrateStart := time.Now()
	if err := db.AutoMigrate(
		&User{},
		&Tenant{},
		&Cluster{},
		&Resource{},
		&Job{},
		&AccelerationSuite{},
		&SecurityPolicy{},
		&Alert{},
	); err != nil {
		logger.ErrorWithCtx(nil, "SQLite initialization failed - migration error", err,
			"migration_duration", time.Since(migrateStart),
			"total_duration", time.Since(start))
		return nil, fmt.Errorf("failed to migrate SQLite database: %w", err)
	}
	logger.DebugWithCtx(nil, "SQLite migration completed",
		"migration_duration", time.Since(migrateStart))

	initDataStart := time.Now()
	if err := InitData(db); err != nil {
		logger.WarnWithCtx(nil, "SQLite initialization - failed to initialize default data",
			"error", err,
			"init_data_duration", time.Since(initDataStart))
	} else {
		logger.DebugWithCtx(nil, "SQLite default data initialized",
			"init_data_duration", time.Since(initDataStart))
	}

	logger.InfoWithCtx(nil, "SQLite initialization completed successfully",
		"duration", time.Since(start))
	return db, nil
}

func initPostgreSQL(cfg *config.Config) (*gorm.DB, error) {
	start := time.Now()
	logger.DebugWithCtx(nil, "PostgreSQL initialization started",
		"database_host", cfg.DatabaseHost,
		"database_port", cfg.DatabasePort,
		"database_name", cfg.DatabaseName,
		"ssl_mode", cfg.DatabaseSSLMode)

	dsn := cfg.GetDatabaseDSN()
	logger.DebugWithCtx(nil, "PostgreSQL DSN constructed",
		"dsn", maskDSN(dsn))

	db, err := gorm.Open(postgres.Open(dsn), &gorm.Config{})
	if err != nil {
		logger.ErrorWithCtx(nil, "PostgreSQL initialization failed - failed to open database", err,
			"database_host", cfg.DatabaseHost,
			"database_port", cfg.DatabasePort,
			"duration", time.Since(start))
		return nil, fmt.Errorf("failed to open PostgreSQL database: %w", err)
	}

	logger.DebugWithCtx(nil, "PostgreSQL connection established",
		"duration", time.Since(start))

	sqlDB, err := db.DB()
	if err != nil {
		logger.ErrorWithCtx(nil, "PostgreSQL initialization failed - failed to get underlying SQL database", err,
			"duration", time.Since(start))
		return nil, fmt.Errorf("failed to get underlying SQL database: %w", err)
	}

	sqlDB.SetMaxOpenConns(100)
	sqlDB.SetMaxIdleConns(20)
	sqlDB.SetConnMaxLifetime(300 * time.Second)
	sqlDB.SetConnMaxIdleTime(60 * time.Second)

	logger.DebugWithCtx(nil, "PostgreSQL connection pool configured",
		"max_open_conns", 100,
		"max_idle_conns", 20,
		"conn_max_lifetime", "300s",
		"conn_max_idle_time", "60s")

	pingStart := time.Now()
	if err := sqlDB.Ping(); err != nil {
		logger.ErrorWithCtx(nil, "PostgreSQL initialization failed - ping failed", err,
			"ping_duration", time.Since(pingStart),
			"total_duration", time.Since(start))
		return nil, fmt.Errorf("failed to ping PostgreSQL database: %w", err)
	}
	logger.DebugWithCtx(nil, "PostgreSQL ping successful",
		"ping_duration", time.Since(pingStart))

	migrateStart := time.Now()
	if err := db.AutoMigrate(
		&User{},
		&Tenant{},
		&Cluster{},
		&Resource{},
		&Job{},
		&AccelerationSuite{},
		&SecurityPolicy{},
		&Alert{},
	); err != nil {
		logger.ErrorWithCtx(nil, "PostgreSQL initialization failed - migration error", err,
			"migration_duration", time.Since(migrateStart),
			"total_duration", time.Since(start))
		return nil, fmt.Errorf("failed to migrate PostgreSQL database: %w", err)
	}
	logger.DebugWithCtx(nil, "PostgreSQL migration completed",
		"migration_duration", time.Since(migrateStart))

	initDataStart := time.Now()
	if err := InitData(db); err != nil {
		logger.WarnWithCtx(nil, "PostgreSQL initialization - failed to initialize default data",
			"error", err,
			"init_data_duration", time.Since(initDataStart))
	} else {
		logger.DebugWithCtx(nil, "PostgreSQL default data initialized",
			"init_data_duration", time.Since(initDataStart))
	}

	logger.InfoWithCtx(nil, "PostgreSQL initialization completed successfully",
		"database_host", cfg.DatabaseHost,
		"database_port", cfg.DatabasePort,
		"database_name", cfg.DatabaseName,
		"duration", time.Since(start))

	return db, nil
}

func maskDSN(dsn string) string {
	parts := strings.Split(dsn, " ")
	result := make([]string, 0, len(parts))
	for _, part := range parts {
		if strings.HasPrefix(part, "password=") {
			result = append(result, "password=******")
		} else {
			result = append(result, part)
		}
	}
	return strings.Join(result, " ")
}

func (s *MemoryStore) AddUserWithIndex(user *User) {
	s.Users[user.ID] = user
	s.UsersByUsername[user.Username] = user
	if user.Email != "" {
		s.UsersByEmail[user.Email] = user
	}
}

func (s *MemoryStore) RemoveUserFromIndex(userID uint) {
	if user, exists := s.Users[userID]; exists {
		delete(s.UsersByUsername, user.Username)
		if user.Email != "" {
			delete(s.UsersByEmail, user.Email)
		}
		delete(s.Users, userID)
	}
}

func (s *MemoryStore) AddResourceWithIndex(resource *Resource) {
	s.Resources[resource.ID] = resource

	if s.ResourcesByStatus[resource.Status] == nil {
		s.ResourcesByStatus[resource.Status] = make(map[uint]*Resource)
	}
	s.ResourcesByStatus[resource.Status][resource.ID] = resource

	if s.ResourcesByCluster[resource.ClusterID] == nil {
		s.ResourcesByCluster[resource.ClusterID] = make(map[uint]*Resource)
	}
	s.ResourcesByCluster[resource.ClusterID][resource.ID] = resource

	if s.ResourcesByType[resource.Type] == nil {
		s.ResourcesByType[resource.Type] = make(map[uint]*Resource)
	}
	s.ResourcesByType[resource.Type][resource.ID] = resource
}

func (s *MemoryStore) RemoveResourceFromIndex(resourceID uint) {
	if resource, exists := s.Resources[resourceID]; exists {
		delete(s.ResourcesByStatus[resource.Status], resourceID)
		delete(s.ResourcesByCluster[resource.ClusterID], resourceID)
		delete(s.ResourcesByType[resource.Type], resourceID)
		delete(s.Resources, resourceID)
	}
}

func (s *MemoryStore) UpdateResourceStatus(resourceID uint, newStatus string) {
	if resource, exists := s.Resources[resourceID]; exists {
		delete(s.ResourcesByStatus[resource.Status], resourceID)
		resource.Status = newStatus
		if s.ResourcesByStatus[newStatus] == nil {
			s.ResourcesByStatus[newStatus] = make(map[uint]*Resource)
		}
		s.ResourcesByStatus[newStatus][resourceID] = resource
	}
}

func (s *MemoryStore) AddJobWithIndex(job *Job) {
	s.Jobs[job.ID] = job

	if s.JobsByStatus[job.Status] == nil {
		s.JobsByStatus[job.Status] = make(map[uint]*Job)
	}
	s.JobsByStatus[job.Status][job.ID] = job

	if s.JobsByPriority[job.Priority] == nil {
		s.JobsByPriority[job.Priority] = make(map[uint]*Job)
	}
	s.JobsByPriority[job.Priority][job.ID] = job

	if s.JobsByTenant[job.TenantID] == nil {
		s.JobsByTenant[job.TenantID] = make(map[uint]*Job)
	}
	s.JobsByTenant[job.TenantID][job.ID] = job
}

func (s *MemoryStore) RemoveJobFromIndex(jobID uint) {
	if job, exists := s.Jobs[jobID]; exists {
		delete(s.JobsByStatus[job.Status], jobID)
		delete(s.JobsByPriority[job.Priority], jobID)
		delete(s.JobsByTenant[job.TenantID], jobID)
		delete(s.Jobs, jobID)
	}
}

func (s *MemoryStore) UpdateJobStatus(jobID uint, newStatus string) {
	if job, exists := s.Jobs[jobID]; exists {
		delete(s.JobsByStatus[job.Status], jobID)
		job.Status = newStatus
		if s.JobsByStatus[newStatus] == nil {
			s.JobsByStatus[newStatus] = make(map[uint]*Job)
		}
		s.JobsByStatus[newStatus][jobID] = job
	}
}

func (s *MemoryStore) UpdateJobPriority(jobID uint, newPriority int) {
	if job, exists := s.Jobs[jobID]; exists {
		delete(s.JobsByPriority[job.Priority], jobID)
		job.Priority = newPriority
		if s.JobsByPriority[newPriority] == nil {
			s.JobsByPriority[newPriority] = make(map[uint]*Job)
		}
		s.JobsByPriority[newPriority][jobID] = job
	}
}

func InitMemoryStore() *MemoryStore {
	db, _ := GetMemoryStore()
	return db
}

func GetDBStore(db interface{}, serviceName string) (*MemoryStore, error) {
	if db == nil {
		logger.WarnWithCtx(nil, "Service received nil database, using global memory store", "service", serviceName)
		return GetMemoryStore()
	}

	if ms, ok := db.(*MemoryStore); ok {
		return ms, nil
	}

	if _, ok := db.(*gorm.DB); ok {
		logger.ErrorWithCtx(nil, "Service received GORM database but expects MemoryStore - this may cause data inconsistency", nil,
			"service", serviceName,
			"database_type", "gorm.DB")
		return nil, fmt.Errorf("service %s requires MemoryStore but received gorm.DB", serviceName)
	}

	logger.ErrorWithCtx(nil, "Service received unknown database type", nil,
		"service", serviceName,
		"database_type", fmt.Sprintf("%T", db))
	return nil, fmt.Errorf("service %s received unknown database type: %T", serviceName, db)
}
