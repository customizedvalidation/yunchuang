package services

import (
	"context"
	"sync"
	"time"

	"metaclouds-backend/pkg/logger"

	"github.com/robfig/cron/v3"
)

type Schedule struct {
	ID          uint      `json:"id"`
	Name        string    `json:"name"`
	CronExpr    string    `json:"cron_expr"`
	JobID       uint      `json:"job_id"`
	Enabled     bool      `json:"enabled"`
	LastRun     time.Time `json:"last_run"`
	NextRun     time.Time `json:"next_run"`
	LastSuccess time.Time `json:"last_success"`
	LastError   string    `json:"last_error"`
	RunCount    int       `json:"run_count"`
	ErrorCount  int       `json:"error_count"`
	CreatedAt   time.Time `json:"created_at"`
	UpdatedAt   time.Time `json:"updated_at"`
}

type Scheduler struct {
	cron          *cron.Cron
	jobService    *JobService
	k8sService    *K8SService
	jobs          map[cron.EntryID]Schedule
	mu            sync.RWMutex
	executingJobs sync.Map
	maxConcurrent int
	ctx           context.Context
	cancel        context.CancelFunc
	monitorWG     sync.WaitGroup
}

func NewScheduler(jobService *JobService, k8sService *K8SService) *Scheduler {
	ctx, cancel := context.WithCancel(context.Background())
	return &Scheduler{
		cron:          cron.New(cron.WithLogger(cron.DefaultLogger)),
		jobService:    jobService,
		k8sService:    k8sService,
		jobs:          make(map[cron.EntryID]Schedule),
		maxConcurrent: 10,
		ctx:           ctx,
		cancel:        cancel,
	}
}

func (s *Scheduler) Start() {
	logger.InfoWithCtx(context.Background(), "Starting job scheduler")
	s.cron.Start()
}

func (s *Scheduler) Stop() {
	logger.InfoWithCtx(context.Background(), "Stopping job scheduler")

	cronCtx := s.cron.Stop()
	<-cronCtx.Done()

	s.cancel()

	done := make(chan struct{})
	go func() {
		s.monitorWG.Wait()
		close(done)
	}()

	select {
	case <-done:
		logger.InfoWithCtx(context.Background(), "All monitoring goroutines stopped gracefully")
	case <-time.After(30 * time.Second):
		logger.WarnWithCtx(context.Background(), "Timeout waiting for monitoring goroutines to stop")
	}

	logger.InfoWithCtx(context.Background(), "Job scheduler stopped completely")
}

func (s *Scheduler) AddSchedule(name string, cronExpr string, jobID uint) (cron.EntryID, error) {
	s.mu.Lock()
	defer s.mu.Unlock()

	id, err := s.cron.AddFunc(cronExpr, func() {
		s.executeJob(jobID, name)
	})
	if err != nil {
		logger.ErrorWithCtx(context.Background(), "Failed to add schedule", err, "name", name)
		return 0, err
	}

	schedule := Schedule{
		ID:        uint(id),
		Name:      name,
		CronExpr:  cronExpr,
		JobID:     jobID,
		Enabled:   true,
		CreatedAt: time.Now(),
		UpdatedAt: time.Now(),
	}
	s.jobs[id] = schedule

	logger.InfoWithCtx(context.Background(), "Added schedule", "name", name, "job_id", jobID, "cron", cronExpr)
	return id, nil
}

func (s *Scheduler) RemoveSchedule(id cron.EntryID) {
	s.mu.Lock()
	defer s.mu.Unlock()

	s.cron.Remove(id)
	schedule, exists := s.jobs[id]
	if exists {
		logger.InfoWithCtx(context.Background(), "Removed schedule", "name", schedule.Name, "id", id)
		delete(s.jobs, id)
	}
}

func (s *Scheduler) executeJob(jobID uint, scheduleName string) {
	_, loaded := s.executingJobs.LoadOrStore(jobID, struct{}{})
	if loaded {
		logger.WarnWithCtx(context.Background(), "Job already executing, skipping", "job_id", jobID, "schedule", scheduleName)
		return
	}
	defer s.executingJobs.Delete(jobID)

	logger.InfoWithCtx(context.Background(), "Executing scheduled job", "schedule", scheduleName, "job_id", jobID)

	s.mu.Lock()
	entryID, schedule, exists := s.findScheduleEntryByJobIDLocked(jobID)
	if exists {
		schedule.RunCount++
		schedule.LastRun = time.Now()
		s.jobs[entryID] = schedule
	}
	s.mu.Unlock()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Minute)
	defer cancel()

	if err := s.executeJobWithRetry(ctx, jobID, scheduleName); err != nil {
		logger.ErrorWithCtx(ctx, "Scheduled job failed", err, "schedule", scheduleName, "job_id", jobID)

		s.mu.Lock()
		entryID, schedule, exists := s.findScheduleEntryByJobIDLocked(jobID)
		if exists {
			schedule.ErrorCount++
			schedule.LastError = err.Error()
			s.jobs[entryID] = schedule
		}
		s.mu.Unlock()
	} else {
		logger.InfoWithCtx(context.Background(), "Scheduled job completed", "schedule", scheduleName, "job_id", jobID)

		s.mu.Lock()
		entryID, schedule, exists := s.findScheduleEntryByJobIDLocked(jobID)
		if exists {
			schedule.LastSuccess = time.Now()
			schedule.LastError = ""
			s.jobs[entryID] = schedule
		}
		s.mu.Unlock()
	}
}

func (s *Scheduler) executeJobWithRetry(ctx context.Context, jobID uint, scheduleName string) error {
	maxRetries := 3
	var lastErr error

	for attempt := 1; attempt <= maxRetries; attempt++ {
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
		}

		if err := s.executeJobOnce(ctx, jobID, scheduleName); err != nil {
			lastErr = err
			logger.WarnWithCtx(ctx, "Job attempt failed", "schedule", scheduleName, "job_id", jobID, "attempt", attempt, "error", err)

			if attempt < maxRetries {
				backoff := time.Duration(attempt) * time.Second
				logger.InfoWithCtx(ctx, "Retrying job", "schedule", scheduleName, "job_id", jobID, "delay", backoff)
				time.Sleep(backoff)
			}
		} else {
			return nil
		}
	}

	return lastErr
}

func (s *Scheduler) executeJobOnce(ctx context.Context, jobID uint, scheduleName string) error {
	job, err := s.jobService.GetJob(jobID)
	if err != nil {
		return err
	}

	if job.Status != "pending" && job.Status != "completed" {
		logger.InfoWithCtx(ctx, "Job not in pending or completed state", "job_id", jobID, "status", job.Status)
		return nil
	}

	updateReq := UpdateJobRequest{
		Status: "running",
	}
	if _, err := s.jobService.UpdateJob(jobID, updateReq); err != nil {
		return err
	}

	if s.k8sService != nil {
		if _, err := s.k8sService.SubmitJob(SubmitJobRequest{JobID: jobID}); err != nil {
			rollbackReq := UpdateJobRequest{
				Status: "failed",
			}
			s.jobService.UpdateJob(jobID, rollbackReq)
			return err
		}
	}

	s.monitorWG.Add(1)
	go s.monitorJobCompletion(jobID, scheduleName)
	return nil
}

func (s *Scheduler) monitorJobCompletion(jobID uint, scheduleName string) {
	defer s.monitorWG.Done()

	monitorCtx, cancel := context.WithTimeout(s.ctx, 10*time.Minute)
	defer cancel()

	ticker := time.NewTicker(time.Second)
	defer ticker.Stop()

	for {
		select {
		case <-monitorCtx.Done():
			if monitorCtx.Err() == context.DeadlineExceeded {
				logger.WarnWithCtx(monitorCtx, "Job monitoring timeout", "job_id", jobID, "schedule", scheduleName)
			} else {
				logger.InfoWithCtx(monitorCtx, "Monitor job completion cancelled", "job_id", jobID)
			}
			return
		case <-ticker.C:
			job, err := s.jobService.GetJob(jobID)
			if err != nil {
				logger.ErrorWithCtx(monitorCtx, "Failed to get job status", err, "job_id", jobID)
				return
			}

			if job.Status == "completed" || job.Status == "failed" || job.Status == "cancelled" {
				logger.InfoWithCtx(monitorCtx, "Job reached final state", "schedule", scheduleName, "job_id", jobID, "status", job.Status)
				return
			}
		}
	}
}

func (s *Scheduler) findScheduleByJobID(jobID uint) (Schedule, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.findScheduleByJobIDLocked(jobID)
}

func (s *Scheduler) findScheduleByJobIDLocked(jobID uint) (Schedule, bool) {
	for _, schedule := range s.jobs {
		if schedule.JobID == jobID {
			return schedule, true
		}
	}
	return Schedule{}, false
}

func (s *Scheduler) findScheduleEntryByJobID(jobID uint) (cron.EntryID, Schedule, bool) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	return s.findScheduleEntryByJobIDLocked(jobID)
}

func (s *Scheduler) findScheduleEntryByJobIDLocked(jobID uint) (cron.EntryID, Schedule, bool) {
	for entryID, schedule := range s.jobs {
		if schedule.JobID == jobID {
			return entryID, schedule, true
		}
	}
	return 0, Schedule{}, false
}

func (s *Scheduler) GetSchedules() []Schedule {
	s.mu.RLock()
	defer s.mu.RUnlock()

	schedules := make([]Schedule, 0, len(s.jobs))
	for id, schedule := range s.jobs {
		entry := s.cron.Entry(id)
		schedule.NextRun = entry.Next
		schedule.LastRun = entry.Prev
		schedules = append(schedules, schedule)
	}
	return schedules
}

func (s *Scheduler) AddDefaultSchedules() {
	s.AddSchedule("sample-training", "*/30 * * * *", 1)
	s.AddSchedule("sample-inference", "0 */2 * * *", 2)
}
