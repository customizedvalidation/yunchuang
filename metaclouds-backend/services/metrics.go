package services

import (
	"context"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"time"

	"metaclouds-backend/pkg/logger"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"github.com/prometheus/client_golang/prometheus/promhttp"
)

var (
	cpuUsageGauge = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "cpu_usage_percent",
			Help: "Current CPU usage percentage",
		},
		[]string{"node"},
	)
	memoryUsageGauge = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "memory_usage_percent",
			Help: "Current memory usage percentage",
		},
		[]string{"node"},
	)
	gpuUsageGauge = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "gpu_usage_percent",
			Help: "Current GPU usage percentage",
		},
		[]string{"node", "gpu_id"},
	)
	gpuMemoryUsageGauge = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "gpu_memory_usage_percent",
			Help: "Current GPU memory usage percentage",
		},
		[]string{"node", "gpu_id"},
	)
	gpuMemoryAvailableGauge = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "gpu_memory_available_bytes",
			Help: "Available GPU memory in bytes",
		},
		[]string{"node", "gpu_id"},
	)
	gpuAllocatedCounter = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "gpu_allocated_total",
			Help: "Total number of GPUs allocated",
		},
		[]string{"cluster"},
	)
	gpuReleasedCounter = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "gpu_released_total",
			Help: "Total number of GPUs released",
		},
		[]string{"cluster"},
	)
	gpuAllocationFailedCounter = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "gpu_allocation_failed_total",
			Help: "Total number of failed GPU allocations",
		},
		[]string{"cluster"},
	)
	jobTotalCounter = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "job_total_count",
			Help: "Total number of jobs processed",
		},
		[]string{"status"},
	)
	jobFailedGauge = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "job_failed_count",
			Help: "Current number of failed jobs",
		},
	)
	jobRunningGauge = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "job_running_count",
			Help: "Number of running jobs",
		},
	)
	jobPendingGauge = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "job_pending_count",
			Help: "Number of pending jobs",
		},
	)
	jobCompletedGauge = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "job_completed_count",
			Help: "Number of completed jobs",
		},
	)
	apiRequestCounter = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "api_request_total",
			Help: "Total number of API requests",
		},
		[]string{"method", "endpoint", "status"},
	)
	apiLatencyHistogram = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "api_latency_seconds",
			Help:    "API request latency",
			Buckets: []float64{0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1, 2, 5, 10, 30},
		},
		[]string{"endpoint", "method", "status"},
	)
	apiActiveRequestsGauge = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "api_active_requests",
			Help: "Number of currently active API requests",
		},
	)
	databaseConnectionGauge = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "database_connections_active",
			Help: "Number of active database connections",
		},
	)
	redisConnectionGauge = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "redis_connections_active",
			Help: "Number of active Redis connections",
		},
	)
	clusterCountGauge = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "cluster_count",
			Help: "Number of registered clusters",
		},
	)
	nodeCountGauge = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "node_count",
			Help: "Total number of nodes across all clusters",
		},
	)
	resourceUtilizationGauge = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "resource_utilization_percent",
			Help: "Resource utilization percentage",
		},
		[]string{"resource_type", "cluster"},
	)
	rateLimitHitCounter = promauto.NewCounter(
		prometheus.CounterOpts{
			Name: "rate_limit_hits_total",
			Help: "Total number of rate limit hits",
		},
	)
	circuitBreakerTrippedCounter = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "circuit_breaker_tripped_total",
			Help: "Total number of circuit breaker trips",
		},
		[]string{"service"},
	)
	schedulerRunCounter = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "scheduler_runs_total",
			Help: "Total number of scheduler runs",
		},
		[]string{"schedule_name", "result"},
	)
	schedulerLatencyHistogram = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "scheduler_latency_seconds",
			Help:    "Scheduler execution latency",
			Buckets: []float64{0.1, 0.5, 1, 2, 5, 10},
		},
		[]string{"schedule_name"},
	)
	uptimeCounter = promauto.NewCounter(
		prometheus.CounterOpts{
			Name: "service_uptime_seconds",
			Help: "Service uptime in seconds",
		},
	)
)

type MetricsCollector struct {
	monitoringService *MonitoringService
	jobService        *JobService
	startedAt         time.Time
	mu                sync.RWMutex
	running           bool
}

func NewMetricsCollector(monitoringService *MonitoringService, jobService *JobService) *MetricsCollector {
	return &MetricsCollector{
		monitoringService: monitoringService,
		jobService:        jobService,
		startedAt:         time.Now(),
		running:           true,
	}
}

func (m *MetricsCollector) StartCollecting(interval time.Duration) {
	logger.InfoWithCtx(context.Background(), "Starting metrics collection")
	ticker := time.NewTicker(interval)
	defer ticker.Stop()

	go m.startUptimeCounter()

	for range ticker.C {
		m.mu.RLock()
		if !m.running {
			m.mu.RUnlock()
			break
		}
		m.mu.RUnlock()

		m.collectMetrics()
	}
}

func (m *MetricsCollector) startUptimeCounter() {
	ticker := time.NewTicker(time.Second)
	defer ticker.Stop()

	for range ticker.C {
		m.mu.RLock()
		if !m.running {
			m.mu.RUnlock()
			break
		}
		m.mu.RUnlock()

		uptimeCounter.Inc()
	}
}

func (m *MetricsCollector) Stop() {
	m.mu.Lock()
	m.running = false
	m.mu.Unlock()
	logger.InfoWithCtx(context.Background(), "Metrics collection stopped")
}

func (m *MetricsCollector) collectMetrics() {
	metrics, err := m.monitoringService.GetMetrics()
	if err != nil {
		logger.ErrorWithCtx(context.Background(), "Error collecting metrics", err)
		return
	}

	if cpu, ok := metrics["cpu"].(map[string]interface{}); ok {
		if usage, ok := cpu["usage"].(float64); ok {
			cpuUsageGauge.WithLabelValues("localhost").Set(usage)
		}
	}

	if memory, ok := metrics["memory"].(map[string]interface{}); ok {
		if usage, ok := memory["usage"].(float64); ok {
			memoryUsageGauge.WithLabelValues("localhost").Set(usage)
		}
	}

	if gpu, ok := metrics["gpu"].(map[string]interface{}); ok {
		if usage, ok := gpu["usage"].(float64); ok {
			gpuUsageGauge.WithLabelValues("localhost", "0").Set(usage)
		}
		if memUsage, ok := gpu["memory_usage"].(float64); ok {
			gpuMemoryUsageGauge.WithLabelValues("localhost", "0").Set(memUsage)
		}
		if available, ok := gpu["available"].(float64); ok {
			gpuMemoryAvailableGauge.WithLabelValues("localhost", "0").Set(available * 1024 * 1024 * 1024)
		}
	}

	if jobs, ok := metrics["jobs"].(map[string]interface{}); ok {
		if running, ok := jobs["running"].(int); ok {
			jobRunningGauge.Set(float64(running))
		}
		if pending, ok := jobs["pending"].(int); ok {
			jobPendingGauge.Set(float64(pending))
		}
		if completed, ok := jobs["completed"].(int); ok {
			jobCompletedGauge.Set(float64(completed))
		}
		if failed, ok := jobs["failed"].(int); ok {
			jobFailedGauge.Set(float64(failed))
		}
	}
}

func (m *MetricsCollector) RecordAPILatency(endpoint, method string, status int, latency time.Duration) {
	apiLatencyHistogram.WithLabelValues(endpoint, method, strconv.Itoa(status)).Observe(latency.Seconds())
}

func (m *MetricsCollector) RecordAPIRequest(method, endpoint string, status int) {
	apiRequestCounter.WithLabelValues(method, endpoint, strconv.Itoa(status)).Inc()
}

func (m *MetricsCollector) IncrementActiveRequests() {
	apiActiveRequestsGauge.Inc()
}

func (m *MetricsCollector) DecrementActiveRequests() {
	apiActiveRequestsGauge.Dec()
}

func (m *MetricsCollector) UpdateClusterCount(count int) {
	clusterCountGauge.Set(float64(count))
}

func (m *MetricsCollector) UpdateNodeCount(count int) {
	nodeCountGauge.Set(float64(count))
}

func (m *MetricsCollector) RecordSchedulerRun(scheduleName, result string, latency time.Duration) {
	schedulerRunCounter.WithLabelValues(scheduleName, result).Inc()
	schedulerLatencyHistogram.WithLabelValues(scheduleName).Observe(latency.Seconds())
}

func (m *MetricsCollector) RecordRateLimitHit() {
	rateLimitHitCounter.Inc()
}

func (m *MetricsCollector) RecordCircuitBreakerTrip(service string) {
	circuitBreakerTrippedCounter.WithLabelValues(service).Inc()
}

func MetricsHandler() http.Handler {
	return promhttp.Handler()
}

// healthChecks 保存由 main 在启动时注册的关键依赖探针（数据库、缓存等）。
// /health 不再恒为 healthy，只有全部探针通过才返回 200。
var (
	healthCheckMu sync.Mutex
	healthChecks  []func() error
)

// RegisterHealthCheck 注册一个依赖健康探针，/health 会依次执行它们。
func RegisterHealthCheck(fn func() error) {
	healthCheckMu.Lock()
	defer healthCheckMu.Unlock()
	healthChecks = append(healthChecks, fn)
}

func HealthCheckHandler(w http.ResponseWriter, r *http.Request) {
	healthCheckMu.Lock()
	checks := make([]func() error, len(healthChecks))
	copy(checks, healthChecks)
	healthCheckMu.Unlock()

	var failed []string
	for _, fn := range checks {
		if err := fn(); err != nil {
			failed = append(failed, err.Error())
		}
	}

	status := "healthy"
	code := http.StatusOK
	detail := ""
	if len(failed) > 0 {
		status = "unhealthy"
		code = http.StatusServiceUnavailable
		detail = strings.Join(failed, "; ")
	}

	if strings.Contains(r.Header.Get("Accept"), "text/html") {
		w.Header().Set("Content-Type", "text/html; charset=utf-8")
		w.WriteHeader(code)
		w.Write([]byte(healthHTML(status, detail)))
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	w.Write([]byte(`{"status":"` + status + `","timestamp":` + strconv.FormatInt(time.Now().Unix(), 10) + `,"dependencies":` + strconv.Itoa(len(checks)) + `}`))
}

func healthHTML(status, detail string) string {
	titleColor := "#28a745"
	title := "Service is Healthy"
	msg := "Metaclouds Backend API is running normally"
	if status != "healthy" {
		titleColor = "#dc3545"
		title = "Service is Unhealthy"
		msg = "Dependency check failed: " + detail
	}
	return `<!DOCTYPE html><html lang="zh-CN"><head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Metaclouds - Health Check</title><style>*{margin:0;padding:0;box-sizing:border-box;}body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,sans-serif;background:linear-gradient(135deg,#667eea 0%,#764ba2 100%);min-height:100vh;display:flex;justify-content:center;align-items:center;padding:20px;}.container{background:white;border-radius:16px;box-shadow:0 20px 60px rgba(0,0,0,0.15);padding:40px;max-width:500px;width:100%;text-align:center;}.status-icon{font-size:64px;margin-bottom:20px;}.status-title{font-size:28px;font-weight:700;margin-bottom:8px;}.status-message{color:#6c757d;font-size:16px;margin-bottom:24px;}</style></head><body><div class="container"><div class="status-icon">` + statusIcon(status) + `</div><h1 class="status-title" style="color:` + titleColor + `">` + title + `</h1><p class="status-message">` + msg + `</p></div></body></html>`
}

func statusIcon(status string) string {
	if status == "healthy" {
		return "✅"
	}
	return "❌"
}
