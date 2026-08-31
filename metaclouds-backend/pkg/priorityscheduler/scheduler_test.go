package priorityscheduler

import (
	"sync"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

type MockTask struct {
	id       uint
	priority int
	status   string
	mu       sync.Mutex
}

func (t *MockTask) GetID() uint {
	return t.id
}

func (t *MockTask) GetPriority() int {
	t.mu.Lock()
	defer t.mu.Unlock()
	return t.priority
}

func (t *MockTask) GetStatus() string {
	t.mu.Lock()
	defer t.mu.Unlock()
	return t.status
}

func (t *MockTask) SetPriority(priority int) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.priority = priority
}

func (t *MockTask) SetStatus(status string) {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.status = status
}

func TestPriorityScheduler_AddAndGetTask(t *testing.T) {
	scheduler := NewPriorityScheduler(DefaultConfig())

	task := &MockTask{id: 1, priority: 2, status: string(StatusPending)}
	err := scheduler.AddTask(task)
	assert.NoError(t, err)

	retrieved, err := scheduler.GetTask(1)
	assert.NoError(t, err)
	assert.Equal(t, uint(1), retrieved.GetID())
	assert.Equal(t, 2, retrieved.GetPriority())
}

func TestPriorityScheduler_InvalidPriority(t *testing.T) {
	scheduler := NewPriorityScheduler(DefaultConfig())

	task := &MockTask{id: 1, priority: 5, status: string(StatusPending)}
	err := scheduler.AddTask(task)
	assert.Error(t, err)
	assert.Equal(t, ErrInvalidPriority, err)
}

func TestPriorityScheduler_GetNextTask(t *testing.T) {
	scheduler := NewPriorityScheduler(DefaultConfig())

	tasks := []*MockTask{
		{id: 1, priority: 0, status: string(StatusPending)},
		{id: 2, priority: 3, status: string(StatusPending)},
		{id: 3, priority: 1, status: string(StatusPending)},
		{id: 4, priority: 2, status: string(StatusPending)},
	}

	for _, task := range tasks {
		scheduler.AddTask(task)
	}

	time.Sleep(100 * time.Millisecond)

	nextTask, err := scheduler.GetNextTask()
	assert.NoError(t, err)
	assert.Equal(t, uint(2), nextTask.GetID())
	assert.Equal(t, string(StatusRunning), nextTask.GetStatus())

	nextTask, err = scheduler.GetNextTask()
	assert.NoError(t, err)
	assert.Equal(t, uint(4), nextTask.GetID())
}

func TestPriorityScheduler_UpdatePriority(t *testing.T) {
	scheduler := NewPriorityScheduler(DefaultConfig())

	task := &MockTask{id: 1, priority: 0, status: string(StatusPending)}
	scheduler.AddTask(task)

	err := scheduler.UpdatePriority(1, 3)
	assert.NoError(t, err)
	assert.Equal(t, 3, task.GetPriority())

	err = scheduler.UpdatePriority(1, 3)
	assert.NoError(t, err)

	err = scheduler.UpdatePriority(1, 5)
	assert.Error(t, err)
	assert.Equal(t, ErrInvalidPriority, err)
}

func TestPriorityScheduler_RemoveTask(t *testing.T) {
	scheduler := NewPriorityScheduler(DefaultConfig())

	task := &MockTask{id: 1, priority: 1, status: string(StatusPending)}
	scheduler.AddTask(task)

	time.Sleep(100 * time.Millisecond)

	err := scheduler.RemoveTask(1)
	assert.NoError(t, err)

	time.Sleep(100 * time.Millisecond)

	_, err = scheduler.GetTask(1)
	assert.Error(t, err)
	assert.Equal(t, ErrTaskNotFound, err)
}

func TestPriorityScheduler_PriorityChangeNotification(t *testing.T) {
	scheduler := NewPriorityScheduler(DefaultConfig())

	task := &MockTask{id: 1, priority: 0, status: string(StatusPending)}
	scheduler.AddTask(task)

	go func() {
		time.Sleep(100 * time.Millisecond)
		scheduler.UpdatePriority(1, 2)
	}()

	select {
	case change := <-scheduler.GetPriorityChangeChannel():
		assert.Equal(t, uint(1), change.TaskID)
		assert.Equal(t, 0, change.OldPriority)
		assert.Equal(t, 2, change.NewPriority)
	case <-time.After(1 * time.Second):
		t.Error("Timeout waiting for priority change notification")
	}
}

func TestPriorityScheduler_ConcurrentOperations(t *testing.T) {
	scheduler := NewPriorityScheduler(DefaultConfig())

	var wg sync.WaitGroup
	numGoroutines := 100

	wg.Add(numGoroutines)
	for i := 0; i < numGoroutines; i++ {
		go func(idx int) {
			defer wg.Done()
			task := &MockTask{
				id:       uint(idx + 1),
				priority: idx % 4,
				status:   string(StatusPending),
			}
			scheduler.AddTask(task)
		}(i)
	}
	wg.Wait()

	tasks := scheduler.GetTasks()
	assert.Equal(t, numGoroutines, len(tasks))

	wg.Add(numGoroutines)
	for i := 0; i < numGoroutines; i++ {
		go func(idx int) {
			defer wg.Done()
			newPriority := (idx + 1) % 4
			scheduler.UpdatePriority(uint(idx+1), newPriority)
		}(i)
	}
	wg.Wait()
}

func TestPriorityScheduler_TaskCompletion(t *testing.T) {
	scheduler := NewPriorityScheduler(DefaultConfig())

	task := &MockTask{id: 1, priority: 3, status: string(StatusPending)}
	scheduler.AddTask(task)

	time.Sleep(100 * time.Millisecond)

	nextTask, err := scheduler.GetNextTask()
	assert.NoError(t, err)
	assert.Equal(t, string(StatusRunning), nextTask.GetStatus())

	err = scheduler.CompleteTask(1)
	assert.NoError(t, err)
	assert.Equal(t, string(StatusCompleted), task.GetStatus())
}

func TestPriorityScheduler_IsValidPriority(t *testing.T) {
	assert.True(t, IsValidPriority(0))
	assert.True(t, IsValidPriority(3))
	assert.False(t, IsValidPriority(-1))
	assert.False(t, IsValidPriority(4))
}

func TestPriorityScheduler_GetPriorityLevel(t *testing.T) {
	assert.Equal(t, "low", GetPriorityLevel(0))
	assert.Equal(t, "medium", GetPriorityLevel(1))
	assert.Equal(t, "high", GetPriorityLevel(2))
	assert.Equal(t, "critical", GetPriorityLevel(3))
	assert.Equal(t, "invalid", GetPriorityLevel(5))
}