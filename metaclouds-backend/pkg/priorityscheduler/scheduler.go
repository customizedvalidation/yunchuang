package priorityscheduler

import (
	"sync"
	"time"
)

type PriorityScheduler struct {
	tasks               map[uint]Task
	priorityQueue       [4][]uint
	priorityChanged     chan PriorityChange
	config              *SchedulerConfig
	mu                  sync.RWMutex
	runningTasks        sync.Map
	taskAdded           chan uint
	taskRemoved         chan uint
}

func NewPriorityScheduler(config *SchedulerConfig) *PriorityScheduler {
	if config == nil {
		config = DefaultConfig()
	}

	s := &PriorityScheduler{
		tasks:           make(map[uint]Task),
		priorityQueue:   [4][]uint{},
		config:          config,
		taskAdded:       make(chan uint, config.BufferSize),
		taskRemoved:     make(chan uint, config.BufferSize),
	}

	if config.EnableNotifications {
		s.priorityChanged = make(chan PriorityChange, config.BufferSize)
	}

	go s.processUpdates()

	return s
}

func (s *PriorityScheduler) processUpdates() {
	for {
		select {
		case taskID := <-s.taskAdded:
			s.mu.Lock()
			task, exists := s.tasks[taskID]
			if exists {
				s.updatePriorityQueue(task)
			}
			s.mu.Unlock()
		case taskID := <-s.taskRemoved:
			s.mu.Lock()
			task, exists := s.tasks[taskID]
			if exists {
				s.removeFromPriorityQueue(task)
				delete(s.tasks, taskID)
			}
			s.mu.Unlock()
		default:
			time.Sleep(time.Millisecond * 50)
		}
	}
}

func (s *PriorityScheduler) AddTask(task Task) error {
	if task == nil {
		return ErrNilTask
	}

	if !IsValidPriority(task.GetPriority()) {
		return ErrInvalidPriority
	}

	s.mu.Lock()
	s.tasks[task.GetID()] = task
	s.mu.Unlock()

	select {
	case s.taskAdded <- task.GetID():
	default:
		s.mu.Lock()
		s.updatePriorityQueue(task)
		s.mu.Unlock()
	}

	return nil
}

func (s *PriorityScheduler) RemoveTask(taskID uint) error {
	s.mu.RLock()
	_, exists := s.tasks[taskID]
	s.mu.RUnlock()

	if !exists {
		return ErrTaskNotFound
	}

	select {
	case s.taskRemoved <- taskID:
	default:
		s.mu.Lock()
		task, exists := s.tasks[taskID]
		if exists {
			s.removeFromPriorityQueue(task)
			delete(s.tasks, taskID)
		}
		s.mu.Unlock()
	}

	return nil
}

func (s *PriorityScheduler) GetTask(taskID uint) (Task, error) {
	s.mu.RLock()
	task, exists := s.tasks[taskID]
	s.mu.RUnlock()

	if !exists {
		return nil, ErrTaskNotFound
	}

	return task, nil
}

func (s *PriorityScheduler) GetTasks() []Task {
	s.mu.RLock()
	tasks := make([]Task, 0, len(s.tasks))
	for _, task := range s.tasks {
		tasks = append(tasks, task)
	}
	s.mu.RUnlock()

	return tasks
}

func (s *PriorityScheduler) UpdatePriority(taskID uint, newPriority int) error {
	if !IsValidPriority(newPriority) {
		return ErrInvalidPriority
	}

	s.mu.Lock()
	task, exists := s.tasks[taskID]
	if !exists {
		s.mu.Unlock()
		return ErrTaskNotFound
	}

	oldPriority := task.GetPriority()
	if oldPriority == newPriority {
		s.mu.Unlock()
		return nil
	}

	s.removeFromPriorityQueue(task)
	task.SetPriority(newPriority)
	s.updatePriorityQueue(task)

	if s.config.EnableNotifications {
		select {
		case s.priorityChanged <- PriorityChange{
			TaskID:      taskID,
			OldPriority: oldPriority,
			NewPriority: newPriority,
			Timestamp:   time.Now(),
		}:
		default:
		}
	}

	s.mu.Unlock()
	return nil
}

func (s *PriorityScheduler) GetNextTask() (Task, error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	for priority := MaxPriority; priority >= MinPriority; priority-- {
		for len(s.priorityQueue[priority]) > 0 {
			taskID := s.priorityQueue[priority][0]
			s.priorityQueue[priority] = s.priorityQueue[priority][1:]

			task, exists := s.tasks[taskID]
			if !exists {
				continue
			}

			if task.GetStatus() == string(StatusPending) {
				_, loaded := s.runningTasks.LoadOrStore(taskID, struct{}{})
				if loaded {
					continue
				}

				task.SetStatus(string(StatusRunning))
				return task, nil
			}
		}
	}

	return nil, ErrNoPendingTasks
}

func (s *PriorityScheduler) CompleteTask(taskID uint) error {
	s.mu.Lock()
	task, exists := s.tasks[taskID]
	s.mu.Unlock()

	if !exists {
		return ErrTaskNotFound
	}

	s.runningTasks.Delete(taskID)
	task.SetStatus(string(StatusCompleted))

	return nil
}

func (s *PriorityScheduler) FailTask(taskID uint) error {
	s.mu.Lock()
	task, exists := s.tasks[taskID]
	s.mu.Unlock()

	if !exists {
		return ErrTaskNotFound
	}

	s.runningTasks.Delete(taskID)
	task.SetStatus(string(StatusFailed))

	return nil
}

func (s *PriorityScheduler) CancelTask(taskID uint) error {
	s.mu.Lock()
	task, exists := s.tasks[taskID]
	s.mu.Unlock()

	if !exists {
		return ErrTaskNotFound
	}

	s.runningTasks.Delete(taskID)
	task.SetStatus(string(StatusCancelled))

	return nil
}

func (s *PriorityScheduler) GetPendingTasksByPriority(priority int) []Task {
	if !IsValidPriority(priority) {
		return nil
	}

	s.mu.RLock()
	defer s.mu.RUnlock()

	tasks := []Task{}
	for _, taskID := range s.priorityQueue[priority] {
		task, exists := s.tasks[taskID]
		if exists && task.GetStatus() == string(StatusPending) {
			tasks = append(tasks, task)
		}
	}

	return tasks
}

func (s *PriorityScheduler) GetPriorityQueue() [][]uint {
	s.mu.RLock()
	defer s.mu.RUnlock()

	result := make([][]uint, 4)
	for i := range result {
		result[i] = make([]uint, len(s.priorityQueue[i]))
		copy(result[i], s.priorityQueue[i])
	}

	return result
}

func (s *PriorityScheduler) GetPriorityChangeChannel() <-chan PriorityChange {
	return s.priorityChanged
}

func (s *PriorityScheduler) updatePriorityQueue(task Task) {
	priority := task.GetPriority()
	if priority < MinPriority || priority > MaxPriority {
		return
	}

	for _, id := range s.priorityQueue[priority] {
		if id == task.GetID() {
			return
		}
	}

	s.priorityQueue[priority] = append(s.priorityQueue[priority], task.GetID())
}

func (s *PriorityScheduler) removeFromPriorityQueue(task Task) {
	priority := task.GetPriority()
	if priority < MinPriority || priority > MaxPriority {
		return
	}

	queue := s.priorityQueue[priority]
	for i, id := range queue {
		if id == task.GetID() {
			s.priorityQueue[priority] = append(queue[:i], queue[i+1:]...)
			return
		}
	}
}

func IsValidPriority(priority int) bool {
	return priority >= MinPriority && priority <= MaxPriority
}

func GetPriorityLevel(priority int) string {
	switch priority {
	case PriorityCritical:
		return "critical"
	case PriorityHigh:
		return "high"
	case PriorityMedium:
		return "medium"
	case PriorityLow:
		return "low"
	default:
		return "invalid"
	}
}