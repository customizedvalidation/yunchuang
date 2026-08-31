package priorityscheduler

import (
	"fmt"
	"sync"
	"time"
)

type ExampleTask struct {
	id          uint
	name        string
	priority    int
	status      string
	description string
	mu          sync.RWMutex
}

func (t *ExampleTask) GetID() uint {
	return t.id
}

func (t *ExampleTask) GetPriority() int {
	t.mu.Lock()
	defer t.mu.Unlock()
	return t.priority
}

func (t *ExampleTask) GetStatus() string {
	t.mu.Lock()
	defer t.mu.Unlock()
	return t.status
}

func (t *ExampleTask) SetPriority(priority int) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.priority = priority
}

func (t *ExampleTask) SetStatus(status string) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.status = status
}

func RunCompleteExample() {
	fmt.Println("=" + string(rune('=')) + " Priority Scheduler Complete Example " + string(rune('=')))
	fmt.Println()

	scheduler := NewPriorityScheduler(DefaultConfig())

	go monitorPriorityChanges(scheduler)

	fmt.Println("1. 添加任务...")
	addTasks(scheduler)

	fmt.Println("\n2. 获取下一个任务（应该是最高优先级）...")
	scheduleTasks(scheduler)

	fmt.Println("\n3. 模拟任务执行...")
	simulateTaskExecution(scheduler)

	fmt.Println("\n4. 测试优先级变更...")
	testPriorityChanges(scheduler)

	fmt.Println("\n5. 测试并发操作...")
	testConcurrentOperations()

	fmt.Println("\n6. 清理完成...")
	fmt.Println("示例运行结束！")
}

func addTasks(scheduler *PriorityScheduler) {
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

	for _, t := range tasks {
		task := &ExampleTask{
			id:          t.id,
			name:        t.name,
			priority:    t.priority,
			status:      string(StatusPending),
			description: t.description,
		}

		err := scheduler.AddTask(task)
		if err != nil {
			fmt.Printf("  ❌ 添加任务失败: %v\n", err)
		} else {
			level := GetPriorityLevel(t.priority)
			fmt.Printf("  ✅ 添加任务: ID=%d, 名称=%s, 优先级=%s (%d)\n",
				t.id, t.name, level, t.priority)
		}
	}

	time.Sleep(200 * time.Millisecond)

	fmt.Println("\n  当前队列状态:")
	queue := scheduler.GetPriorityQueue()
	for i := 3; i >= 0; i-- {
		if len(queue[i]) > 0 {
			fmt.Printf("    优先级 %d (%s): %d 个任务\n",
				i, GetPriorityLevel(i), len(queue[i]))
		}
	}
}

func scheduleTasks(scheduler *PriorityScheduler) {
	count := 0
	for {
		task, err := scheduler.GetNextTask()
		if err != nil {
			if err == ErrNoPendingTasks {
				fmt.Printf("  ✅ 所有待处理任务已调度完成\n")
				break
			}
			fmt.Printf("  ❌ 调度失败: %v\n", err)
			break
		}

		count++
		fmt.Printf("  📋 调度任务 #%d: ID=%d, 名称=%s, 状态=%s\n",
			count, task.GetID(), "Task", task.GetStatus())
	}
}

func simulateTaskExecution(scheduler *PriorityScheduler) {
	tasks := scheduler.GetTasks()
	completed := 0

	for _, task := range tasks {
		if task.GetStatus() == string(StatusRunning) {
			time.Sleep(100 * time.Millisecond)

			err := scheduler.CompleteTask(task.GetID())
			if err == nil {
				completed++
				fmt.Printf("  ✅ 完成任务: ID=%d, 状态=%s\n",
					task.GetID(), task.GetStatus())
			}
		}
	}

	fmt.Printf("  📊 总共完成任务: %d/%d\n", completed, len(tasks))
}

func testPriorityChanges(scheduler *PriorityScheduler) {
	task, err := scheduler.GetTask(1)
	if err != nil {
		fmt.Printf("  ❌ 获取任务失败: %v\n", err)
		return
	}

	oldPriority := task.GetPriority()
	fmt.Printf("  当前任务 #1 优先级: %d (%s)\n", oldPriority, GetPriorityLevel(oldPriority))

	fmt.Println("\n  测试优先级变更...")

	changes := []struct {
		newPriority int
		description string
	}{
		{3, "提升到紧急优先级"},
		{2, "降低到高优先级"},
		{0, "降低到低优先级"},
	}

	for _, change := range changes {
		err := scheduler.UpdatePriority(1, change.newPriority)
		if err != nil {
			fmt.Printf("  ❌ 优先级变更失败: %v\n", err)
		} else {
			fmt.Printf("  ✅ %s: %d (%s)\n",
				change.description, change.newPriority, GetPriorityLevel(change.newPriority))
			time.Sleep(150 * time.Millisecond)
		}
	}
}

func testConcurrentOperations() {
	fmt.Println("  启动并发测试...")

	scheduler := NewPriorityScheduler(DefaultConfig())
	var wg sync.WaitGroup

	numGoroutines := 20
	wg.Add(numGoroutines)

	for i := 0; i < numGoroutines; i++ {
		go func(idx int) {
			defer wg.Done()

			task := &ExampleTask{
				id:       uint(idx + 100),
				name:     fmt.Sprintf("并发任务-%d", idx),
				priority: idx % 4,
				status:   string(StatusPending),
			}

			scheduler.AddTask(task)

			time.Sleep(50 * time.Millisecond)

			if idx%3 == 0 {
				scheduler.UpdatePriority(uint(idx+100), (idx+1)%4)
			}
		}(i)
	}

	wg.Wait()

	time.Sleep(200 * time.Millisecond)

	tasks := scheduler.GetTasks()
	fmt.Printf("  ✅ 成功创建 %d 个任务\n", len(tasks))

	queue := scheduler.GetPriorityQueue()
	stats := make(map[int]int)
	for i := 0; i < 4; i++ {
		stats[i] = len(queue[i])
	}

	fmt.Printf("  📊 优先级分布: P0=%d, P1=%d, P2=%d, P3=%d\n",
		stats[0], stats[1], stats[2], stats[3])
}

func monitorPriorityChanges(scheduler *PriorityScheduler) {
	fmt.Println("  🔔 优先级变更监控已启动...")

	for change := range scheduler.GetPriorityChangeChannel() {
		fmt.Printf("  🔄 任务 #%d 优先级变更: %d (%s) → %d (%s) @ %s\n",
			change.TaskID,
			change.OldPriority, GetPriorityLevel(change.OldPriority),
			change.NewPriority, GetPriorityLevel(change.NewPriority),
			change.Timestamp.Format("15:04:05"))
	}
}

func ExamplePriorityScheduler() {
	scheduler := NewPriorityScheduler(DefaultConfig())

	go func() {
		for change := range scheduler.GetPriorityChangeChannel() {
			fmt.Printf("变更: Task #%d 从 P%d 变更为 P%d\n",
				change.TaskID, change.OldPriority, change.NewPriority)
		}
	}()

	task1 := &ExampleTask{id: 1, name: "Task1", priority: 1, status: string(StatusPending)}
	task2 := &ExampleTask{id: 2, name: "Task2", priority: 3, status: string(StatusPending)}
	task3 := &ExampleTask{id: 3, name: "Task3", priority: 0, status: string(StatusPending)}

	scheduler.AddTask(task1)
	scheduler.AddTask(task2)
	scheduler.AddTask(task3)

	time.Sleep(100 * time.Millisecond)

	fmt.Println("\n=== 按优先级顺序调度 ===")
	for i := 0; i < 3; i++ {
		task, err := scheduler.GetNextTask()
		if err != nil {
			break
		}
		fmt.Printf("调度: Task #%d (优先级: %d)\n", task.GetID(), task.GetPriority())

		time.Sleep(50 * time.Millisecond)
		scheduler.CompleteTask(task.GetID())
	}

	fmt.Println("\n=== 更新优先级 ===")
	scheduler.UpdatePriority(1, 3)
	time.Sleep(50 * time.Millisecond)
	scheduler.UpdatePriority(3, 0)

	time.Sleep(100 * time.Millisecond)

	fmt.Println("\n=== 最终队列状态 ===")
	queue := scheduler.GetPriorityQueue()
	for p := 3; p >= 0; p-- {
		if len(queue[p]) > 0 {
			fmt.Printf("优先级 %d: %v\n", p, queue[p])
		}
	}
}
