package config

import (
	"os"
	"strings"
	"testing"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// clearEnv 清除所有与配置相关的环境变量，确保测试隔离。
func clearEnv(t *testing.T) {
	t.Helper()
	keys := []string{
		"SERVER_PORT", "SERVER_HOST", "SERVER_ENV", "ALLOWED_ORIGINS",
		"USE_SQLITE", "MEMORY_STORE_ENABLED", "DATABASE_HOST", "DATABASE_PORT",
		"DATABASE_USER", "DATABASE_PASSWORD", "DATABASE_NAME", "DATABASE_SSL_MODE",
		"REDIS_ENABLED", "REDIS_HOST", "REDIS_PORT", "REDIS_PASSWORD", "REDIS_DB",
		"JWT_SECRET", "JWT_EXPIRATION_HOURS", "JWT_REFRESH_EXPIRATION_HOURS",
		"PROMETHEUS_ENABLED", "PROMETHEUS_PORT", "MONITORING_ENABLED", "ALERT_ENABLED",
		"METRICS_COLLECTION_INTERVAL_SECONDS", "K8S_ENABLED", "K8S_NAMESPACE",
		"K8S_CONFIG_PATH", "K8S_SIMULATION_MODE", "SCHEDULER_ENABLED",
		"SCHEDULER_INTERVAL_SECONDS", "TRACING_ENABLED", "TRACING_SERVICE_NAME",
		"JAEGER_ENDPOINT", "CONFIG_CENTER_ENABLED", "CONFIG_CENTER_ENDPOINTS",
		"CONFIG_CENTER_PREFIX", "RATE_LIMIT_ENABLED", "RATE_LIMIT_REQUESTS",
		"RATE_LIMIT_DURATION_SECONDS", "CIRCUIT_BREAKER_ENABLED",
		"CIRCUIT_BREAKER_THRESHOLD", "CIRCUIT_BREAKER_TIMEOUT_SECONDS",
		"LOG_LEVEL", "LOG_FORMAT", "LOG_OUTPUT", "LOG_PATH",
		"MAX_REQUEST_BODY_SIZE", "READ_TIMEOUT_SECONDS", "WRITE_TIMEOUT_SECONDS",
		"IDLE_TIMEOUT_SECONDS", "TRUSTED_PROXIES", "DEFAULT_PAGE_SIZE",
		"MAX_PAGE_SIZE", "SLOW_REQUEST_THRESHOLD_MS", "FEATURE_GPU_ALLOCATION",
		"FEATURE_JOB_SCHEDULER", "FEATURE_MONITORING", "FEATURE_SECURITY_POLICIES",
		"ALLOW_PUBLIC_REGISTRATION", "COOKIE_SAME_SITE",
	}
	for _, k := range keys {
		os.Unsetenv(k)
	}
}

// validProductionConfig 返回一个生产环境下可通过校验的最小配置。
func validProductionConfig() *Config {
	return &Config{
		ServerPort:             "8000",
		JWTSecret:              strings.Repeat("s", 32),
		JWTExpirationHours:     24,
		Environment:            "production",
		UseSQLite:              false,
		MemoryStoreEnabled:     false,
		AllowPublicRegistration: false,
		DatabaseSSLMode:        "require",
		AllowedOrigins:         []string{"https://app.example.com"},
		RateLimitEnabled:       true,
		RateLimitRequests:      100,
		RateLimitDurationSeconds: 60,
	}
}

func TestLoadConfig_Defaults(t *testing.T) {
	clearEnv(t)

	cfg, err := LoadConfig()
	require.NoError(t, err)

	// 验证各项默认值
	assert.Equal(t, "8000", cfg.ServerPort)
	assert.Equal(t, "0.0.0.0", cfg.ServerHost)
	assert.Equal(t, "development", cfg.Environment)
	assert.True(t, cfg.UseSQLite)
	assert.True(t, cfg.MemoryStoreEnabled)
	assert.Equal(t, "localhost", cfg.DatabaseHost)
	assert.Equal(t, "5432", cfg.DatabasePort)
	assert.Equal(t, "metaclouds", cfg.DatabaseUser)
	assert.Equal(t, "metaclouds", cfg.DatabaseName)
	assert.Equal(t, "disable", cfg.DatabaseSSLMode)
	assert.False(t, cfg.RedisEnabled)
	assert.Equal(t, "localhost", cfg.RedisHost)
	assert.Equal(t, "6379", cfg.RedisPort)
	assert.Equal(t, 0, cfg.RedisDB)
	assert.Equal(t, "", cfg.JWTSecret)
	assert.Equal(t, 24, cfg.JWTExpirationHours)
	assert.Equal(t, 168, cfg.JWTRefreshExpirationHours)
	assert.True(t, cfg.PrometheusEnabled)
	assert.Equal(t, "9090", cfg.PrometheusPort)
	assert.True(t, cfg.MonitoringEnabled)
	assert.True(t, cfg.AlertEnabled)
	assert.Equal(t, 15, cfg.MetricsCollectionInterval)
	assert.True(t, cfg.K8SEnabled)
	assert.Equal(t, "metaclouds", cfg.K8SNamespace)
	assert.True(t, cfg.K8SSimulationMode)
	assert.True(t, cfg.SchedulerEnabled)
	assert.Equal(t, 10, cfg.SchedulerIntervalSeconds)
	assert.False(t, cfg.TracingEnabled)
	assert.Equal(t, "metaclouds-backend", cfg.TracingServiceName)
	assert.False(t, cfg.ConfigCenterEnabled)
	assert.True(t, cfg.RateLimitEnabled)
	assert.Equal(t, 100, cfg.RateLimitRequests)
	assert.Equal(t, 60, cfg.RateLimitDurationSeconds)
	assert.True(t, cfg.CircuitBreakerEnabled)
	assert.Equal(t, 10, cfg.CircuitBreakerThreshold)
	assert.Equal(t, 30, cfg.CircuitBreakerTimeoutSeconds)
	assert.Equal(t, "info", cfg.LogLevel)
	assert.Equal(t, "json", cfg.LogFormat)
	assert.Equal(t, "console", cfg.LogOutput)
	assert.Equal(t, int64(10485760), cfg.MaxRequestBodySize)
	assert.Equal(t, 30, cfg.ReadTimeoutSeconds)
	assert.Equal(t, 30, cfg.WriteTimeoutSeconds)
	assert.Equal(t, 60, cfg.IdleTimeoutSeconds)
	assert.Equal(t, 10, cfg.DefaultPageSize)
	assert.Equal(t, 100, cfg.MaxPageSize)
	assert.Equal(t, 2000, cfg.SlowRequestThresholdMs)
	assert.True(t, cfg.FeatureGPUAllocation)
	assert.True(t, cfg.FeatureJobScheduler)
	assert.True(t, cfg.FeatureMonitoring)
	assert.True(t, cfg.FeatureSecurityPolicies)
	assert.False(t, cfg.AllowPublicRegistration)
	assert.Equal(t, "lax", cfg.CookieSameSite)
	assert.Empty(t, cfg.AllowedOrigins)
	assert.Empty(t, cfg.TrustedProxies)
}

func TestLoadConfig_EnvironmentOverrides(t *testing.T) {
	clearEnv(t)

	os.Setenv("SERVER_PORT", "9090")
	os.Setenv("SERVER_HOST", "127.0.0.1")
	os.Setenv("SERVER_ENV", "staging")
	os.Setenv("JWT_SECRET", "my-secret-key-at-least-32-chars-long!!")
	os.Setenv("JWT_EXPIRATION_HOURS", "12")
	os.Setenv("USE_SQLITE", "false")
	os.Setenv("MEMORY_STORE_ENABLED", "false")
	os.Setenv("DATABASE_HOST", "db.example.com")
	os.Setenv("DATABASE_PORT", "5433")
	os.Setenv("DATABASE_SSL_MODE", "require")
	os.Setenv("ALLOWED_ORIGINS", "https://a.com,https://b.com")
	os.Setenv("TRUSTED_PROXIES", "10.0.0.1,10.0.0.2")
	os.Setenv("RATE_LIMIT_ENABLED", "false")
	os.Setenv("LOG_LEVEL", "debug")
	os.Setenv("COOKIE_SAME_SITE", "strict")
	os.Setenv("ALLOW_PUBLIC_REGISTRATION", "true")

	cfg, err := LoadConfig()
	require.NoError(t, err)

	assert.Equal(t, "9090", cfg.ServerPort)
	assert.Equal(t, "127.0.0.1", cfg.ServerHost)
	assert.Equal(t, "staging", cfg.Environment)
	assert.Equal(t, "my-secret-key-at-least-32-chars-long!!", cfg.JWTSecret)
	assert.Equal(t, 12, cfg.JWTExpirationHours)
	assert.False(t, cfg.UseSQLite)
	assert.False(t, cfg.MemoryStoreEnabled)
	assert.Equal(t, "db.example.com", cfg.DatabaseHost)
	assert.Equal(t, "5433", cfg.DatabasePort)
	assert.Equal(t, "require", cfg.DatabaseSSLMode)
	assert.Equal(t, []string{"https://a.com", "https://b.com"}, cfg.AllowedOrigins)
	assert.Equal(t, []string{"10.0.0.1", "10.0.0.2"}, cfg.TrustedProxies)
	assert.False(t, cfg.RateLimitEnabled)
	assert.Equal(t, "debug", cfg.LogLevel)
	assert.Equal(t, "strict", cfg.CookieSameSite)
	assert.True(t, cfg.AllowPublicRegistration)
}

func TestValidate_ProductionValid(t *testing.T) {
	cfg := validProductionConfig()
	assert.NoError(t, cfg.Validate())
}

func TestValidate_JWTSecretTooShort(t *testing.T) {
	cfg := validProductionConfig()
	cfg.JWTSecret = "short"
	err := cfg.Validate()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "JWT_SECRET must be at least 32 characters")
}

func TestValidate_JWTSecretEmpty(t *testing.T) {
	cfg := validProductionConfig()
	cfg.JWTSecret = ""
	err := cfg.Validate()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "JWT_SECRET is required")
}

func TestValidate_JWTExpirationInvalid(t *testing.T) {
	cfg := validProductionConfig()
	cfg.JWTExpirationHours = 0
	err := cfg.Validate()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "JWT_EXPIRATION_HOURS must be greater than 0")
}

func TestValidate_ServerPortEmpty(t *testing.T) {
	cfg := validProductionConfig()
	cfg.ServerPort = ""
	err := cfg.Validate()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "SERVER_PORT is required")
}

func TestValidate_ProductionNoSQLite(t *testing.T) {
	cfg := validProductionConfig()
	cfg.UseSQLite = true
	err := cfg.Validate()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "USE_SQLITE must be false in production")
}

func TestValidate_ProductionNoMemoryStore(t *testing.T) {
	cfg := validProductionConfig()
	cfg.MemoryStoreEnabled = true
	err := cfg.Validate()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "MEMORY_STORE_ENABLED must be false in production")
}

func TestValidate_ProductionNoPublicRegistration(t *testing.T) {
	cfg := validProductionConfig()
	cfg.AllowPublicRegistration = true
	err := cfg.Validate()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "ALLOW_PUBLIC_REGISTRATION must be false in production")
}

func TestValidate_ProductionSSLNotDisable(t *testing.T) {
	cfg := validProductionConfig()
	cfg.DatabaseSSLMode = "disable"
	err := cfg.Validate()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "DATABASE_SSL_MODE must not be 'disable' in production")
}

func TestValidate_ProductionAllowedOriginsRequired(t *testing.T) {
	cfg := validProductionConfig()
	cfg.AllowedOrigins = []string{}
	err := cfg.Validate()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "ALLOWED_ORIGINS must be set in production")
}

func TestValidate_ProductionAllowedOriginsNoWildcard(t *testing.T) {
	cfg := validProductionConfig()
	cfg.AllowedOrigins = []string{"*"}
	err := cfg.Validate()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "ALLOWED_ORIGINS must not contain '*'")
}

func TestValidate_RateLimitDisabledSkipsCheck(t *testing.T) {
	// 关闭限流时，RateLimitRequests=0 不应报错
	cfg := validProductionConfig()
	cfg.RateLimitEnabled = false
	cfg.RateLimitRequests = 0
	cfg.RateLimitDurationSeconds = 0
	assert.NoError(t, cfg.Validate())
}

func TestValidate_RateLimitEnabledRequiresPositive(t *testing.T) {
	cfg := validProductionConfig()
	cfg.RateLimitEnabled = true
	cfg.RateLimitRequests = 0
	err := cfg.Validate()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "RATE_LIMIT_REQUESTS must be greater than 0")

	cfg.RateLimitRequests = 100
	cfg.RateLimitDurationSeconds = 0
	err = cfg.Validate()
	require.Error(t, err)
	assert.Contains(t, err.Error(), "RATE_LIMIT_DURATION_SECONDS must be greater than 0")
}

func TestValidate_DevelopmentAllowsSQLite(t *testing.T) {
	cfg := &Config{
		ServerPort:         "8000",
		JWTSecret:          strings.Repeat("s", 32),
		JWTExpirationHours: 24,
		Environment:        "development",
		UseSQLite:          true,
		MemoryStoreEnabled: true,
		DatabaseSSLMode:    "disable",
		RateLimitEnabled:   false,
	}
	assert.NoError(t, cfg.Validate())
}

func TestGetDatabaseDSN(t *testing.T) {
	cfg := &Config{
		DatabaseHost:     "localhost",
		DatabasePort:     "5432",
		DatabaseUser:     "user",
		DatabasePassword: "pass",
		DatabaseName:     "mydb",
		DatabaseSSLMode:  "require",
	}
	dsn := cfg.GetDatabaseDSN()
	assert.Contains(t, dsn, "host=localhost")
	assert.Contains(t, dsn, "port=5432")
	assert.Contains(t, dsn, "user=user")
	assert.Contains(t, dsn, "password=pass")
	assert.Contains(t, dsn, "dbname=mydb")
	assert.Contains(t, dsn, "sslmode=require")
}

func TestGetRedisAddr(t *testing.T) {
	cfg := &Config{RedisHost: "redis.local", RedisPort: "6380"}
	assert.Equal(t, "redis.local:6380", cfg.GetRedisAddr())
}

func TestGetPrometheusURL(t *testing.T) {
	cfg := &Config{PrometheusPort: "9091"}
	url := cfg.GetPrometheusURL()
	assert.Equal(t, "http://localhost:9091", url)
}

func TestGetServerAddr(t *testing.T) {
	cfg := &Config{ServerHost: "0.0.0.0", ServerPort: "8080"}
	assert.Equal(t, "0.0.0.0:8080", cfg.GetServerAddr())
}

func TestParseList(t *testing.T) {
	tests := []struct {
		name     string
		input    string
		expected []string
	}{
		{"空字符串", "", []string{}},
		{"单个值", "a", []string{"a"}},
		{"多个值", "a,b,c", []string{"a", "b", "c"}},
		{"带空格", "a, b , c", []string{"a", "b", "c"}},
		{"空项被过滤", "a,,b,", []string{"a", "b"}},
		{"仅逗号", ",,,", []string{}},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			result := parseList(tc.input)
			assert.Equal(t, tc.expected, result)
		})
	}
}

func TestParseAllowedOrigins(t *testing.T) {
	result := parseAllowedOrigins("https://a.com, https://b.com")
	assert.Equal(t, []string{"https://a.com", "https://b.com"}, result)

	empty := parseAllowedOrigins("")
	assert.Empty(t, empty)
}

func TestGetEnv(t *testing.T) {
	os.Setenv("TEST_CONFIG_ENV_VAR", "custom-value")
	assert.Equal(t, "custom-value", getEnv("TEST_CONFIG_ENV_VAR", "default"))
	os.Unsetenv("TEST_CONFIG_ENV_VAR")

	assert.Equal(t, "default", getEnv("TEST_CONFIG_ENV_VAR", "default"))
}
