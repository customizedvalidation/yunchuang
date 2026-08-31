# Priority Scheduler Module

一个独立的、线程安全的优先级任务调度模块，支持高并发场景下的任务优先级管理。

## 特性

- ✅ 4级优先级调度（0-低, 1-中, 2-高, 3-紧急）
- ✅ 完全线程安全的并发操作
- ✅ 优先级变更通知机制
- ✅ 任务状态管理（pending/running/completed/failed/cancelled）
- ✅ 可配置的缓冲区大小
- ✅ 独立模块，易于集成到其他项目

## 快速开始

### 安装

```bash
go get github.com/your-username/priorityscheduler
```

### 基本用法

```go
package main

import (
    "fmt"
    "github.com/your-username/priorityscheduler"
)

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

func main() {
    scheduler := priorityscheduler.NewPriorityScheduler(nil)
    
    task := &MyTask{id: 1, priority: 2, status: string(priorityscheduler.StatusPending)}
    scheduler.AddTask(task)
    
    nextTask, _ := scheduler.GetNextTask()
    fmt.Printf("Next task: %d\n", nextTask.GetID())
    
    scheduler.UpdatePriority(1, 3)
    scheduler.CompleteTask(1)
}
```

## API 参考

### 创建调度器

```go
config := priorityscheduler.DefaultConfig()
config.BufferSize = 200
scheduler := priorityscheduler.NewPriorityScheduler(config)
```

### 任务管理

```go
// 添加任务
err := scheduler.AddTask(task)

// 获取任务
task, err := scheduler.GetTask(taskID)

// 删除任务
err := scheduler.RemoveTask(taskID)

// 获取所有任务
tasks := scheduler.GetTasks()
```

### 优先级操作

```go
// 更新优先级
err := scheduler.UpdatePriority(taskID, newPriority)

// 检查优先级有效性
valid := priorityscheduler.IsValidPriority(priority)

// 获取优先级级别名称
level := priorityscheduler.GetPriorityLevel(priority)
```

### 任务调度

```go
// 获取下一个待执行任务（按优先级）
task, err := scheduler.GetNextTask()

// 完成任务
err := scheduler.CompleteTask(taskID)

// 标记任务失败
err := scheduler.FailTask(taskID)

// 取消任务
err := scheduler.CancelTask(taskID)
```

### 优先级变更通知

```go
// 监听优先级变更
go func() {
    for change := range scheduler.GetPriorityChangeChannel() {
        fmt.Printf("Task %d priority changed: %d -> %d\n", 
            change.TaskID, change.OldPriority, change.NewPriority)
    }
}()
```

## 配置选项

| 配置项 | 类型 | 默认值 | 说明 |
|-------|------|-------|------|
| BufferSize | int | 100 | 内部通道缓冲区大小 |
| MaxConcurrentTasks | int | 10 | 最大并发任务数 |
| EnableNotifications | bool | true | 是否启用优先级变更通知 |

## 优先级级别

| 优先级值 | 级别名称 | 说明 |
|---------|---------|------|
| 0 | low | 低优先级 |
| 1 | medium | 中优先级 |
| 2 | high | 高优先级 |
| 3 | critical | 紧急优先级 |

## 任务状态

| 状态 | 说明 |
|-----|------|
| pending | 等待执行 |
| running | 执行中 |
| completed | 已完成 |
| failed | 执行失败 |
| cancelled | 已取消 |

## 测试

```bash
go test -v ./...
```

## 许可证

MIT License
