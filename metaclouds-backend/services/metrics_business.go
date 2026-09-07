package services

import (
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

// ============================================================================
// HTTP 请求指标（标准命名，供中间件埋点使用）
// ============================================================================

var (
	// HTTPRequestsTotal 记录 HTTP 请求总数，按 method、path、status 分类。
	HTTPRequestsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "http_requests_total",
			Help: "Total number of HTTP requests processed, labeled by method, path and status code.",
		},
		[]string{"method", "path", "status"},
	)

	// HTTPRequestDurationSeconds 记录 HTTP 请求耗时直方图。
	// buckets 覆盖从 5ms 到 10s 的常见延迟区间。
	HTTPRequestDurationSeconds = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "http_request_duration_seconds",
			Help:    "HTTP request latency in seconds, labeled by method and path.",
			Buckets: []float64{0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10},
		},
		[]string{"method", "path"},
	)

	// HTTPRequestsInFlight 记录当前正在处理的 HTTP 请求数。
	HTTPRequestsInFlight = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "http_requests_in_flight",
			Help: "Current number of in-flight HTTP requests being processed.",
		},
	)
)

// ============================================================================
// 业务指标：作业
// ============================================================================

var (
	// MetacloudsJobsTotal 记录作业提交/状态变更总数，按状态和租户分类。
	MetacloudsJobsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "metaclouds_jobs_total",
			Help: "Total number of jobs processed, labeled by status and tenant_id.",
		},
		[]string{"status", "tenant_id"},
	)

	// MetacloudsJobsActive 记录当前运行中的作业数。
	MetacloudsJobsActive = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "metaclouds_jobs_active",
			Help: "Current number of active (running) jobs.",
		},
	)

	// MetacloudsJobsQueueDurationSeconds 记录作业排队时长直方图。
	MetacloudsJobsQueueDurationSeconds = promauto.NewHistogram(
		prometheus.HistogramOpts{
			Name:    "metaclouds_jobs_queue_duration_seconds",
			Help:    "Time jobs spend in queue before starting, in seconds.",
			Buckets: []float64{1, 5, 10, 30, 60, 120, 300, 600, 1800},
		},
	)
)

// ============================================================================
// 业务指标：资源 / 集群 / 租户
// ============================================================================

var (
	// MetacloudsResourcesGPUTotal 记录 GPU 总数，按集群和节点分类。
	MetacloudsResourcesGPUTotal = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "metaclouds_resources_gpu_total",
			Help: "Total number of GPUs, labeled by cluster_id and node_id.",
		},
		[]string{"cluster_id", "node_id"},
	)

	// MetacloudsResourcesGPUAllocated 记录已分配 GPU 数，按集群和节点分类。
	MetacloudsResourcesGPUAllocated = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "metaclouds_resources_gpu_allocated",
			Help: "Number of allocated GPUs, labeled by cluster_id and node_id.",
		},
		[]string{"cluster_id", "node_id"},
	)

	// MetacloudsClustersTotal 记录集群总数，按状态分类。
	MetacloudsClustersTotal = promauto.NewGaugeVec(
		prometheus.GaugeOpts{
			Name: "metaclouds_clusters_total",
			Help: "Total number of clusters, labeled by status.",
		},
		[]string{"status"},
	)

	// MetacloudsTenantsTotal 记录租户总数。
	MetacloudsTenantsTotal = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "metaclouds_tenants_total",
			Help: "Total number of tenants.",
		},
	)
)

// ============================================================================
// 业务指标：认证 / 调度
// ============================================================================

var (
	// MetacloudsAuthLoginTotal 记录登录尝试总数，按结果（success/failure）分类。
	MetacloudsAuthLoginTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "metaclouds_auth_login_total",
			Help: "Total number of login attempts, labeled by result (success/failure).",
		},
		[]string{"result"},
	)

	// MetacloudsSchedulerDecisionsTotal 记录调度决策总数，按动作分类。
	// action: allocate（分配成功）、preempt（抢占）、queue（排队等待）
	MetacloudsSchedulerDecisionsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "metaclouds_scheduler_decisions_total",
			Help: "Total number of scheduler decisions, labeled by action (allocate/preempt/queue).",
		},
		[]string{"action"},
	)

	// MetacloudsBackendUp 后端服务存活探针，1=正常，0=异常。
	// 由 main 在启动时设为 1，关闭时设为 0。
	MetacloudsBackendUp = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "metaclouds_backend_up",
			Help: "Metaclouds backend service health status (1=up, 0=down).",
		},
	)
)

// ============================================================================
// 指标辅助函数：供 service 层调用
// ============================================================================

// RecordJobStatus 记录一次作业状态变更。
// status: pending/running/completed/failed/cancelled
// tenantID: 租户 ID（0 表示未关联租户）
func RecordJobStatus(status string, tenantID uint) {
	tenantStr := "0"
	if tenantID > 0 {
		tenantStr = uintToString(tenantID)
	}
	MetacloudsJobsTotal.WithLabelValues(status, tenantStr).Inc()

	// 同步更新运行中作业数的 Gauge
	switch status {
	case "running":
		MetacloudsJobsActive.Inc()
	case "completed", "failed", "cancelled":
		MetacloudsJobsActive.Dec()
	}
}

// RecordJobQueueDuration 记录作业排队时长。
func RecordJobQueueDuration(duration time.Duration) {
	MetacloudsJobsQueueDurationSeconds.Observe(duration.Seconds())
}

// RecordLoginResult 记录一次登录结果。
// result: "success" 或 "failure"
func RecordLoginResult(result string) {
	MetacloudsAuthLoginTotal.WithLabelValues(result).Inc()
}

// RecordSchedulerDecision 记录一次调度决策。
// action: "allocate" / "preempt" / "queue"
func RecordSchedulerDecision(action string) {
	MetacloudsSchedulerDecisionsTotal.WithLabelValues(action).Inc()
}

// SetBackendUp 设置后端存活状态。
func SetBackendUp(up bool) {
	if up {
		MetacloudsBackendUp.Set(1)
	} else {
		MetacloudsBackendUp.Set(0)
	}
}

// uintToString 将 uint 转为字符串，避免在热路径上重复导入 strconv。
func uintToString(v uint) string {
	if v == 0 {
		return "0"
	}
	var buf [20]byte
	i := len(buf)
	for v > 0 {
		i--
		buf[i] = byte('0' + v%10)
		v /= 10
	}
	return string(buf[i:])
}
