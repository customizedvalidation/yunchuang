# 优先级调度器 - 快速参考

## 🔧 核心配置项

```go
type SchedulerConfig struct {
    BufferSize          int    // Channel缓冲区大小 (默认: 100)
    MaxConcurrentTasks  int    // 最大并发任务数 (默认: 10)
    EnableNotifications bool   // 启用优先级变更通知 (默认: true)
}
```

## 📊 环境配置推荐

| 环境 | BufferSize | MaxConcurrent | 适用场景 |
|------|-----------|--------------|---------|
| 开发 | 50-100 | 5-10 | 本地调试 |
| 测试 | 100-200 | 10-20 | CI/CD |
| 生产 | 200-500 | 20-50 | 常规业务 |
| 高并发 | 500-1000 | 50-100 | 大规模任务 |

## 🚀 快速集成步骤

### 1. 添加依赖
```go
import "metaclouds-backend/pkg/priorityscheduler"
```

### 2. 创建调度器
```go
config := priorityscheduler.DefaultConfig()
scheduler := priorityscheduler.NewPriorityScheduler(config)
```

### 3. 实现Task接口
```go
type MyTask struct {
    id       uint
    priority int
    status   string
}

func (t *MyTask) GetID() uint           { return t.id }
func (t *MyTask) GetPriority() int      { return t.priority }
func (t *MyTask) GetStatus() string     { return t.status }
func (t *MyTask) SetPriority(p int)     { t.priority = p }
func (t *MyTask) SetStatus(s string)    { t.status = s }
```

### 4. 基本操作

```go
// 添加任务
scheduler.AddTask(&MyTask{...})

// 获取下一个任务（按优先级）
task, _ := scheduler.GetNextTask()

// 更新优先级
scheduler.UpdatePriority(taskID, newPriority)

// 完成任务
scheduler.CompleteTask(taskID)

// 获取所有任务
tasks := scheduler.GetTasks()
```

### 5. 监听优先级变更
```go
go func() {
    for change := range scheduler.GetPriorityChangeChannel() {
        fmt.Printf("Task %d: P%d → P%d\n", 
            change.TaskID, change.OldPriority, change.NewPriority)
    }
}()
```

## ⚡ 性能调优

### 缓冲区大小计算
```
建议 = 预期并发任务数 × 2
```

### 高并发场景
```go
config := &SchedulerConfig{
    BufferSize:          500,
    MaxConcurrentTasks:  50,
    EnableNotifications: true,
}
```

### 低延迟场景
```go
config := &SchedulerConfig{
    BufferSize:          50,
    MaxConcurrentTasks:  20,
    EnableNotifications: false, // 关闭通知以提升性能
}
```

## 🔍 错误处理

```go
import "metaclouds-backend/pkg/priorityscheduler"

switch err {
case priorityscheduler.ErrNilTask:
    // 处理空任务错误
case priorityscheduler.ErrInvalidPriority:
    // 处理无效优先级错误
case priorityscheduler.ErrTaskNotFound:
    // 处理任务不存在错误
case priorityscheduler.ErrNoPendingTasks:
    // 处理无待处理任务错误
}
```

## 📈 监控指标

| 指标 | 类型 | 说明 |
|-----|------|------|
| `scheduler_tasks_total` | Gauge | 任务总数（按优先级） |
| `scheduler_priority_changes` | Counter | 优先级变更次数 |
| `scheduler_queue_depth` | Gauge | 队列深度 |

## 🛡️ 多租户隔离

```go
type TenantScheduler struct {
    schedulers map[string]*PriorityScheduler
    mu         sync.RWMutex
}

func (t *TenantScheduler) Get(tenantID string) *PriorityScheduler {
    t.mu.RLock()
    defer t.mu.RUnlock()
    return t.schedulers[tenantID]
}
```

## 🔄 与现有系统集成

### JobService集成
```go
type JobService struct {
    db        *MemoryStore
    scheduler *priorityscheduler.PriorityScheduler
}

func (s *JobService) CreateJob(job *Job) error {
    s.db.Jobs[job.ID] = job
    return s.scheduler.AddTask(job)
}
```

### 中间件集成
```go
func PriorityMiddleware(scheduler *PriorityScheduler) gin.HandlerFunc {
    return func(c *gin.Context) {
        // 请求前：记录优先级变更
        // ...
        c.Next()
        // 请求后：更新任务状态
    }
}
```

## 📝 日志集成

```go
import "metaclouds-backend/pkg/logger"

func logPriorityChanges(scheduler *PriorityScheduler) {
    for change := range scheduler.GetPriorityChangeChannel() {
        logger.Info("Priority changed",
            "task_id", change.TaskID,
            "from", change.OldPriority,
            "to", change.NewPriority,
        )
    }
}
```

## 🔧 常见问题

### Q: 如何避免优先级饥饿？
A: 使用多级反馈队列，定期重新评估优先级

### Q: 如何处理大量任务？
A: 使用批量操作和连接池

### Q: 如何保证通知不丢失？
A: 使用足够的BufferSize或实现持久化队列

### Q: 如何在重启后恢复状态？
A: 实现状态持久化和恢复机制

## 📚 相关资源

- [完整集成指南](./INTEGRATION.md)
- [API文档](./README.md)
- [示例代码](./examples/)
- [基准测试](./benchmark_test.go)

## ✅ 检查清单

- [ ] 确认BufferSize足够处理峰值负载
- [ ] 实现错误处理和重试机制
- [ ] 添加日志和监控
- [ ] 配置健康检查
- [ ] 实现优雅关闭
- [ ] 测试多租户隔离
- [ ] 性能基准测试
- [ ] 文档化配置项
