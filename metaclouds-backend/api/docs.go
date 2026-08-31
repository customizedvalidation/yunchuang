package api

import (
	"net/http"
	"text/template"

	"github.com/gin-gonic/gin"
)

func RegisterDocsRoutes(r *gin.Engine) {
	r.GET("/api/docs", swaggerDocsHandler)
	r.GET("/api/docs/swagger.yaml", swaggerYAMLHandler)
	r.Static("/api/docs/swagger-ui", "./api/swagger-ui")
}

func swaggerDocsHandler(c *gin.Context) {
	tmpl := template.Must(template.New("swagger").Parse(`
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <title>Metaclouds API Documentation</title>
    <style>
        body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 0; padding: 20px; background: #f5f5f5; }
        .container { max-width: 800px; margin: 0 auto; background: white; padding: 40px; border-radius: 8px; box-shadow: 0 2px 10px rgba(0,0,0,0.1); }
        h1 { color: #2c3e50; margin-bottom: 20px; }
        p { color: #7f8c8d; line-height: 1.6; }
        .links { margin-top: 30px; }
        .link { display: inline-block; padding: 12px 24px; background: #3498db; color: white; text-decoration: none; border-radius: 4px; margin-right: 10px; transition: background 0.3s; }
        .link:hover { background: #2980b9; }
        .link.secondary { background: #95a5a6; }
        .link.secondary:hover { background: #7f8c8d; }
    </style>
</head>
<body>
    <div class="container">
        <h1>Metaclouds API Documentation</h1>
        <p>Welcome to the Metaclouds Backend API documentation. Below are the available documentation formats:</p>
        <div class="links">
            <a href="/api/docs/swagger-ui/" class="link">Swagger UI</a>
            <a href="/api/docs/swagger.yaml" class="link secondary">OpenAPI YAML</a>
        </div>
        <h2 style="margin-top: 30px; color: #2c3e50;">API Endpoints</h2>
        <ul style="color: #7f8c8d; line-height: 2;">
            <li><strong>Authentication:</strong> POST /api/v1/auth/login, POST /api/v1/auth/register</li>
            <li><strong>Clusters:</strong> GET/POST /api/v1/clusters, GET/PUT/DELETE /api/v1/clusters/{id}</li>
            <li><strong>Jobs:</strong> GET/POST /api/v1/jobs, GET/PUT/DELETE /api/v1/jobs/{id}</li>
            <li><strong>Monitoring:</strong> GET /api/v1/monitoring/metrics, GET /api/v1/monitoring/alerts</li>
            <li><strong>Health:</strong> GET /health</li>
            <li><strong>Metrics:</strong> GET /metrics</li>
        </ul>
        <h2 style="margin-top: 30px; color: #2c3e50;">Authentication</h2>
        <p style="color: #7f8c8d;">All endpoints except /health, /metrics, /auth/login, and /auth/register require authentication. Use the Bearer token in the Authorization header.</p>
        <pre style="background: #f8f9fa; padding: 15px; border-radius: 4px; overflow-x: auto; color: #34495e;">Authorization: Bearer &lt;your-token&gt;</pre>
    </div>
</body>
</html>
`))
	tmpl.Execute(c.Writer, nil)
}

func swaggerYAMLHandler(c *gin.Context) {
	c.Header("Content-Type", "application/x-yaml")
	http.ServeFile(c.Writer, c.Request, "docs/swagger.yaml")
}
