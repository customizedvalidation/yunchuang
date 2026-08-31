package api

import (
	"errors"
	"net/http"
	"strconv"
	"text/template"
	"time"

	"github.com/gin-contrib/cors"
	"github.com/gin-gonic/gin"

	"metaclouds-backend/config"
	"metaclouds-backend/controllers"
	"metaclouds-backend/middlewares"
	"metaclouds-backend/pkg/response"
	"metaclouds-backend/services"
)

// maxTestDelayMs 是 /api/v1/test/slow 允许的最大人为延迟。
const maxTestDelayMs = 5000

var indexTemplate = template.Must(template.New("index").Parse(`
<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Metaclouds - AI Computing Power Scheduling</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }
        .container {
            background: white;
            border-radius: 16px;
            box-shadow: 0 20px 60px rgba(0, 0, 0, 0.3);
            padding: 48px;
            width: 100%;
            max-width: 420px;
            animation: fadeInUp 0.4s ease;
        }
        @keyframes fadeInUp {
            from {
                opacity: 0;
                transform: translateY(20px);
            }
            to {
                opacity: 1;
                transform: translateY(0);
            }
        }
        .logo {
            text-align: center;
            margin-bottom: 32px;
        }
        .logo h1 {
            font-size: 28px;
            color: #2c3e50;
            margin-bottom: 8px;
        }
        .logo p {
            color: #7f8c8d;
            font-size: 14px;
        }
        .form-group {
            margin-bottom: 20px;
        }
        .form-group label {
            display: block;
            margin-bottom: 8px;
            color: #2c3e50;
            font-weight: 500;
            font-size: 14px;
        }
        .form-group input {
            width: 100%;
            padding: 12px 16px;
            border: 2px solid #ecf0f1;
            border-radius: 10px;
            font-size: 16px;
            transition: all 0.3s ease;
            background: #fafafa;
        }
        .form-group input:focus {
            outline: none;
            border-color: #667eea;
            background: white;
            box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
        }
        .form-group input::placeholder {
            color: #bdc3c7;
        }
        .btn {
            width: 100%;
            padding: 14px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border: none;
            border-radius: 10px;
            font-size: 16px;
            font-weight: 600;
            cursor: pointer;
            transition: all 0.3s ease;
            margin-top: 24px;
        }
        .btn:hover {
            transform: translateY(-2px);
            box-shadow: 0 10px 30px rgba(102, 126, 234, 0.4);
        }
        .btn:active {
            transform: translateY(0);
        }
        .btn:disabled {
            opacity: 0.7;
            cursor: not-allowed;
            transform: none;
        }
        .error-message {
            background: #ffebee;
            color: #c62828;
            padding: 12px 16px;
            border-radius: 8px;
            margin-bottom: 16px;
            font-size: 14px;
            display: none;
            border-left: 4px solid #c62828;
        }
        .success-message {
            background: #e8f5e9;
            color: #2e7d32;
            padding: 12px 16px;
            border-radius: 8px;
            margin-bottom: 16px;
            font-size: 14px;
            display: none;
            border-left: 4px solid #2e7d32;
        }
        .loading {
            display: inline-block;
            width: 20px;
            height: 20px;
            border: 2px solid #fff;
            border-radius: 50%;
            border-top-color: transparent;
            animation: spin 0.8s linear infinite;
        }
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
        .footer {
            text-align: center;
            margin-top: 24px;
            padding-top: 24px;
            border-top: 1px solid #ecf0f1;
        }
        .footer a {
            color: #667eea;
            text-decoration: none;
            font-size: 14px;
        }
        .footer a:hover {
            text-decoration: underline;
        }
        .demo-info {
            background: #fff3e0;
            border-left: 4px solid #ff9800;
            padding: 12px 16px;
            border-radius: 0 8px 8px 0;
            margin-bottom: 20px;
            font-size: 13px;
            color: #e65100;
        }
        .demo-info strong {
            display: block;
            margin-bottom: 4px;
        }
        .dashboard {
            display: none;
        }
        .dashboard h2 {
            color: #2c3e50;
            margin-bottom: 20px;
        }
        .dashboard .card {
            background: #f8f9fa;
            padding: 16px;
            border-radius: 10px;
            margin-bottom: 12px;
        }
        .dashboard .card h3 {
            font-size: 16px;
            color: #2c3e50;
            margin-bottom: 8px;
        }
        .dashboard .card p {
            color: #7f8c8d;
            font-size: 14px;
        }
        .dashboard .btn-logout {
            background: #e74c3c;
            margin-top: 16px;
        }
        .dashboard .btn-logout:hover {
            box-shadow: 0 10px 30px rgba(231, 76, 60, 0.4);
        }
        .metrics {
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: 12px;
            margin-bottom: 20px;
        }
        .metric {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            padding: 16px;
            border-radius: 10px;
            color: white;
            text-align: center;
        }
        .metric .value {
            font-size: 28px;
            font-weight: 700;
        }
        .metric .label {
            font-size: 12px;
            opacity: 0.9;
            margin-top: 4px;
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="logo">
            <h1>Metaclouds</h1>
            <p>AI Computing Power Scheduling Platform</p>
        </div>

        <div class="demo-info">
            <strong>演示账户</strong>
            默认管理员账号为 admin，初始口令由服务端的 DEFAULT_ADMIN_PASSWORD
            环境变量决定，请查阅部署文档或询问系统管理员。
        </div>

        <div class="error-message" id="errorMessage"></div>
        <div class="success-message" id="successMessage"></div>

        <div class="login-form" id="loginForm">
            <div class="form-group">
                <label for="username">用户名</label>
                <input type="text" id="username" placeholder="请输入用户名" autocomplete="username">
            </div>
            <div class="form-group">
                <label for="password">密码</label>
                <input type="password" id="password" placeholder="请输入密码" autocomplete="current-password">
            </div>
            <button class="btn" id="loginBtn" onclick="login()">
                <span id="btnText">登录</span>
            </button>
        </div>

        <div class="dashboard" id="dashboard">
            <h2>欢迎回来!</h2>
            <div class="metrics">
                <div class="metric">
                    <div class="value">3</div>
                    <div class="label">集群数</div>
                </div>
                <div class="metric">
                    <div class="value">12</div>
                    <div class="label">GPU资源</div>
                </div>
                <div class="metric">
                    <div class="value">5</div>
                    <div class="label">运行任务</div>
                </div>
                <div class="metric">
                    <div class="value">24</div>
                    <div class="label">完成任务</div>
                </div>
            </div>
            <div class="card">
                <h3>API 端点</h3>
                <p><strong>认证:</strong> POST /api/v1/auth/login</p>
                <p><strong>集群:</strong> GET /api/v1/clusters</p>
                <p><strong>任务:</strong> GET /api/v1/jobs</p>
                <p><strong>监控:</strong> GET /api/v1/monitoring/metrics</p>
            </div>
            <div class="card">
                <h3>服务状态</h3>
                <p>✅ 健康检查: /health</p>
                <p>📊 监控指标: /metrics</p>
                <p>📝 API文档: /api/docs</p>
            </div>
            <button class="btn btn-logout" onclick="logout()">退出登录</button>
        </div>

        <div class="footer">
            <a href="/api/docs" target="_blank">查看 API 文档</a>
        </div>
    </div>

    <script>
        function login() {
            const username = document.getElementById('username').value;
            const password = document.getElementById('password').value;
            const btn = document.getElementById('loginBtn');
            const btnText = document.getElementById('btnText');
            const errorMessage = document.getElementById('errorMessage');
            const successMessage = document.getElementById('successMessage');

            errorMessage.style.display = 'none';
            successMessage.style.display = 'none';

            if (!username || !password) {
                errorMessage.textContent = '请输入用户名和密码';
                errorMessage.style.display = 'block';
                return;
            }

            btn.disabled = true;
            btnText.innerHTML = '<span class="loading"></span> 登录中...';

            fetch('/api/v1/auth/login', {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify({ username, password }),
            })
            .then(response => response.json())
            .then(data => {
                btn.disabled = false;
                btnText.textContent = '登录';

                if (data.success && data.data && data.data.token) {
                    localStorage.setItem('token', data.data.token);
                    localStorage.setItem('user', JSON.stringify(data.data.user));
                    successMessage.textContent = '登录成功! 正在跳转...';
                    successMessage.style.display = 'block';
                    
                    setTimeout(() => {
                        document.getElementById('loginForm').style.display = 'none';
                        document.getElementById('dashboard').style.display = 'block';
                        successMessage.style.display = 'none';
                    }, 1000);
                } else {
                    errorMessage.textContent = data.message || data.error || '登录失败';
                    errorMessage.style.display = 'block';
                }
            })
            .catch(error => {
                btn.disabled = false;
                btnText.textContent = '登录';
                errorMessage.textContent = '网络错误，请重试';
                errorMessage.style.display = 'block';
            });
        }

        function logout() {
            localStorage.removeItem('token');
            localStorage.removeItem('user');
            document.getElementById('dashboard').style.display = 'none';
            document.getElementById('loginForm').style.display = 'block';
            document.getElementById('username').value = '';
            document.getElementById('password').value = '';
        }

        document.getElementById('password').addEventListener('keyup', function(e) {
            if (e.key === 'Enter') {
                login();
            }
        });

        document.addEventListener('DOMContentLoaded', function() {
            const token = localStorage.getItem('token');
            if (token) {
                document.getElementById('loginForm').style.display = 'none';
                document.getElementById('dashboard').style.display = 'block';
            }
        });
    </script>
</body>
</html>
`))

func SetupRouter(cfg *config.Config) *gin.Engine {
	if cfg.Environment == "production" {
		gin.SetMode(gin.ReleaseMode)
	}

	r := gin.New()
	r.Use(gin.Recovery())
	r.Use(middlewares.RequestID())
	r.Use(middlewares.SecurityHeaders())
	r.Use(middlewares.ErrorHandler())
	r.Use(middlewares.DefaultTimingMiddleware)

	r.Use(cors.New(cors.Config{
		AllowOrigins:     []string{"http://localhost:3000", "http://localhost:8080", "http://localhost:8000"},
		AllowMethods:     []string{"GET", "POST", "PUT", "DELETE", "OPTIONS"},
		AllowHeaders:     []string{"Origin", "Content-Type", "Authorization", "X-Request-ID", "X-Trace-ID"},
		ExposeHeaders:    []string{"Content-Length", "X-Request-ID", "X-Trace-ID"},
		AllowCredentials: true,
		MaxAge:           12 * time.Hour,
	}))

	return r
}

func RegisterRoutes(r *gin.Engine,
	cfg *config.Config,
	authController *controllers.AuthController,
	clusterController *controllers.ClusterController,
	resourceController *controllers.ResourceController,
	jobController *controllers.JobController,
	monitoringController *controllers.MonitoringController,
	tenantController *controllers.TenantController,
	accelerationController *controllers.AccelerationController,
	securityController *controllers.SecurityController,
	k8sController *controllers.K8SController) {

	isProduction := cfg.Environment == "production"

	// API 文档默认开放；生产环境下关闭，避免对外暴露接口细节。
	if !isProduction {
		RegisterDocsRoutes(r)
	}

	r.GET("/", func(c *gin.Context) {
		c.File("./frontend/dist/index.html")
	})

	r.Static("/static", "./frontend/dist")
	r.Static("/assets", "./frontend/dist/assets")

	r.GET("/app", func(c *gin.Context) {
		c.File("./frontend/dist/index.html")
	})

	// /backend 内置调试页仅在非生产环境提供。
	if !isProduction {
		r.GET("/backend", func(c *gin.Context) {
			c.Header("Content-Type", "text/html; charset=utf-8")
			indexTemplate.Execute(c.Writer, nil)
		})
	}

	r.GET("/health", gin.WrapH(http.HandlerFunc(services.HealthCheckHandler)))
	r.GET("/metrics", gin.WrapH(services.MetricsHandler()))

	v1 := r.Group("/api/v1")

	auth := v1.Group("/auth")
	{
		auth.POST("/login", authController.Login)
		auth.POST("/register", authController.Register)
		auth.POST("/refresh", middlewares.NewJWTAuth(cfg), authController.Refresh)
		auth.GET("/profile", middlewares.NewJWTAuth(cfg), authController.GetProfile)
	}

	// 调试端点仅在非生产环境注册。
	//
	// /test/slow 原本对匿名调用者开放且延迟无上限，单个请求即可长期占用
	// 一个服务协程，是现成的资源耗尽型 DoS 入口。
	if !isProduction {
		test := v1.Group("/test")
		{
			test.GET("/slow", func(c *gin.Context) {
				delayStr := c.DefaultQuery("delay", "600")
				delay, _ := strconv.Atoi(delayStr)
				if delay < 0 {
					delay = 0
				}
				// 即使参数被篡改也限制在最坏 5 秒，避免协程被长期挂住。
				if delay > maxTestDelayMs {
					delay = maxTestDelayMs
				}
				time.Sleep(time.Duration(delay) * time.Millisecond)
				response.Success(c, gin.H{"message": "slow request completed", "delay_ms": delay})
			})
			test.GET("/error", func(c *gin.Context) {
				response.Error(c, errors.New("test error"))
			})
		}
	}

	authorized := v1.Group("/")
	authorized.Use(middlewares.NewJWTAuth(cfg))
	{
		clusters := authorized.Group("/clusters")
		{
			clusters.GET("", clusterController.GetClusters)
			clusters.POST("", clusterController.CreateCluster)
			clusters.GET("/:id", clusterController.GetCluster)
			clusters.PUT("/:id", clusterController.UpdateCluster)
			clusters.DELETE("/:id", clusterController.DeleteCluster)
			clusters.GET("/:id/status", k8sController.GetClusterStatus)
		}

		resources := authorized.Group("/resources")
		{
			resources.GET("", resourceController.GetResources)
			resources.GET("/:id", resourceController.GetResource)
			resources.PUT("/:id", resourceController.UpdateResource)
			resources.GET("/gpu", k8sController.GetGPUResources)
		}

		jobs := authorized.Group("/jobs")
		{
			jobs.GET("", jobController.GetJobs)
			jobs.POST("", jobController.CreateJob)
			jobs.GET("/:id", jobController.GetJob)
			jobs.PUT("/:id", jobController.UpdateJob)
			jobs.DELETE("/:id", jobController.DeleteJob)
			jobs.POST("/:id/cancel", jobController.CancelJob)
			jobs.POST("/:id/submit", k8sController.SubmitJob)
			jobs.GET("/:id/status", k8sController.GetJobStatus)
		}

		monitoring := authorized.Group("/monitoring")
		{
			monitoring.GET("/metrics", monitoringController.GetMetrics)
			monitoring.GET("/alerts", monitoringController.GetAlerts)
			monitoring.PUT("/alerts/:id/resolve", monitoringController.ResolveAlert)
		}

		tenants := authorized.Group("/tenants")
		{
			tenants.GET("", tenantController.GetTenants)
			tenants.POST("", tenantController.CreateTenant)
			tenants.GET("/:id", tenantController.GetTenant)
			tenants.PUT("/:id", tenantController.UpdateTenant)
			tenants.DELETE("/:id", tenantController.DeleteTenant)
		}

		acceleration := authorized.Group("/acceleration")
		{
			acceleration.GET("", accelerationController.GetAccelerationSuites)
			acceleration.POST("", accelerationController.CreateAccelerationSuite)
			acceleration.GET("/:id", accelerationController.GetAccelerationSuite)
			acceleration.PUT("/:id", accelerationController.UpdateAccelerationSuite)
			acceleration.DELETE("/:id", accelerationController.DeleteAccelerationSuite)
		}

		security := authorized.Group("/security")
		{
			security.GET("/policies", securityController.GetSecurityPolicies)
			security.POST("/policies", securityController.CreateSecurityPolicy)
			security.GET("/policies/:id", securityController.GetSecurityPolicy)
			security.PUT("/policies/:id", securityController.UpdateSecurityPolicy)
			security.DELETE("/policies/:id", securityController.DeleteSecurityPolicy)
		}
	}

	r.NoRoute(func(c *gin.Context) {
		c.File("./frontend/dist/index.html")
	})
}
