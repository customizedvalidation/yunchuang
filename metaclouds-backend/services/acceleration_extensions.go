package services

import (
	"time"

	"metaclouds-backend/models"
	mcerrors "metaclouds-backend/pkg/errors"
)

// 本文件扩展 AccelerationService，添加 Dataset、FluidCache、
// DistributedTrainingConfig、InferenceConfig、Checkpoint 的 CRUD 与操作。

// --- Dataset CRUD ---

func (s *AccelerationService) ListDatasets(tenantID uint) ([]models.Dataset, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var datasets []models.Dataset
	for _, d := range s.db.Datasets {
		if tenantID > 0 && d.TenantID != tenantID {
			continue
		}
		datasets = append(datasets, *d)
	}
	return datasets, nil
}

func (s *AccelerationService) GetDataset(id uint) (*models.Dataset, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	dataset, exists := s.db.Datasets[id]
	if !exists {
		return nil, mcerrors.NotFound("dataset not found")
	}
	return dataset, nil
}

func (s *AccelerationService) CreateDataset(dataset *models.Dataset) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	for _, d := range s.db.Datasets {
		if d.Name == dataset.Name {
			return mcerrors.Conflict("dataset name already exists")
		}
	}

	dataset.ID = s.db.DatasetSeq
	dataset.CreatedAt = time.Now()
	dataset.UpdatedAt = time.Now()
	if dataset.Status == "" {
		dataset.Status = "active"
	}
	if dataset.AccessMode == "" {
		dataset.AccessMode = "ReadWriteMany"
	}
	s.db.Datasets[s.db.DatasetSeq] = dataset
	s.db.DatasetSeq++
	return nil
}

func (s *AccelerationService) UpdateDataset(dataset *models.Dataset) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	existing, exists := s.db.Datasets[dataset.ID]
	if !exists {
		return mcerrors.NotFound("dataset not found")
	}

	if dataset.Name != "" {
		existing.Name = dataset.Name
	}
	if dataset.Description != "" {
		existing.Description = dataset.Description
	}
	if dataset.SourceType != "" {
		existing.SourceType = dataset.SourceType
	}
	if dataset.SourcePath != "" {
		existing.SourcePath = dataset.SourcePath
	}
	if dataset.MountPath != "" {
		existing.MountPath = dataset.MountPath
	}
	if dataset.SizeGB >= 0 {
		existing.SizeGB = dataset.SizeGB
	}
	if dataset.MountOptions != "" {
		existing.MountOptions = dataset.MountOptions
	}
	if dataset.AccessMode != "" {
		existing.AccessMode = dataset.AccessMode
	}
	if dataset.Status != "" {
		existing.Status = dataset.Status
	}
	existing.UpdatedAt = time.Now()
	return nil
}

func (s *AccelerationService) DeleteDataset(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.Datasets[id]; !exists {
		return mcerrors.NotFound("dataset not found")
	}
	delete(s.db.Datasets, id)

	// 级联删除关联的 FluidCache
	for cacheID, cache := range s.db.FluidCaches {
		if cache.DatasetID == id {
			delete(s.db.FluidCaches, cacheID)
		}
	}
	return nil
}

// --- FluidCache CRUD ---

func (s *AccelerationService) ListFluidCaches(datasetID uint) ([]models.FluidCache, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var caches []models.FluidCache
	for _, c := range s.db.FluidCaches {
		if datasetID > 0 && c.DatasetID != datasetID {
			continue
		}
		caches = append(caches, *c)
	}
	return caches, nil
}

func (s *AccelerationService) GetFluidCache(id uint) (*models.FluidCache, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	cache, exists := s.db.FluidCaches[id]
	if !exists {
		return nil, mcerrors.NotFound("fluid cache not found")
	}
	return cache, nil
}

func (s *AccelerationService) CreateFluidCache(cache *models.FluidCache) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.Datasets[cache.DatasetID]; !exists {
		return mcerrors.NotFound("dataset not found")
	}

	cache.ID = s.db.FluidCacheSeq
	cache.CreatedAt = time.Now()
	cache.UpdatedAt = time.Now()
	if cache.RuntimeType == "" {
		cache.RuntimeType = "alluxio"
	}
	if cache.Replicas == 0 {
		cache.Replicas = 1
	}
	if cache.MediumType == "" {
		cache.MediumType = "memory"
	}
	if cache.Status == "" {
		cache.Status = "inactive"
	}
	s.db.FluidCaches[s.db.FluidCacheSeq] = cache
	s.db.FluidCacheSeq++
	return nil
}

func (s *AccelerationService) UpdateFluidCache(cache *models.FluidCache) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	existing, exists := s.db.FluidCaches[cache.ID]
	if !exists {
		return mcerrors.NotFound("fluid cache not found")
	}

	if cache.RuntimeType != "" {
		existing.RuntimeType = cache.RuntimeType
	}
	if cache.CacheCapacityGB >= 0 {
		existing.CacheCapacityGB = cache.CacheCapacityGB
	}
	if cache.Replicas > 0 {
		existing.Replicas = cache.Replicas
	}
	if cache.MediumType != "" {
		existing.MediumType = cache.MediumType
	}
	existing.PrefetchEnabled = cache.PrefetchEnabled
	if cache.PrefetchPolicy != "" {
		existing.PrefetchPolicy = cache.PrefetchPolicy
	}
	existing.CompressionEnabled = cache.CompressionEnabled
	existing.MetadataAccelerationEnabled = cache.MetadataAccelerationEnabled
	if cache.Status != "" {
		existing.Status = cache.Status
	}
	existing.UpdatedAt = time.Now()
	return nil
}

func (s *AccelerationService) DeleteFluidCache(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.FluidCaches[id]; !exists {
		return mcerrors.NotFound("fluid cache not found")
	}
	delete(s.db.FluidCaches, id)
	return nil
}

func (s *AccelerationService) EnableFluidCache(cacheID uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	cache, exists := s.db.FluidCaches[cacheID]
	if !exists {
		return mcerrors.NotFound("fluid cache not found")
	}
	cache.Status = "active"
	cache.UpdatedAt = time.Now()
	return nil
}

func (s *AccelerationService) DisableFluidCache(cacheID uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	cache, exists := s.db.FluidCaches[cacheID]
	if !exists {
		return mcerrors.NotFound("fluid cache not found")
	}
	cache.Status = "inactive"
	cache.UpdatedAt = time.Now()
	return nil
}

func (s *AccelerationService) TriggerPrefetch(cacheID uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	cache, exists := s.db.FluidCaches[cacheID]
	if !exists {
		return mcerrors.NotFound("fluid cache not found")
	}
	if !cache.PrefetchEnabled {
		return mcerrors.BadRequest("prefetch is not enabled for this cache")
	}
	// 模拟预取触发，更新命中率
	cache.CacheHitRate = 0.85
	cache.UpdatedAt = time.Now()
	return nil
}

// --- DistributedTrainingConfig CRUD ---

func (s *AccelerationService) GetDistributedTrainingConfig(jobID uint) (*models.DistributedTrainingConfig, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	for _, c := range s.db.DistributedTrainingConfigs {
		if c.JobID == jobID {
			return c, nil
		}
	}
	return nil, mcerrors.NotFound("distributed training config not found")
}

func (s *AccelerationService) CreateDistributedTrainingConfig(config *models.DistributedTrainingConfig) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	config.ID = s.db.DistributedTrainingConfigSeq
	config.CreatedAt = time.Now()
	config.UpdatedAt = time.Now()
	if config.ParallelStrategy == "" {
		config.ParallelStrategy = "data"
	}
	if config.WorldSize == 0 {
		config.WorldSize = 1
	}
	if config.CommunicationBackend == "" {
		config.CommunicationBackend = "nccl"
	}
	s.db.DistributedTrainingConfigs[s.db.DistributedTrainingConfigSeq] = config
	s.db.DistributedTrainingConfigSeq++
	return nil
}

func (s *AccelerationService) UpdateDistributedTrainingConfig(config *models.DistributedTrainingConfig) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	existing, exists := s.db.DistributedTrainingConfigs[config.ID]
	if !exists {
		return mcerrors.NotFound("distributed training config not found")
	}

	if config.ParallelStrategy != "" {
		existing.ParallelStrategy = config.ParallelStrategy
	}
	if config.WorldSize > 0 {
		existing.WorldSize = config.WorldSize
	}
	if config.TensorParallelSize > 0 {
		existing.TensorParallelSize = config.TensorParallelSize
	}
	if config.PipelineStages > 0 {
		existing.PipelineStages = config.PipelineStages
	}
	existing.DeepSpeedEnabled = config.DeepSpeedEnabled
	if config.DeepSpeedConfigPath != "" {
		existing.DeepSpeedConfigPath = config.DeepSpeedConfigPath
	}
	existing.MegatronEnabled = config.MegatronEnabled
	if config.NCCLConfig != "" {
		existing.NCCLConfig = config.NCCLConfig
	}
	existing.GradientCompression = config.GradientCompression
	if config.CommunicationBackend != "" {
		existing.CommunicationBackend = config.CommunicationBackend
	}
	existing.UpdatedAt = time.Now()
	return nil
}

func (s *AccelerationService) DeleteDistributedTrainingConfig(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.DistributedTrainingConfigs[id]; !exists {
		return mcerrors.NotFound("distributed training config not found")
	}
	delete(s.db.DistributedTrainingConfigs, id)
	return nil
}

// --- InferenceConfig CRUD ---

func (s *AccelerationService) GetInferenceConfig(jobID uint) (*models.InferenceConfig, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	for _, c := range s.db.InferenceConfigs {
		if c.JobID == jobID {
			return c, nil
		}
	}
	return nil, mcerrors.NotFound("inference config not found")
}

func (s *AccelerationService) CreateInferenceConfig(config *models.InferenceConfig) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	config.ID = s.db.InferenceConfigSeq
	config.CreatedAt = time.Now()
	config.UpdatedAt = time.Now()
	if config.Backend == "" {
		config.Backend = "pytorch"
	}
	if config.Precision == "" {
		config.Precision = "fp32"
	}
	if config.BatchSize == 0 {
		config.BatchSize = 1
	}
	if config.MaxBatchSize == 0 {
		config.MaxBatchSize = 32
	}
	if config.GPUUtilizationTarget == 0 {
		config.GPUUtilizationTarget = 0.8
	}
	s.db.InferenceConfigs[s.db.InferenceConfigSeq] = config
	s.db.InferenceConfigSeq++
	return nil
}

func (s *AccelerationService) UpdateInferenceConfig(config *models.InferenceConfig) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	existing, exists := s.db.InferenceConfigs[config.ID]
	if !exists {
		return mcerrors.NotFound("inference config not found")
	}

	if config.Backend != "" {
		existing.Backend = config.Backend
	}
	if config.Precision != "" {
		existing.Precision = config.Precision
	}
	existing.QuantizationEnabled = config.QuantizationEnabled
	if config.CalibrationDataset != "" {
		existing.CalibrationDataset = config.CalibrationDataset
	}
	if config.BatchSize > 0 {
		existing.BatchSize = config.BatchSize
	}
	existing.DynamicBatching = config.DynamicBatching
	if config.MaxBatchSize > 0 {
		existing.MaxBatchSize = config.MaxBatchSize
	}
	if config.MaxLatencyMs >= 0 {
		existing.MaxLatencyMs = config.MaxLatencyMs
	}
	if config.ModelPath != "" {
		existing.ModelPath = config.ModelPath
	}
	if config.TensorRTEnginePath != "" {
		existing.TensorRTEnginePath = config.TensorRTEnginePath
	}
	if config.GPUUtilizationTarget > 0 {
		existing.GPUUtilizationTarget = config.GPUUtilizationTarget
	}
	existing.UpdatedAt = time.Now()
	return nil
}

func (s *AccelerationService) DeleteInferenceConfig(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.InferenceConfigs[id]; !exists {
		return mcerrors.NotFound("inference config not found")
	}
	delete(s.db.InferenceConfigs, id)
	return nil
}

// --- Checkpoint CRUD ---

func (s *AccelerationService) ListCheckpoints(jobID uint) ([]models.Checkpoint, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var checkpoints []models.Checkpoint
	for _, c := range s.db.Checkpoints {
		if jobID > 0 && c.JobID != jobID {
			continue
		}
		checkpoints = append(checkpoints, *c)
	}
	return checkpoints, nil
}

func (s *AccelerationService) GetCheckpoint(id uint) (*models.Checkpoint, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	checkpoint, exists := s.db.Checkpoints[id]
	if !exists {
		return nil, mcerrors.NotFound("checkpoint not found")
	}
	return checkpoint, nil
}

func (s *AccelerationService) CreateCheckpoint(checkpoint *models.Checkpoint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	checkpoint.ID = s.db.CheckpointSeq
	checkpoint.CreatedAt = time.Now()
	checkpoint.UpdatedAt = time.Now()
	if checkpoint.Format == "" {
		checkpoint.Format = "pytorch"
	}
	if checkpoint.Status == "" {
		checkpoint.Status = "completed"
	}

	// 如果标记为 latest，取消其他同 job 的 latest
	if checkpoint.IsLatest {
		for _, c := range s.db.Checkpoints {
			if c.JobID == checkpoint.JobID {
				c.IsLatest = false
			}
		}
	}

	s.db.Checkpoints[s.db.CheckpointSeq] = checkpoint
	s.db.CheckpointSeq++
	return nil
}

func (s *AccelerationService) DeleteCheckpoint(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.Checkpoints[id]; !exists {
		return mcerrors.NotFound("checkpoint not found")
	}
	delete(s.db.Checkpoints, id)
	return nil
}

func (s *AccelerationService) GetLatestCheckpoint(jobID uint) (*models.Checkpoint, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var latest *models.Checkpoint
	for _, c := range s.db.Checkpoints {
		if c.JobID != jobID {
			continue
		}
		if c.IsLatest {
			return c, nil
		}
		if latest == nil || c.CreatedAt.After(latest.CreatedAt) {
			latest = c
		}
	}
	if latest == nil {
		return nil, mcerrors.NotFound("no checkpoint found for this job")
	}
	return latest, nil
}
