package models

import (
	"context"
	"fmt"
	"sync"
	"time"

	"metaclouds-backend/config"
	"metaclouds-backend/pkg/logger"

	"github.com/go-redis/redis/v8"
)

var RedisClient *redis.Client
var ctx = context.Background()

var memoryCache = make(map[string]string)
var memoryCacheMu sync.RWMutex

func InitRedis(cfg *config.Config) (*redis.Client, error) {
	start := time.Now()
	logger.InfoWithCtx(nil, "Redis initialization started",
		"redis_enabled", cfg.RedisEnabled,
		"redis_host", cfg.RedisHost,
		"redis_port", cfg.RedisPort,
		"redis_db", cfg.RedisDB)

	if !cfg.RedisEnabled {
		logger.InfoWithCtx(nil, "Redis initialization - Redis is disabled, using memory fallback",
			"duration", time.Since(start))
		return nil, nil
	}

	logger.DebugWithCtx(nil, "Redis initialization - Creating Redis client",
		"redis_addr", cfg.GetRedisAddr(),
		"pool_size", 10,
		"read_timeout", "5s",
		"write_timeout", "5s")

	client := redis.NewClient(&redis.Options{
		Addr:         cfg.GetRedisAddr(),
		Password:     cfg.RedisPassword,
		DB:           cfg.RedisDB,
		ReadTimeout:  5 * time.Second,
		WriteTimeout: 5 * time.Second,
		PoolSize:     10,
	})

	pingStart := time.Now()
	_, err := client.Ping(ctx).Result()
	if err != nil {
		logger.WarnWithCtx(nil, "Redis initialization failed - Ping failed, falling back to memory cache",
			"redis_host", cfg.RedisHost,
			"redis_port", cfg.RedisPort,
			"error", err,
			"ping_duration", time.Since(pingStart),
			"total_duration", time.Since(start))
		return nil, err
	}
	logger.DebugWithCtx(nil, "Redis initialization - Ping successful",
		"ping_duration", time.Since(pingStart))

	RedisClient = client
	logger.InfoWithCtx(nil, "Redis initialization completed successfully",
		"redis_host", cfg.RedisHost,
		"redis_port", cfg.RedisPort,
		"redis_db", cfg.RedisDB,
		"duration", time.Since(start))

	return client, nil
}

func GetRedisClient() *redis.Client {
	return RedisClient
}

func setMemoryCache(key string, value string, expiration time.Duration) {
	memoryCacheMu.Lock()
	defer memoryCacheMu.Unlock()
	memoryCache[key] = value
}

func getMemoryCache(key string) (string, bool) {
	memoryCacheMu.RLock()
	defer memoryCacheMu.RUnlock()
	value, exists := memoryCache[key]
	return value, exists
}

func deleteMemoryCache(key string) {
	memoryCacheMu.Lock()
	defer memoryCacheMu.Unlock()
	delete(memoryCache, key)
}

func SetCache(key string, value interface{}, expiration time.Duration) error {
	if RedisClient != nil {
		return RedisClient.Set(ctx, key, value, expiration).Err()
	}
	setMemoryCache(key, fmt.Sprintf("%v", value), expiration)
	return nil
}

func GetCache(key string) (string, error) {
	if RedisClient != nil {
		return RedisClient.Get(ctx, key).Result()
	}
	value, exists := getMemoryCache(key)
	if !exists {
		return "", redis.Nil
	}
	return value, nil
}

func DeleteCache(key string) error {
	if RedisClient != nil {
		return RedisClient.Del(ctx, key).Err()
	}
	deleteMemoryCache(key)
	return nil
}

func SetTokenBlacklist(token string, expiration time.Duration) error {
	return SetCache("blacklist:"+token, "true", expiration)
}

func IsTokenBlacklisted(token string) (bool, error) {
	val, err := GetCache("blacklist:" + token)
	if err != nil {
		if err == redis.Nil {
			return false, nil
		}
		return false, err
	}
	return val == "true", nil
}

func StoreSession(sessionID string, userID uint, expiration time.Duration) error {
	return SetCache("session:"+sessionID, userID, expiration)
}

func GetSession(sessionID string) (uint, error) {
	val, err := GetCache("session:" + sessionID)
	if err != nil {
		return 0, err
	}
	var userID uint
	fmt.Sscanf(val, "%d", &userID)
	return userID, nil
}

func InvalidateSession(sessionID string) error {
	return DeleteCache("session:" + sessionID)
}

func CacheMetrics(metrics map[string]interface{}) error {
	if RedisClient == nil {
		return fmt.Errorf("redis client not initialized")
	}
	for key, value := range metrics {
		err := RedisClient.Set(ctx, "metrics:"+key, value, 5*time.Minute).Err()
		if err != nil {
			return err
		}
	}
	return nil
}

func GetCachedMetrics() (map[string]interface{}, error) {
	if RedisClient == nil {
		return nil, fmt.Errorf("redis client not initialized")
	}
	keys, err := RedisClient.Keys(ctx, "metrics:*").Result()
	if err != nil {
		return nil, err
	}
	metrics := make(map[string]interface{})
	for _, key := range keys {
		val, err := RedisClient.Get(ctx, key).Result()
		if err != nil {
			return nil, err
		}
		metrics[key[8:]] = val
	}
	return metrics, nil
}
