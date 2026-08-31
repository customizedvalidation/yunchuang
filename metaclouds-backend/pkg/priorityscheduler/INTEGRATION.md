# 优先级调度器 - 微服务集成指南

## 📋 目录

1. [模块配置项详解](#1-模块配置项详解)
2. [与现有JobService集成](#2-与现有jobservice集成)
3. [依赖注入模式](#3-依赖注入模式)
4. [错误处理策略](#4-错误处理策略)
5. [日志与监控集成](#5-日志与监控集成)
6. [多租户支持](#6-多租户支持)
7. [性能优化建议](#7-性能优化建议)
8. [完整集成示例](#8-完整集成示例)

---

## 1. 模块配置项详解

### 核心配置参数

```go
type SchedulerConfig struct {
    BufferSize          int    // Channel缓冲区大小
    MaxConcurrentTasks  int    // 最大并发任务数
    EnableNotifications bool   // 是否启用优先级变更通知
}
```

### 配置建议

| 环境 | BufferSize | MaxConcurrentTasks | EnableNotifications |
|------|-----------|-------------------|-------------------|
| **开发环境** | 50-100 | 5-10 | true |
| **测试环境** | 100-200 | 10-20 | true |
| **生产环境** | 200-500 | 20-50 | true |
| **高并发场景** | 500-1000 | 50-100 | true |

### 环境特定配置示例

```go
func getSchedulerConfig(env string) *SchedulerConfig {
    switch env {
    case "production":
        return &SchedulerConfig{
            BufferSize:          500,
            MaxConcurrentTasks:  50,
            EnableNotifications: true,
        }
    case "development":
        return &SchedulerConfig{
            BufferSize:          100,
            MaxConcurrentTasks:  10,
            EnableNotifications: true,
        }
    default:
        return DefaultConfig()
    }
}
```

---

## 2. 与现有JobService集成

### 集成架构图

```
┌─────────────────────────────────────────────────────────────┐
│                      API Layer                               │
│  POST /api/v1/jobs  │  PUT /api/v1/jobs/:id  │  GET ...   │
└──────────┬──────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────┐
│                    JobService                                │
│  • CreateJob()                                              │
│  • UpdateJob()  ────► PriorityScheduler.UpdatePriority()     │
│  • GetJobs()                                                │
└──────────┬──────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────┐
│               PriorityScheduler (独立模块)                   │
│  • 优先级队列管理                                           │
│  • 任务调度                                                 │
│  • 变更通知                                                  │
└──────────┬──────────────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────────────┐
│               MemoryStore / Redis                            │
└─────────────────────────────────────────────────────────────┘
```

### 集成方式一：直接集成（推荐）

```go
package services

import (
    "metaclouds-backend/pkg/priorityscheduler"
)

type JobService struct {
    db           *models.MemoryStore
    scheduler    *priorityscheduler.PriorityScheduler
    config       *config.Config
}

func NewJobService(db *models.MemoryStore, cfg *config.Config, k8sService *K8SService) *JobService {
    schedulerConfig := getSchedulerConfig(cfg.Environment)
    
    return &JobService{
        db:        db,
        scheduler: priorityscheduler.NewPriorityScheduler(schedulerConfig),
        config:    cfg,
    }
}

func (s *JobService) CreateJob(req CreateJobRequest) (*models.Job, error) {
    job := &models.Job{
        Name:     req.Name,
        Priority: req.Priority,
        Status:   "pending",
    }
    
    s.db.Mu.Lock()
    s.db.Jobs[job.ID] = job
    s.db.Mu.Unlock()
    
    // 集成到优先级调度器
    s.scheduler.AddTask(job)
    
    return job, nil
}

func (s *JobService) UpdateJob(id uint, req UpdateJobRequest) (*models.Job, error) {
    s.db.Mu.Lock()
    job, exists := s.db.Jobs[id]
    if !exists {
        s.db.Mu.Unlock()
        return nil, ErrJobNotFound
    }
    
    if req.Priority != nil && *req.Priority != job.Priority {
        oldPriority := job.Priority
        job.Priority = *req.Priority
        
        // 同步更新到优先级调度器
        if err := s.scheduler.UpdatePriority(id, *req.Priority); err != nil {
            job.Priority = oldPriority
            s.db.Mu.Unlock()
            return nil, err
        }
    }
    
    s.db.Mu.Unlock()
    return job, nil
}
```

### 集成方式二：事件驱动集成

```go
type JobEvent struct {
    Type    string
    JobID   uint
    Payload interface{}
}

func (s *JobService) setupEventDrivenIntegration() {
    go s.listenToSchedulerNotifications()
}

func (s *JobService) listenToSchedulerNotifications() {
    for change := range s.scheduler.GetPriorityChangeChannel() {
        s.logPriorityChange(change)
        
        // 触发自定义事件处理
        event := JobEvent{
            Type:  "priority_changed",
            JobID: change.TaskID,
            Payload: change,
        }
        
        s.publishEvent(event)
    }
}
```

---

## 3. 依赖注入模式

### 3.1 使用构造器注入

```go
type ServiceContainer struct {
    JobService    *JobService
    Scheduler     *priorityscheduler.PriorityScheduler
    Config       *config.Config
    Logger       *logger.Logger
    Metrics      *metrics.Collector
}

func NewServiceContainer(cfg *config.Config) *ServiceContainer {
    // 创建共享的调度器实例
    schedulerConfig := &priorityscheduler.SchedulerConfig{
        BufferSize:          cfg.SchedulerBufferSize,
        MaxConcurrentTasks:  cfg.SchedulerMaxConcurrent,
        EnableNotifications: true,
    }
    scheduler := priorityscheduler.NewPriorityScheduler(schedulerConfig)
    
    // 创建数据库实例
    db := models.NewMemoryStore()
    
    // 创建服务实例
    jobService := NewJobService(db, cfg, nil)
    
    return &ServiceContainer{
        JobService: jobService,
        Scheduler:  scheduler,
        Config:    cfg,
    }
}
```

### 3.2 使用接口抽象

```go
type TaskScheduler interface {
    AddTask(task priorityscheduler.Task) error
    UpdatePriority(taskID uint, priority int) error
    GetNextTask() (priorityscheduler.Task, error)
    CompleteTask(taskID uint) error
    GetTasks() []priorityscheduler.Task
}

type ConcreteScheduler struct {
    scheduler *priorityscheduler.PriorityScheduler
}

func (s *ConcreteScheduler) AddTask(task priorityscheduler.Task) error {
    return s.scheduler.AddTask(task)
}

func (s *ConcreteScheduler) UpdatePriority(taskID uint, priority int) error {
    return s.scheduler.UpdatePriority(taskID, priority)
}

// ... 其他方法实现
```

---

## 4. 错误处理策略

### 4.1 错误类型映射

```go
import "metaclouds-backend/pkg/priorityscheduler"

func mapSchedulerError(err error) error {
    switch err {
    case priorityscheduler.ErrNilTask:
        return ErrInvalidTask
    case priorityscheduler.ErrInvalidPriority:
        return ErrInvalidPriority
    case priorityscheduler.ErrTaskNotFound:
        return ErrJobNotFound
    case priorityscheduler.ErrNoPendingTasks:
        return ErrNoPendingJobs
    default:
        return ErrInternalServer
    }
}
```

### 4.2 重试机制

```go
func withRetry(maxRetries int, operation func() error) error {
    var lastErr error
    
    for attempt := 0; attempt < maxRetries; attempt++ {
        if err := operation(); err != nil {
            lastErr = err
            
            if isRetryableError(err) {
                time.Sleep(time.Duration(attempt+1) * 100 * time.Millisecond)
                continue
            }
            
            return err
        }
        return nil
    }
    
    return lastErr
}

func isRetryableError(err error) bool {
    return err == priorityscheduler.ErrTaskNotFound || 
           err == context.DeadlineExceeded
}
```

### 4.3 超时控制

```go
import "context"

func (s *JobService) scheduleWithTimeout(timeout time.Duration) error {
    ctx, cancel := context.WithTimeout(context.Background(), timeout)
    defer cancel()
    
    done := make(chan error, 1)
    
    go func() {
        task, err := s.scheduler.GetNextTask()
        if err != nil {
            done <- err
            return
        }
        
        // 执行任务
        err = s.executeTask(task)
        done <- err
    }()
    
    select {
    case err := <-done:
        return err
    case <-ctx.Done():
        return ErrTimeout
    }
}
```

---

## 5. 日志与监控集成

### 5.1 日志集成

```go
import (
    "metaclouds-backend/pkg/logger"
)

func (s *JobService) setupLogging() {
    go s.logSchedulerEvents()
}

func (s *JobService) logSchedulerEvents() {
    for change := range s.scheduler.GetPriorityChangeChannel() {
        logger.InfoWithCtx(context.Background(), "Priority changed",
            "task_id", change.TaskID,
            "old_priority", change.OldPriority,
            "new_priority", change.NewPriority,
            "timestamp", change.Timestamp,
        )
    }
}

// 集成到现有日志系统
func (s *JobService) logPriorityChange(change priorityscheduler.PriorityChange) {
    s.logger.WithFields(logger.Fields{
        "job_id":       change.TaskID,
        "old_priority": change.OldPriority,
        "new_priority": change.NewPriority,
    }).Info("Job priority updated")
}
```

### 5.2 Prometheus监控指标

```go
import "github.com/prometheus/client_golang/prometheus"

var (
    schedulerTasksGauge = prometheus.NewGaugeVec(
        prometheus.GaugeOpts{
            Name: "scheduler_tasks_total",
            Help: "Total number of tasks in scheduler",
        },
        []string{"priority", "status"},
    )
    
    priorityChangesCounter = prometheus.NewCounterVec(
        prometheus.CounterOpts{
            Name: "scheduler_priority_changes_total",
            Help: "Total number of priority changes",
        },
        []string{"direction"},
    )
)

func (s *JobService) setupMetrics() {
    prometheus.MustRegister(schedulerTasksGauge)
    prometheus.MustRegister(priorityChangesCounter)
    
    go s.collectMetrics()
}

func (s *JobService) collectMetrics() {
    ticker := time.NewTicker(10 * time.Second)
    defer ticker.Stop()
    
    for range ticker.C {
        tasks := s.scheduler.GetTasks()
        
        priorityCounts := map[int]int{}
        statusCounts := map[string]int{}
        
        for _, task := range tasks {
            priorityCounts[task.GetPriority()]++
            statusCounts[task.GetStatus()]++
        }
        
        for priority, count := range priorityCounts {
            schedulerTasksGauge.WithLabelValues(
                strconv.Itoa(priority),
                "total",
            ).Set(float64(count))
        }
    }
}
```

### 5.3 健康检查

```go
func (s *JobService) HealthCheck() error {
    tasks := s.scheduler.GetTasks()
    
    if len(tasks) > s.config.MaxQueueSize {
        return fmt.Errorf("scheduler queue overflow: %d > %d",
            len(tasks), s.config.MaxQueueSize)
    }
    
    return nil
}
```

---

## 6. 多租户支持

### 6.1 租户隔离方案

```go
type MultiTenantScheduler struct {
    tenants    map[string]*priorityscheduler.PriorityScheduler
    mu         sync.RWMutex
    config     *SchedulerConfig
}

func NewMultiTenantScheduler(cfg *SchedulerConfig) *MultiTenantScheduler {
    return &MultiTenantScheduler{
        tenants: make(map[string]*priorityscheduler.PriorityScheduler),
        config:  cfg,
    }
}

func (m *MultiTenantScheduler) GetTenantScheduler(tenantID string) *priorityscheduler.PriorityScheduler {
    m.mu.RLock()
    scheduler, exists := m.tenants[tenantID]
    m.mu.RUnlock()
    
    if exists {
        return scheduler
    }
    
    m.mu.Lock()
    defer m.mu.Unlock()
    
    if scheduler, exists = m.tenants[tenantID]; exists {
        return scheduler
    }
    
    scheduler = priorityscheduler.NewPriorityScheduler(m.config)
    m.tenants[tenantID] = scheduler
    
    return scheduler
}

func (m *MultiTenantScheduler) CreateJob(tenantID string, job *models.Job) error {
    scheduler := m.GetTenantScheduler(tenantID)
    return scheduler.AddTask(job)
}
```

### 6.2 资源配额管理

```go
type TenantQuota struct {
    MaxTasks          int
    MaxPriorityTasks  int
    MaxConcurrent     int
}

type QuotaManager struct {
    quotas map[string]*TenantQuota
}

func (q *QuotaManager) CanAddTask(tenantID string, priority int) bool {
    quota := q.quotas[tenantID]
    if quota == nil {
        return true
    }
    
    if priority == 3 && quota.MaxPriorityTasks > 0 {
        // 检查高优先级任务配额
        return true
    }
    
    return true
}
```

---

## 7. 性能优化建议

### 7.1 批量操作优化

```go
func (s *JobService) BatchAddTasks(tasks []models.Job) error {
    schedulerTasks := make([]priorityscheduler.Task, len(tasks))
    
    s.db.Mu.Lock()
    for i, task := range tasks {
        s.db.Jobs[task.ID] = &task
        schedulerTasks[i] = &task
    }
    s.db.Mu.Unlock()
    
    for _, task := range schedulerTasks {
        if err := s.scheduler.AddTask(task); err != nil {
            return err
        }
    }
    
    return nil
}
```

### 7.2 连接池配置

```go
type SchedulerPool struct {
    schedulers []*priorityscheduler.PriorityScheduler
    current   int
    mu        sync.Mutex
}

func NewSchedulerPool(size int, cfg *SchedulerConfig) *SchedulerPool {
    pool := &SchedulerPool{
        schedulers: make([]*priorityscheduler.PriorityScheduler, size),
    }
    
    for i := 0; i < size; i++ {
        pool.schedulers[i] = priorityscheduler.NewPriorityScheduler(cfg)
    }
    
    return pool
}

func (p *SchedulerPool) Get() *priorityscheduler.PriorityScheduler {
    p.mu.Lock()
    defer p.mu.Unlock()
    
    scheduler := p.schedulers[p.current]
    p.current = (p.current + 1) % len(p.schedulers)
    
    return scheduler
}
```

### 7.3 缓存策略

```go
type CachedScheduler struct {
    scheduler *priorityscheduler.PriorityScheduler
    cache     *ristretto.Cache
    ttl       time.Duration
}

func NewCachedScheduler(scheduler *priorityscheduler.PriorityScheduler) *CachedScheduler {
    cache, _ := ristretto.NewCache(&ristretto.Config{
        NumCounters: 10000,
        MaxCost:     1000 << 20,
        BufferItems: 64,
    })
    
    return &CachedScheduler{
        scheduler: scheduler,
        cache:     cache,
        ttl:       30 * time.Second,
    }
}

func (c *CachedScheduler) GetTask(id uint) (priorityscheduler.Task, error) {
    if task, ok := c.cache.Get(id); ok {
        return task.(priorityscheduler.Task), nil
    }
    
    return c.scheduler.GetTask(id)
}
```

### 7.4 性能基准测试

```go
func BenchmarkSchedulerOperations(b *testing.B) {
    scheduler := priorityscheduler.NewPriorityScheduler(
        priorityscheduler.DefaultConfig(),
    )
    
    tasks := make([]*models.Job, 1000)
    for i := 0; i < 1000; i++ {
        tasks[i] = &models.Job{
            ID:       uint(i),
            Priority: i % 4,
            Status:   "pending",
        }
    }
    
    b.ResetTimer()
    
    for i := 0; i < b.N; i++ {
        for _, task := range tasks {
            scheduler.AddTask(task)
        }
    }
}
```

---

## 8. 完整集成示例

### 8.1 项目结构

```
your-project/
├── cmd/
│   └── server/
│       └── main.go
├── internal/
│   ├── scheduler/
│   │   ├── adapter.go
│   │   └── config.go
│   ├── service/
│   │   └── job_service.go
│   └── handler/
│       └── job_handler.go
├── pkg/
│   └── priorityscheduler/  # 独立模块
├── configs/
│   └── config.yaml
└── go.mod
```

### 8.2 配置加载

```go
// configs/config.yaml
scheduler:
  buffer_size: 200
  max_concurrent_tasks: 20
  enable_notifications: true
  pool_size: 5
```

```go
// internal/scheduler/config.go
type Config struct {
    BufferSize          int  `yaml:"buffer_size"`
    MaxConcurrentTasks  int  `yaml:"max_concurrent_tasks"`
    EnableNotifications bool `yaml:"enable_notifications"`
    PoolSize            int  `yaml:"pool_size"`
}

func LoadSchedulerConfig(path string) (*Config, error) {
    data, err := os.ReadFile(path)
    if err != nil {
        return nil, err
    }
    
    var cfg Config
    if err := yaml.Unmarshal(data, &cfg); err != nil {
        return nil, err
    }
    
    if cfg.BufferSize == 0 {
        cfg.BufferSize = 200
    }
    
    return &cfg, nil
}
```

### 8.3 适配器实现

```go
// internal/scheduler/adapter.go
package scheduler

import (
    "your-project/internal/models"
    "your-project/pkg/priorityscheduler"
)

type JobAdapter struct{}

func (a *JobAdapter) ToTask(job *models.Job) priorityscheduler.Task {
    return &jobAdapter{task: job}
}

type jobAdapter struct {
    task *models.Job
}

func (a *jobAdapter) GetID() uint {
    return a.task.ID
}

func (a *jobAdapter) GetPriority() int {
    return a.task.Priority
}

func (a *jobAdapter) GetStatus() string {
    return a.task.Status
}

func (a *jobAdapter) SetPriority(priority int) {
    a.task.Priority = priority
}

func (a *jobAdapter) SetStatus(status string) {
    a.task.Status = status
}
```

### 8.4 服务集成

```go
// internal/service/job_service.go
package service

import (
    "your-project/internal/models"
    "your-project/internal/scheduler"
    "your-project/pkg/priorityscheduler"
)

type JobService struct {
    db        *models.MemoryStore
    scheduler *priorityscheduler.PriorityScheduler
    adapter   *scheduler.JobAdapter
}

func NewJobService(db *models.MemoryStore, cfg *scheduler.Config) *JobService {
    psCfg := &priorityscheduler.SchedulerConfig{
        BufferSize:          cfg.BufferSize,
        MaxConcurrentTasks:  cfg.MaxConcurrentTasks,
        EnableNotifications: cfg.EnableNotifications,
    }
    
    return &JobService{
        db:        db,
        scheduler: priorityscheduler.NewPriorityScheduler(psCfg),
        adapter:   &scheduler.JobAdapter{},
    }
}

func (s *JobService) CreateJob(job *models.Job) error {
    s.db.Mu.Lock()
    s.db.Jobs[job.ID] = job
    s.db.Mu.Unlock()
    
    task := s.adapter.ToTask(job)
    return s.scheduler.AddTask(task)
}

func (s *JobService) UpdatePriority(id uint, priority int) error {
    s.db.Mu.Lock()
    job, exists := s.db.Jobs[id]
    s.db.Mu.Unlock()
    
    if !exists {
        return fmt.Errorf("job not found")
    }
    
    job.Priority = priority
    return s.scheduler.UpdatePriority(id, priority)
}
```

### 8.5 启动和关闭

```go
// cmd/server/main.go
func main() {
    cfg, err := scheduler.LoadConfig("configs/config.yaml")
    if err != nil {
        log.Fatal(err)
    }
    
    jobService := service.NewJobService(db, cfg)
    
    // 启动调度器监控
    go jobService.StartSchedulerMonitoring()
    
    // 启动HTTP服务器
    server := http.Server{
        Addr: ":8080",
    }
    
    // 优雅关闭
    quit := make(chan os.Signal, 1)
    signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
    
    go func() {
        <-quit
        ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
        defer cancel()
        
        jobService.Shutdown(ctx)
        server.Shutdown(ctx)
    }()
    
    server.ListenAndServe()
}
```

---

## 最佳实践清单

- ✅ 使用构造器注入管理依赖
- ✅ 配置与代码分离
- ✅ 实现完整的错误处理和重试机制
- ✅ 集成日志和监控
- ✅ 支持多租户隔离
- ✅ 进行性能基准测试
- ✅ 实施熔断和限流策略
- ✅ 优雅关闭和资源清理
- ✅ 提供健康检查接口
- ✅ 文档化所有配置项

## 下一步

1. 查看完整示例代码: `pkg/priorityscheduler/examples/`
2. 运行基准测试: `go test -bench=. ./pkg/priorityscheduler/...`
3. 查看API文档: `pkg/priorityscheduler/README.md`

如有问题，请提交Issue或联系维护团队！
