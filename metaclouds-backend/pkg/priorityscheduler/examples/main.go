package main

import (
	"fmt"
	"sync"
	"time"

	priorityscheduler "metaclouds-backend/pkg/priorityscheduler"
)

type Task struct {
	id          uint
	name        string
	priority    int
	status      string
	description string
	mu          sync.RWMutex
}

func (t *Task) GetID() uint {
	return t.id
}

func (t *Task) GetPriority() int {
	t.mu.RLock()
	defer t.mu.RUnlock()
	return t.priority
}

func (t *Task) GetStatus() string {
	t.mu.RLock()
	defer t.mu.RUnlock()
	return t.status
}

func (t *Task) SetPriority(priority int) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.priority = priority
}

func (t *Task) SetStatus(status string) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.status = status
}

func main() {
	fmt.Println("╔═══════════════════════════════════════════════════════════════╗")
	fmt.Println("║       优先级调度器完整使用示例 (Priority Scheduler Demo)        ║")
	fmt.Println("╚═══════════════════════════════════════════════════════════════╝")
	fmt.Println()

	scheduler := priorityscheduler.NewPriorityScheduler(priorityscheduler.DefaultConfig())

	go monitorPriorityChanges(scheduler)

	runBasicExample(scheduler)

	runConcurrentExample()

	fmt.Println("\n✅ 所有示例运行完成！")
}

func runBasicExample(scheduler *priorityscheduler.PriorityScheduler) {
	fmt.Println("【示例1】基础使用 - 添加和调度任务")
	fmt.Println("───────────────────────────────────────────────────────────────")

	tasks := []struct {
		id          uint
		name        string
		priority    int
		description string
	}{
		{1, "数据备份", 0, "低优先级后台备份任务"},
		{2, "日志分析", 1, "中优先级日志统计任务"},
		{3, "模型训练", 2, "高优先级AI模型训练"},
		{4, "紧急故障处理", 3, "紧急优先级故障响应"},
		{5, "性能监控", 1, "中优先级性能监控"},
		{6, "用户通知", 2, "高优先级用户消息推送"},
	}

	fmt.Println("\n1. 添加任务到调度器：")
	for _, t := range tasks {
		task := &Task{
			id:          t.id,
			name:        t.name,
			priority:    t.priority,
			status:      string(priorityscheduler.StatusPending),
			description: t.description,
		}

		err := scheduler.AddTask(task)
		if err != nil {
			fmt.Printf("   ❌ 添加失败: %v\n", err)
		} else {
			level := priorityscheduler.GetPriorityLevel(t.priority)
			fmt.Printf("   ✅ 添加: [ID=%d] %s - 优先级: %s (%d)\n",
				t.id, t.name, level, t.priority)
		}
	}

	time.Sleep(200 * time.Millisecond)

	fmt.Println("\n2. 按优先级顺序调度任务：")
	scheduled := 0
	for {
		task, err := scheduler.GetNextTask()
		if err != nil {
			if err == priorityscheduler.ErrNoPendingTasks {
				fmt.Printf("   ✅ 调度完成，共 %d 个任务\n", scheduled)
				break
			}
			break
		}

		scheduled++
		level := priorityscheduler.GetPriorityLevel(task.GetPriority())
		fmt.Printf("   📋 [%d] 调度: ID=%d, 优先级: %s, 状态: %s\n",
			scheduled, task.GetID(), level, task.GetStatus())
	}

	fmt.Println("\n3. 模拟任务执行并完成：")
	for _, t := range tasks {
		time.Sleep(50 * time.Millisecond)
		err := scheduler.CompleteTask(t.id)
		if err == nil {
			fmt.Printf("   ✅ 完成任务: ID=%d (%s)\n", t.id, t.name)
		}
	}

	fmt.Println("\n4. 测试优先级变更：")
	task, _ := scheduler.GetTask(1)
	if task != nil {
		oldPriority := task.GetPriority()
		fmt.Printf("   当前任务 #1 优先级: %d (%s)\n",
			oldPriority, priorityscheduler.GetPriorityLevel(oldPriority))

		fmt.Println("\n   执行优先级变更：")
		changes := []struct {
			newPriority int
			desc       string
		}{
			{3, "提升到紧急"},
			{2, "降低到高"},
			{0, "降低到低"},
			{1, "提升到中"},
		}

		for _, c := range changes {
			err := scheduler.UpdatePriority(1, c.newPriority)
			if err == nil {
				level := priorityscheduler.GetPriorityLevel(c.newPriority)
				fmt.Printf("   ✅ %s: 优先级=%d (%s)\n", c.desc, c.newPriority, level)
				time.Sleep(150 * time.Millisecond)
			}
		}
	}
}

func runConcurrentExample() {
	fmt.Println("\n\n【示例2】高并发场景 - 并发添加和更新任务")
	fmt.Println("───────────────────────────────────────────────────────────────")

	scheduler := priorityscheduler.NewPriorityScheduler(priorityscheduler.DefaultConfig())

	var wg sync.WaitGroup
	numGoroutines := 30

	fmt.Printf("\n1. 启动 %d 个并发goroutine...\n", numGoroutines)
	wg.Add(numGoroutines)

	startTime := time.Now()

	for i := 0; i < numGoroutines; i++ {
		go func(idx int) {
			defer wg.Done()

			task := &Task{
				id:       uint(idx + 100),
				name:     fmt.Sprintf("并发任务-%d", idx),
				priority: idx % 4,
				status:   string(priorityscheduler.StatusPending),
			}

			scheduler.AddTask(task)

			time.Sleep(10 * time.Millisecond)

			if idx%2 == 0 {
				scheduler.UpdatePriority(uint(idx+100), (idx+1)%4)
			}

			if idx%3 == 0 {
				scheduler.GetNextTask()
				time.Sleep(20 * time.Millisecond)
				scheduler.CompleteTask(uint(idx + 100))
			}
		}(i)
	}

	wg.Wait()
	duration := time.Since(startTime)

	time.Sleep(200 * time.Millisecond)

	fmt.Printf("\n2. 并发操作统计：")
	fmt.Printf("\n   ⏱️  执行时间: %v\n", duration)
	fmt.Printf("   📊 并发数: %d goroutines\n", numGoroutines)

	allTasks := scheduler.GetTasks()
	fmt.Printf("   ✅ 总任务数: %d\n", len(allTasks))

	queue := scheduler.GetPriorityQueue()
	stats := make(map[int]int)
	for p := 0; p < 4; p++ {
		stats[p] = len(queue[p])
	}

	fmt.Printf("\n3. 优先级队列分布：\n")
	for p := 3; p >= 0; p-- {
		level := priorityscheduler.GetPriorityLevel(p)
		count := stats[p]
		percentage := float64(count) / float64(numGoroutines) * 100
		fmt.Printf("   P%d (%s): %2d 个任务 (%.1f%%)\n", p, level, count, percentage)
	}

	runningCount := 0
	completedCount := 0
	for _, task := range allTasks {
		if task.GetStatus() == string(priorityscheduler.StatusRunning) {
			runningCount++
		} else if task.GetStatus() == string(priorityscheduler.StatusCompleted) {
			completedCount++
		}
	}

	fmt.Printf("\n4. 任务状态分布：\n")
	fmt.Printf("   🔄 Running: %d\n", runningCount)
	fmt.Printf("   ✅ Completed: %d\n", completedCount)
	fmt.Printf("   ⏳ Pending: %d\n", len(allTasks)-runningCount-completedCount)
}

func monitorPriorityChanges(scheduler *priorityscheduler.PriorityScheduler) {
	fmt.Println("\n🔔 优先级变更监控已启动，等待变更通知...")
	fmt.Println()

	for change := range scheduler.GetPriorityChangeChannel() {
		oldLevel := priorityscheduler.GetPriorityLevel(change.OldPriority)
		newLevel := priorityscheduler.GetPriorityLevel(change.NewPriority)

		fmt.Printf("   🔄 [通知] 任务 #%d 优先级变更: %s(P%d) → %s(P%d) @ %s\n",
			change.TaskID,
			oldLevel, change.OldPriority,
			newLevel, change.NewPriority,
			change.Timestamp.Format("15:04:05.000"))
	}
}
