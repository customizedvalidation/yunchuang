package services

import (
	"context"
	"fmt"
	"time"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
	mcerrors "metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/logger"
)

// GPUService GPU 细粒度设备与分配服务
type GPUService struct {
	db     *models.MemoryStore
	config *config.Config
}

func NewGPUService(db interface{}, config *config.Config) *GPUService {
	memoryStore, err := models.GetDBStore(db, "GPUService")
	if err != nil {
		logger.ErrorWithCtx(context.Background(), "Failed to initialize GPUService", err)
		return nil
	}
	return &GPUService{
		db:     memoryStore,
		config: config,
	}
}

// --- GPUDevice CRUD ---

func (s *GPUService) ListGPUDevices(clusterID uint, vendor string, status string) ([]models.GPUDevice, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var devices []models.GPUDevice
	for _, d := range s.db.GPUDevices {
		if clusterID > 0 && d.ClusterID != clusterID {
			continue
		}
		if vendor != "" && d.Vendor != vendor {
			continue
		}
		if status != "" && d.Status != status {
			continue
		}
		devices = append(devices, *d)
	}
	return devices, nil
}

func (s *GPUService) GetGPUDevice(id uint) (*models.GPUDevice, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	device, exists := s.db.GPUDevices[id]
	if !exists {
		return nil, mcerrors.NotFound("GPU device not found")
	}
	return device, nil
}

func (s *GPUService) CreateGPUDevice(device *models.GPUDevice) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	device.ID = s.db.GPUDeviceSeq
	device.CreatedAt = time.Now()
	device.UpdatedAt = time.Now()
	if device.Status == "" {
		device.Status = "available"
	}
	s.db.GPUDevices[s.db.GPUDeviceSeq] = device
	s.db.GPUDeviceSeq++
	return nil
}

func (s *GPUService) UpdateGPUDevice(device *models.GPUDevice) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	existing, exists := s.db.GPUDevices[device.ID]
	if !exists {
		return mcerrors.NotFound("GPU device not found")
	}

	if device.NodeName != "" {
		existing.NodeName = device.NodeName
	}
	if device.Vendor != "" {
		existing.Vendor = device.Vendor
	}
	if device.Model != "" {
		existing.Model = device.Model
	}
	if device.TotalMemoryGB > 0 {
		existing.TotalMemoryGB = device.TotalMemoryGB
	}
	if device.AllocatableMemoryGB >= 0 {
		existing.AllocatableMemoryGB = device.AllocatableMemoryGB
	}
	if device.UsedMemoryGB >= 0 {
		existing.UsedMemoryGB = device.UsedMemoryGB
	}
	existing.MIGEnabled = device.MIGEnabled
	if device.MIGProfiles != "" {
		existing.MIGProfiles = device.MIGProfiles
	}
	if device.DriverVersion != "" {
		existing.DriverVersion = device.DriverVersion
	}
	if device.CUDAVersion != "" {
		existing.CUDAVersion = device.CUDAVersion
	}
	if device.Status != "" {
		existing.Status = device.Status
	}
	existing.Utilization = device.Utilization
	existing.Temperature = device.Temperature
	existing.PowerDraw = device.PowerDraw
	if device.Details != "" {
		existing.Details = device.Details
	}
	existing.UpdatedAt = time.Now()
	return nil
}

func (s *GPUService) DeleteGPUDevice(id uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.GPUDevices[id]; !exists {
		return mcerrors.NotFound("GPU device not found")
	}
	delete(s.db.GPUDevices, id)
	return nil
}

// --- GPU Allocation ---

func (s *GPUService) AllocateGPU(jobID, tenantID, userID uint, fraction float64, memoryGB int, vendor string) (*models.GPUAllocation, error) {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	// 查找可用设备
	var selected *models.GPUDevice
	for _, d := range s.db.GPUDevices {
		if d.Status != "available" && d.Status != "allocated" {
			continue
		}
		if vendor != "" && d.Vendor != vendor {
			continue
		}
		// 检查显存是否足够
		availableMemory := d.AllocatableMemoryGB - d.UsedMemoryGB
		if availableMemory < memoryGB {
			continue
		}
		// 检查 fraction 是否可容纳（简化：累计已分配 fraction <= 1.0）
		var usedFraction float64
		for _, alloc := range s.db.GPUAllocations {
			if alloc.DeviceID == d.ID && alloc.Status == "active" {
				usedFraction += alloc.Fraction
			}
		}
		if usedFraction+fraction > 1.0+0.001 {
			continue
		}
		selected = d
		break
	}

	if selected == nil {
		return nil, mcerrors.New(mcerrors.ErrServiceUnavailable, "no available GPU device matching criteria")
	}

	now := time.Now()
	allocation := &models.GPUAllocation{
		ID:        s.db.GPUAllocationSeq,
		DeviceID:  selected.ID,
		JobID:     jobID,
		TenantID:  tenantID,
		UserID:    userID,
		Fraction:  fraction,
		MemoryGB:  memoryGB,
		Status:    "active",
		StartedAt: &now,
		CreatedAt: now,
		UpdatedAt: now,
	}
	s.db.GPUAllocations[s.db.GPUAllocationSeq] = allocation
	s.db.GPUAllocationSeq++

	// 更新设备已用显存和状态
	selected.UsedMemoryGB += memoryGB
	if selected.UsedMemoryGB > 0 {
		selected.Status = "allocated"
	}
	selected.UpdatedAt = now

	return allocation, nil
}

func (s *GPUService) ReleaseGPU(allocationID uint) error {
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	alloc, exists := s.db.GPUAllocations[allocationID]
	if !exists {
		return mcerrors.NotFound("GPU allocation not found")
	}
	if alloc.Status != "active" {
		return mcerrors.BadRequest("allocation is not active")
	}

	now := time.Now()
	alloc.Status = "released"
	alloc.EndedAt = &now
	alloc.UpdatedAt = now

	// 释放设备显存
	if device, ok := s.db.GPUDevices[alloc.DeviceID]; ok {
		device.UsedMemoryGB -= alloc.MemoryGB
		if device.UsedMemoryGB < 0 {
			device.UsedMemoryGB = 0
		}
		if device.UsedMemoryGB == 0 {
			device.Status = "available"
		}
		device.UpdatedAt = now
	}
	return nil
}

func (s *GPUService) GetGPUAllocations(jobID uint) ([]models.GPUAllocation, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var allocations []models.GPUAllocation
	for _, a := range s.db.GPUAllocations {
		if jobID > 0 && a.JobID != jobID {
			continue
		}
		allocations = append(allocations, *a)
	}
	return allocations, nil
}

func (s *GPUService) GetGPUUtilizationSummary(clusterID uint) (map[string]interface{}, error) {
	s.db.Mu.RLock()
	defer s.db.Mu.RUnlock()

	var totalDevices, availableDevices, allocatedDevices, maintenanceDevices, faultDevices int
	var totalMemoryGB, usedMemoryGB float64
	var totalUtilization float64
	vendorStats := make(map[string]int)

	for _, d := range s.db.GPUDevices {
		if clusterID > 0 && d.ClusterID != clusterID {
			continue
		}
		totalDevices++
		totalMemoryGB += float64(d.TotalMemoryGB)
		usedMemoryGB += float64(d.UsedMemoryGB)
		totalUtilization += d.Utilization
		vendorStats[d.Vendor]++

		switch d.Status {
		case "available":
			availableDevices++
		case "allocated":
			allocatedDevices++
		case "maintenance":
			maintenanceDevices++
		case "fault":
			faultDevices++
		}
	}

	avgUtilization := 0.0
	if totalDevices > 0 {
		avgUtilization = totalUtilization / float64(totalDevices)
	}

	return map[string]interface{}{
		"total_devices":       totalDevices,
		"available_devices":   availableDevices,
		"allocated_devices":   allocatedDevices,
		"maintenance_devices": maintenanceDevices,
		"fault_devices":       faultDevices,
		"total_memory_gb":     totalMemoryGB,
		"used_memory_gb":      usedMemoryGB,
		"avg_utilization":     fmt.Sprintf("%.2f", avgUtilization),
		"vendor_stats":        vendorStats,
	}, nil
}
