package priorityscheduler

import "errors"

var (
	ErrNilTask         = errors.New("task cannot be nil")
	ErrInvalidPriority = errors.New("priority must be between 0 and 3")
	ErrTaskNotFound    = errors.New("task not found")
	ErrNoPendingTasks  = errors.New("no pending tasks available")
)