package models

import (
	"log"

	"gorm.io/gorm"
)

// InitData 初始化默认数据
func InitData(db *gorm.DB) error {
	// 检查是否已有用户数据
	var userCount int64
	if err := db.Model(&User{}).Count(&userCount).Error; err != nil {
		return err
	}

	// 如果没有用户，创建默认用户
	if userCount == 0 {
		// 引导口令必须来自环境变量。生产环境缺失时拒绝启动，
		// 非生产环境生成一次性随机口令，不再回退到硬编码常量。
		defaultAdminPassword, err := bootstrapPassword("DEFAULT_ADMIN_PASSWORD", "admin")
		if err != nil {
			return err
		}
		defaultUserPassword, err := bootstrapPassword("DEFAULT_USER_PASSWORD", "user")
		if err != nil {
			return err
		}

		// 创建默认租户
		defaultTenant := Tenant{
			Name:         "默认租户",
			Description:  "系统默认租户",
			Status:       "active",
			GPUQuota:     10,
			CPUQuota:     100,
			MemoryQuota:  1000,
			StorageQuota: 10000,
		}
		if err := db.Create(&defaultTenant).Error; err != nil {
			return err
		}

		// 创建默认管理员用户
		adminUser := User{
			Username: "admin",
			Email:    "admin@example.com",
			Password: defaultAdminPassword, // 密码会在BeforeSave中自动加密
			Role:     "admin",
			TenantID: defaultTenant.ID,
		}
		if err := db.Create(&adminUser).Error; err != nil {
			return err
		}

		// 创建默认普通用户
		userUser := User{
			Username: "user",
			Email:    "user@example.com",
			Password: defaultUserPassword, // 密码会在BeforeSave中自动加密
			Role:     "user",
			TenantID: defaultTenant.ID,
		}
		if err := db.Create(&userUser).Error; err != nil {
			return err
		}

		log.Println("Default users created successfully")
	}

	// 检查是否已有集群数据
	var clusterCount int64
	if err := db.Model(&Cluster{}).Count(&clusterCount).Error; err != nil {
		return err
	}

	// 如果没有集群，创建默认集群
	if clusterCount == 0 {
		defaultCluster := Cluster{
			Name:        "默认集群",
			Description: "系统默认集群",
			Status:      "active",
			Nodes:       10,
			GPUs:        20,
			CPUs:        100,
			Memory:      1000,
			Storage:     10000,
		}
		if err := db.Create(&defaultCluster).Error; err != nil {
			return err
		}

		log.Println("Default cluster created successfully")
	}

	return nil
}
