package main

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/gin-contrib/cors"
	"github.com/gin-gonic/gin"

	"metaclouds-backend/api"
	"metaclouds-backend/config"
	"metaclouds-backend/controllers"
	"metaclouds-backend/middlewares"
	"metaclouds-backend/models"
	"metaclouds-backend/pkg/logger"
	"metaclouds-backend/pkg/middleware"
	"metaclouds-backend/services"
)

func main() {
	if err := run(); err != nil {
		logger.ErrorWithCtx(context.Background(), "Application failed to start", err)
		os.Exit(1)
	}
}

// envOrDefault 读取环境变量，为空时回退到默认值。
// 配置加载完成之前 logger 尚不可用，这里直接读取环境。
func envOrDefault(key, fallback string) string {
	if v := os.Getenv(key); v != "" {
		return v
	}
	return fallback
}

func run() error {
	// 配置加载前先按环境变量建立一个可用的 logger，配置就绪后会重新初始化，
	// 避免把生产运行时的启动日志固定写成 development 级别。
	logger.InitGlobalLogger(envOrDefault("SERVER_ENV", "development"), envOrDefault("LOG_LEVEL", "info"))
	logger.InfoWithCtx(context.Background(), "Starting Metaclouds backend service...")

	logger.InfoWithCtx(context.Background(), "Loading configuration...")
	cfg, err := config.LoadConfig()
	if err != nil {
		return fmt.Errorf("failed to load config: %w", err)
	}

	if err := cfg.Validate(); err != nil {
		return fmt.Errorf("config validation failed: %w", err)
	}

	logger.InitGlobalLoggerWithConfig(logger.LoggerConfig{
		Env:       cfg.Environment,
		LogLevel:  cfg.LogLevel,
		LogOutput: cfg.LogOutput,
		LogPath:   cfg.LogPath,
	})
	logger.InfoWithCtx(context.Background(), "Configuration loaded: Environment=%s, Port=%s, MemoryStore=%v", cfg.Environment, cfg.ServerPort, cfg.MemoryStoreEnabled)

	logger.InfoWithCtx(context.Background(), "Initializing database...")
	db, err := models.InitDB(cfg)
	if err != nil {
		return fmt.Errorf("database init failed: %w", err)
	}

	var dbType string
	if cfg.MemoryStoreEnabled {
		dbType = "in-memory store"
	} else if cfg.UseSQLite {
		dbType = "SQLite"
	} else {
		dbType = "PostgreSQL"
	}
	logger.InfoWithCtx(context.Background(), "Database initialized", "database_type", dbType, "database_host", cfg.DatabaseHost)

	// /health 依赖探针：只有在数据层可正常加锁访问时才算健康。
	services.RegisterHealthCheck(func() error {
		store, err := models.GetDBStore(db, "health")
		if err != nil {
			return err
		}
		store.Mu.RLock()
		defer store.Mu.RUnlock()
		return nil
	})

	logger.InfoWithCtx(context.Background(), "Initializing Redis...")
	var redisClient interface{}
	redisClient, err = models.InitRedis(cfg)
	if err != nil {
		logger.WarnWithCtx(context.Background(), "Redis init failed: %v, continuing without Redis", err)
		redisClient = nil
	} else {
		logger.InfoWithCtx(context.Background(), "Redis initialized")
	}

	logger.InfoWithCtx(context.Background(), "Initializing services...")
	authService := services.NewAuthService(db, redisClient, cfg)
	if authService == nil {
		return fmt.Errorf("failed to initialize AuthService")
	}

	clusterService := services.NewClusterService(db, cfg)
	if clusterService == nil {
		return fmt.Errorf("failed to initialize ClusterService")
	}

	resourceService := services.NewResourceService(db, cfg)
	if resourceService == nil {
		return fmt.Errorf("failed to initialize ResourceService")
	}

	monitoringService := services.NewMonitoringService(db, cfg)
	if monitoringService == nil {
		return fmt.Errorf("failed to initialize MonitoringService")
	}

	tenantService := services.NewTenantService(db, cfg)
	if tenantService == nil {
		return fmt.Errorf("failed to initialize TenantService")
	}

	accelerationService := services.NewAccelerationService(db, cfg)
	if accelerationService == nil {
		return fmt.Errorf("failed to initialize AccelerationService")
	}

	securityService := services.NewSecurityService(db, cfg)
	if securityService == nil {
		return fmt.Errorf("failed to initialize SecurityService")
	}

	k8sService := services.NewK8SService(db, cfg)
	if k8sService == nil {
		return fmt.Errorf("failed to initialize K8SService")
	}

	jobService := services.NewJobService(db, cfg, k8sService)
	if jobService == nil {
		return fmt.Errorf("failed to initialize JobService")
	}

	logger.InfoWithCtx(context.Background(), "Services initialized")

	authController := controllers.NewAuthController(authService)
	clusterController := controllers.NewClusterController(clusterService)
	resourceController := controllers.NewResourceController(resourceService)
	jobController := controllers.NewJobController(jobService)
	monitoringController := controllers.NewMonitoringController(monitoringService)
	tenantController := controllers.NewTenantController(tenantService)
	accelerationController := controllers.NewAccelerationController(accelerationService)
	securityController := controllers.NewSecurityController(securityService)
	k8sController := controllers.NewK8SController(k8sService)

	logger.InfoWithCtx(context.Background(), "Initializing metrics collector...")
	metricsCollector := services.NewMetricsCollector(monitoringService, jobService)
	go metricsCollector.StartCollecting(time.Duration(cfg.MetricsCollectionInterval) * time.Second)
	// 采集协程必须随服务退出而停止，否则会一直持有 ticker 泄漏下去。
	defer metricsCollector.Stop()

	logger.InfoWithCtx(context.Background(), "Initializing scheduler...")
	scheduler := services.NewScheduler(jobService, k8sService)
	if cfg.Environment != "production" {
		// 示例调度会周期性地把作业提交到 K8s。生产环境若默认启用，
		// 会在无人察觉的情况下持续占用真实算力。
		scheduler.AddDefaultSchedules()
	}
	go scheduler.Start()
	defer scheduler.Stop()

	if cfg.Environment == "production" {
		gin.SetMode(gin.ReleaseMode)
	}

	r := gin.New()

	// 客户端 IP 用于限流与审计。默认的 gin 会信任报文中的 X-Forwarded-For，
	// 攻击者只要伪造该头部就能绕过限流，因此这里显式声明可信代理。
	if err := r.SetTrustedProxies(cfg.TrustedProxies); err != nil {
		return fmt.Errorf("failed to configure trusted proxies: %w", err)
	}

	// 请求体大小限制必须排在任何会读取 body 的中间件之前。
	// 原先它注册在 SecurityFilter 之后，而 SecurityFilter 会完整读取 body，
	// 导致该限制实际上从未生效，超大请求体可直接打满内存。
	r.Use(func(c *gin.Context) {
		c.Request.Body = http.MaxBytesReader(c.Writer, c.Request.Body, cfg.MaxRequestBodySize)
		c.Next()
	})

	r.Use(middlewares.PanicRecovery())
	r.Use(middlewares.RequestID())
	r.Use(middlewares.SecurityHeaders())
	r.Use(middlewares.SecurityFilter())
	r.Use(middlewares.RequestLogger())
	r.Use(middlewares.ErrorHandler())
	r.Use(middlewares.DefaultTimingMiddleware)
	r.Use(logger.GinMiddleware(logger.Global))

	if cfg.RateLimitEnabled {
		rateLimiter := middleware.NewSlidingWindowLimiter(cfg.RateLimitRequests, time.Duration(cfg.RateLimitDurationSeconds)*time.Second)
		r.Use(rateLimiter.Middleware())
	}

	if cfg.CircuitBreakerEnabled {
		circuitBreaker := middleware.NewCircuitBreaker(cfg.CircuitBreakerThreshold, 5, time.Duration(cfg.CircuitBreakerTimeoutSeconds)*time.Second)
		r.Use(circuitBreaker.Middleware())
	}

	corsConfig := cors.Config{
		AllowMethods:     []string{"GET", "POST", "PUT", "DELETE", "OPTIONS", "PATCH"},
		AllowHeaders:     []string{"Origin", "Content-Type", "Authorization", "X-Request-ID", "Accept"},
		ExposeHeaders:    []string{"Content-Length", "X-Request-ID", "Authorization"},
		AllowCredentials: true,
		MaxAge:           12 * time.Hour,
	}

	if cfg.Environment == "production" {
		corsConfig.AllowOrigins = cfg.AllowedOrigins
	} else {
		corsConfig.AllowOrigins = []string{"http://localhost:3000", "http://localhost:8080", "http://127.0.0.1:3000", "http://127.0.0.1:8080"}
	}

	r.Use(cors.New(corsConfig))

	r.Static("/swagger", "./api/swagger-ui")

	api.RegisterRoutes(r, cfg, authController, clusterController, resourceController, jobController, monitoringController, tenantController, accelerationController, securityController, k8sController)

	addr := fmt.Sprintf("%s:%s", cfg.ServerHost, cfg.ServerPort)

	srv := &http.Server{
		Addr:         addr,
		Handler:      r,
		ReadTimeout:  time.Duration(cfg.ReadTimeoutSeconds) * time.Second,
		WriteTimeout: time.Duration(cfg.WriteTimeoutSeconds) * time.Second,
		IdleTimeout:  time.Duration(cfg.IdleTimeoutSeconds) * time.Second,
	}

	serverErr := make(chan error, 1)
	go func() {
		logger.InfoWithCtx(context.Background(), "Server starting on %s", addr)
		logger.InfoWithCtx(context.Background(), "========================================")
		logger.InfoWithCtx(context.Background(), "Metaclouds Backend is ready!")
		logger.InfoWithCtx(context.Background(), "- Health Check: http://%s/health", addr)
		logger.InfoWithCtx(context.Background(), "- Metrics: http://%s/metrics", addr)
		logger.InfoWithCtx(context.Background(), "- Swagger UI: http://%s/swagger/index.html", addr)
		logger.InfoWithCtx(context.Background(), "- Login API: POST http://%s/api/v1/auth/login", addr)
		logger.InfoWithCtx(context.Background(), "- Frontend: http://localhost:3000")
		// 注意：此处刻意不输出默认管理员口令。凭据属于敏感信息，
		// 一旦写入日志就会被日志采集系统长期留存并扩大知悉范围。
		logger.InfoWithCtx(context.Background(), "========================================")
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			serverErr <- err
		}
	}()

	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)

	select {
	case err := <-serverErr:
		return fmt.Errorf("failed to start server: %w", err)
	case <-quit:
		logger.InfoWithCtx(context.Background(), "Shutting down server...")

		ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
		defer cancel()
		if err := srv.Shutdown(ctx); err != nil {
			return fmt.Errorf("server forced to shutdown: %w", err)
		}

		logger.InfoWithCtx(context.Background(), "Server exiting gracefully")
		return nil
	}
}
