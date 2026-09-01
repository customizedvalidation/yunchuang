package config

import (
	"fmt"
	"metaclouds-backend/pkg/logger"
	"os"
	"strconv"
	"strings"

	"github.com/joho/godotenv"
)

type Config struct {
	ServerPort                   string
	ServerHost                   string
	Environment                  string
	AllowedOrigins               []string
	UseSQLite                    bool
	MemoryStoreEnabled           bool
	DatabaseHost                 string
	DatabasePort                 string
	DatabaseUser                 string
	DatabasePassword             string
	DatabaseName                 string
	DatabaseSSLMode              string
	RedisEnabled                 bool
	RedisHost                    string
	RedisPort                    string
	RedisPassword                string
	RedisDB                      int
	JWTSecret                    string
	JWTExpirationHours           int
	JWTRefreshExpirationHours    int
	PrometheusEnabled            bool
	PrometheusPort               string
	MonitoringEnabled            bool
	AlertEnabled                 bool
	MetricsCollectionInterval    int
	K8SEnabled                   bool
	K8SNamespace                 string
	K8SConfigPath                string
	K8SSimulationMode            bool
	SchedulerEnabled             bool
	SchedulerIntervalSeconds     int
	TracingEnabled               bool
	TracingServiceName           string
	JaegerEndpoint               string
	ConfigCenterEnabled          bool
	ConfigCenterEndpoints        string
	ConfigCenterPrefix           string
	RateLimitEnabled             bool
	RateLimitRequests            int
	RateLimitDurationSeconds     int
	CircuitBreakerEnabled        bool
	CircuitBreakerThreshold      int
	CircuitBreakerTimeoutSeconds int
	LogLevel                     string
	LogFormat                    string
	LogOutput                    string
	LogPath                      string
	MaxRequestBodySize           int64
	ReadTimeoutSeconds           int
	WriteTimeoutSeconds          int
	IdleTimeoutSeconds           int
	TrustedProxies               []string
	DefaultPageSize              int
	MaxPageSize                  int
	SlowRequestThresholdMs       int
	FeatureGPUAllocation         bool
	FeatureJobScheduler          bool
	FeatureMonitoring            bool
	FeatureSecurityPolicies      bool
	// AllowPublicRegistration 控制是否开放匿名自助注册。
	// 默认关闭：注册接口一旦对公网开放，攻击者可批量灌账号，
	// 在内存存储模式下更是直接的 OOM 入口。
	AllowPublicRegistration bool
}

func LoadConfig() (*Config, error) {
	if err := godotenv.Load(); err != nil {
		logger.WarnWithCtx(nil, "Using environment variables, .env file not found", "error", err)
	}

	useSQLite, _ := strconv.ParseBool(getEnv("USE_SQLITE", "true"))
	memoryStoreEnabled, _ := strconv.ParseBool(getEnv("MEMORY_STORE_ENABLED", "true"))
	redisEnabled, _ := strconv.ParseBool(getEnv("REDIS_ENABLED", "false"))
	redisDB, _ := strconv.Atoi(getEnv("REDIS_DB", "0"))
	jwtExpirationHours, _ := strconv.Atoi(getEnv("JWT_EXPIRATION_HOURS", "24"))
	jwtRefreshExpirationHours, _ := strconv.Atoi(getEnv("JWT_REFRESH_EXPIRATION_HOURS", "168"))
	prometheusEnabled, _ := strconv.ParseBool(getEnv("PROMETHEUS_ENABLED", "true"))
	monitoringEnabled, _ := strconv.ParseBool(getEnv("MONITORING_ENABLED", "true"))
	alertEnabled, _ := strconv.ParseBool(getEnv("ALERT_ENABLED", "true"))
	metricsCollectionInterval, _ := strconv.Atoi(getEnv("METRICS_COLLECTION_INTERVAL_SECONDS", "15"))
	k8sEnabled, _ := strconv.ParseBool(getEnv("K8S_ENABLED", "true"))
	k8sSimulationMode, _ := strconv.ParseBool(getEnv("K8S_SIMULATION_MODE", "true"))
	schedulerEnabled, _ := strconv.ParseBool(getEnv("SCHEDULER_ENABLED", "true"))
	schedulerIntervalSeconds, _ := strconv.Atoi(getEnv("SCHEDULER_INTERVAL_SECONDS", "10"))
	tracingEnabled, _ := strconv.ParseBool(getEnv("TRACING_ENABLED", "false"))
	configCenterEnabled, _ := strconv.ParseBool(getEnv("CONFIG_CENTER_ENABLED", "false"))
	rateLimitEnabled, _ := strconv.ParseBool(getEnv("RATE_LIMIT_ENABLED", "true"))
	rateLimitRequests, _ := strconv.Atoi(getEnv("RATE_LIMIT_REQUESTS", "100"))
	rateLimitDurationSeconds, _ := strconv.Atoi(getEnv("RATE_LIMIT_DURATION_SECONDS", "60"))
	circuitBreakerEnabled, _ := strconv.ParseBool(getEnv("CIRCUIT_BREAKER_ENABLED", "true"))
	circuitBreakerThreshold, _ := strconv.Atoi(getEnv("CIRCUIT_BREAKER_THRESHOLD", "10"))
	circuitBreakerTimeoutSeconds, _ := strconv.Atoi(getEnv("CIRCUIT_BREAKER_TIMEOUT_SECONDS", "30"))
	maxRequestBodySize, _ := strconv.ParseInt(getEnv("MAX_REQUEST_BODY_SIZE", "10485760"), 10, 64)
	readTimeoutSeconds, _ := strconv.Atoi(getEnv("READ_TIMEOUT_SECONDS", "30"))
	writeTimeoutSeconds, _ := strconv.Atoi(getEnv("WRITE_TIMEOUT_SECONDS", "30"))
	idleTimeoutSeconds, _ := strconv.Atoi(getEnv("IDLE_TIMEOUT_SECONDS", "60"))
	defaultPageSize, _ := strconv.Atoi(getEnv("DEFAULT_PAGE_SIZE", "10"))
	maxPageSize, _ := strconv.Atoi(getEnv("MAX_PAGE_SIZE", "100"))
	slowRequestThresholdMs, _ := strconv.Atoi(getEnv("SLOW_REQUEST_THRESHOLD_MS", "2000"))
	featureGPUAllocation, _ := strconv.ParseBool(getEnv("FEATURE_GPU_ALLOCATION", "true"))
	featureJobScheduler, _ := strconv.ParseBool(getEnv("FEATURE_JOB_SCHEDULER", "true"))
	featureMonitoring, _ := strconv.ParseBool(getEnv("FEATURE_MONITORING", "true"))
	featureSecurityPolicies, _ := strconv.ParseBool(getEnv("FEATURE_SECURITY_POLICIES", "true"))
	// 默认关闭开放注册，需显式通过 ALLOW_PUBLIC_REGISTRATION=true 开启。
	allowPublicRegistration, _ := strconv.ParseBool(getEnv("ALLOW_PUBLIC_REGISTRATION", "false"))

	allowedOrigins := parseAllowedOrigins(getEnv("ALLOWED_ORIGINS", ""))
	trustedProxies := parseList(getEnv("TRUSTED_PROXIES", ""))

	return &Config{
		ServerPort:                   getEnv("SERVER_PORT", "8000"),
		ServerHost:                   getEnv("SERVER_HOST", "0.0.0.0"),
		Environment:                  getEnv("SERVER_ENV", "development"),
		AllowedOrigins:               allowedOrigins,
		UseSQLite:                    useSQLite,
		MemoryStoreEnabled:           memoryStoreEnabled,
		DatabaseHost:                 getEnv("DATABASE_HOST", "localhost"),
		DatabasePort:                 getEnv("DATABASE_PORT", "5432"),
		DatabaseUser:                 getEnv("DATABASE_USER", "metaclouds"),
		DatabasePassword:             getEnv("DATABASE_PASSWORD", ""),
		DatabaseName:                 getEnv("DATABASE_NAME", "metaclouds"),
		DatabaseSSLMode:              getEnv("DATABASE_SSL_MODE", "disable"),
		RedisEnabled:                 redisEnabled,
		RedisHost:                    getEnv("REDIS_HOST", "localhost"),
		RedisPort:                    getEnv("REDIS_PORT", "6379"),
		RedisPassword:                getEnv("REDIS_PASSWORD", ""),
		RedisDB:                      redisDB,
		JWTSecret:                    getEnv("JWT_SECRET", ""),
		JWTExpirationHours:           jwtExpirationHours,
		JWTRefreshExpirationHours:    jwtRefreshExpirationHours,
		PrometheusEnabled:            prometheusEnabled,
		PrometheusPort:               getEnv("PROMETHEUS_PORT", "9090"),
		MonitoringEnabled:            monitoringEnabled,
		AlertEnabled:                 alertEnabled,
		MetricsCollectionInterval:    metricsCollectionInterval,
		K8SEnabled:                   k8sEnabled,
		K8SNamespace:                 getEnv("K8S_NAMESPACE", "metaclouds"),
		K8SConfigPath:                getEnv("K8S_CONFIG_PATH", "~/.kube/config"),
		K8SSimulationMode:            k8sSimulationMode,
		SchedulerEnabled:             schedulerEnabled,
		SchedulerIntervalSeconds:     schedulerIntervalSeconds,
		TracingEnabled:               tracingEnabled,
		TracingServiceName:           getEnv("TRACING_SERVICE_NAME", "metaclouds-backend"),
		JaegerEndpoint:               getEnv("JAEGER_ENDPOINT", "http://localhost:14268/api/traces"),
		ConfigCenterEnabled:          configCenterEnabled,
		ConfigCenterEndpoints:        getEnv("CONFIG_CENTER_ENDPOINTS", "localhost:2379"),
		ConfigCenterPrefix:           getEnv("CONFIG_CENTER_PREFIX", "/metaclouds/config/"),
		RateLimitEnabled:             rateLimitEnabled,
		RateLimitRequests:            rateLimitRequests,
		RateLimitDurationSeconds:     rateLimitDurationSeconds,
		CircuitBreakerEnabled:        circuitBreakerEnabled,
		CircuitBreakerThreshold:      circuitBreakerThreshold,
		CircuitBreakerTimeoutSeconds: circuitBreakerTimeoutSeconds,
		LogLevel:                     getEnv("LOG_LEVEL", "info"),
		LogFormat:                    getEnv("LOG_FORMAT", "json"),
		LogOutput:                    getEnv("LOG_OUTPUT", "console"),
		LogPath:                      getEnv("LOG_PATH", "/var/log/metaclouds/backend.log"),
		MaxRequestBodySize:           maxRequestBodySize,
		ReadTimeoutSeconds:           readTimeoutSeconds,
		WriteTimeoutSeconds:          writeTimeoutSeconds,
		IdleTimeoutSeconds:           idleTimeoutSeconds,
		TrustedProxies:               trustedProxies,
		DefaultPageSize:              defaultPageSize,
		MaxPageSize:                  maxPageSize,
		SlowRequestThresholdMs:       slowRequestThresholdMs,
		FeatureGPUAllocation:         featureGPUAllocation,
		FeatureJobScheduler:          featureJobScheduler,
		FeatureMonitoring:            featureMonitoring,
		FeatureSecurityPolicies:      featureSecurityPolicies,
		AllowPublicRegistration:      allowPublicRegistration,
	}, nil
}

func getEnv(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

func (c *Config) Validate() error {
	if c.ServerPort == "" {
		return fmt.Errorf("SERVER_PORT is required")
	}
	if c.JWTSecret == "" {
		return fmt.Errorf("JWT_SECRET is required")
	}
	if len(c.JWTSecret) < 32 {
		return fmt.Errorf("JWT_SECRET must be at least 32 characters long for security")
	}
	if c.JWTExpirationHours <= 0 {
		return fmt.Errorf("JWT_EXPIRATION_HOURS must be greater than 0")
	}
	// 仅当启用限流时才校验限流参数；关闭限流时 RateLimitRequests 默认 0 不应阻断启动。
	if c.RateLimitEnabled {
		if c.RateLimitRequests <= 0 {
			return fmt.Errorf("RATE_LIMIT_REQUESTS must be greater than 0 when RATE_LIMIT_ENABLED is true")
		}
		if c.RateLimitDurationSeconds <= 0 {
			return fmt.Errorf("RATE_LIMIT_DURATION_SECONDS must be greater than 0 when RATE_LIMIT_ENABLED is true")
		}
	}
	if c.Environment == "production" && c.MemoryStoreEnabled {
		return fmt.Errorf("MEMORY_STORE_ENABLED must be false in production environment")
	}
	// 生产环境禁止开放注册：账号必须由管理员或 IdP 开通，
	// 匿名注册在公网等同于把账号创建权交给攻击者。
	if c.Environment == "production" && c.AllowPublicRegistration {
		return fmt.Errorf("ALLOW_PUBLIC_REGISTRATION must be false in production environment")
	}
	if c.Environment == "production" && c.UseSQLite {
		return fmt.Errorf("USE_SQLITE must be false in production environment")
	}
	if c.DatabaseSSLMode == "disable" && c.Environment == "production" {
		return fmt.Errorf("DATABASE_SSL_MODE must not be 'disable' in production environment")
	}

	// CORS 与可信代理在生产环境必须显式声明。
	// 缺少 ALLOWED_ORIGINS 会让浏览器请求被全部拒绝（表现为整站不可用）；
	// 缺少 TRUSTED_PROXIES 则意味着信任报文里的 X-Forwarded-For，
	// 使基于 IP 的限流形同虚设。
	if c.Environment == "production" {
		if len(c.AllowedOrigins) == 0 {
			return fmt.Errorf("ALLOWED_ORIGINS must be set in production environment")
		}
		for _, origin := range c.AllowedOrigins {
			if origin == "*" {
				return fmt.Errorf("ALLOWED_ORIGINS must not contain '*' when credentials are allowed")
			}
		}
	}
	return nil
}

func (c *Config) GetDatabaseDSN() string {
	return fmt.Sprintf(
		"host=%s port=%s user=%s password=%s dbname=%s sslmode=%s",
		c.DatabaseHost, c.DatabasePort, c.DatabaseUser,
		c.DatabasePassword, c.DatabaseName, c.DatabaseSSLMode,
	)
}

func (c *Config) GetRedisAddr() string {
	return fmt.Sprintf("%s:%s", c.RedisHost, c.RedisPort)
}

func (c *Config) GetPrometheusURL() string {
	return fmt.Sprintf("http://%s:%s", "localhost", c.PrometheusPort)
}

func (c *Config) GetServerAddr() string {
	return fmt.Sprintf("%s:%s", c.ServerHost, c.ServerPort)
}

// parseList 把逗号分隔的配置值拆成切片，并去掉空白项。
func parseList(raw string) []string {
	items := make([]string, 0)
	for _, item := range strings.Split(raw, ",") {
		item = strings.TrimSpace(item)
		if item != "" {
			items = append(items, item)
		}
	}
	return items
}

// parseAllowedOrigins 解析 CORS 允许的来源列表。
// 不在此处做安全性判断，合法性校验统一放在 Validate 中。
func parseAllowedOrigins(originsStr string) []string {
	return parseList(originsStr)
}
