package priorityscheduler

import "time"

const (
	PriorityLow      = 0
	PriorityMedium   = 1
	PriorityHigh     = 2
	PriorityCritical = 3

	MaxPriority   = PriorityCritical
	MinPriority   = PriorityLow
	DefaultBuffer = 100
)

type Task interface {
	GetID() uint
	GetPriority() int
	GetStatus() string
	SetPriority(priority int)
	SetStatus(status string)
}

type TaskStatus string

const (
	StatusPending    TaskStatus = "pending"
	StatusRunning    TaskStatus = "running"
	StatusCompleted  TaskStatus = "completed"
	StatusFailed     TaskStatus = "failed"
	StatusCancelled  TaskStatus = "cancelled"
)

type PriorityChange struct {
	TaskID   uint
	OldPriority int
	NewPriority int
	Timestamp  time.Time
}

type SchedulerConfig struct {
	BufferSize          int
	MaxConcurrentTasks  int
	EnableNotifications bool
}

func DefaultConfig() *SchedulerConfig {
	return &SchedulerConfig{
		BufferSize:          DefaultBuffer,
		MaxConcurrentTasks: 10,
		EnableNotifications: true,
	}
}