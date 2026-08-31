package middleware

import (
	"sort"
	"sync"
	"time"

	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/response"

	"github.com/gin-gonic/gin"
)

type RateLimiter struct {
	visitors        map[string]*visitor
	mu              sync.Mutex
	limit           int
	window          time.Duration
	cleanupInterval time.Duration
}

type visitor struct {
	tokens int
	last   time.Time
}

func NewRateLimiter(limit int, window time.Duration) *RateLimiter {
	rl := &RateLimiter{
		visitors:        make(map[string]*visitor),
		limit:           limit,
		window:          window,
		cleanupInterval: window * 2,
	}
	go rl.startCleanup()
	return rl
}

func (rl *RateLimiter) startCleanup() {
	ticker := time.NewTicker(rl.cleanupInterval)
	defer ticker.Stop()
	for range ticker.C {
		rl.cleanupExpired()
	}
}

func (rl *RateLimiter) cleanupExpired() {
	rl.mu.Lock()
	defer rl.mu.Unlock()
	now := time.Now()
	for ip, v := range rl.visitors {
		if now.Sub(v.last) > rl.window*2 {
			delete(rl.visitors, ip)
		}
	}
}

func (rl *RateLimiter) Middleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		ip := c.ClientIP()
		rl.mu.Lock()
		defer rl.mu.Unlock()

		v, exists := rl.visitors[ip]
		if !exists {
			rl.visitors[ip] = &visitor{
				tokens: rl.limit - 1,
				last:   time.Now(),
			}
			c.Next()
			return
		}

		if time.Since(v.last) > rl.window {
			v.tokens = rl.limit - 1
			v.last = time.Now()
			c.Next()
			return
		}

		if v.tokens <= 0 {
			response.Error(c, errors.NewRateLimitError("Too many requests"))
			c.Abort()
			return
		}

		v.tokens--
		c.Next()
	}
}

const (
	// maxTrackedVisitors 是滑动窗口限流器最多同时跟踪的来源数量。
	// 达到上限后会淘汰最久未活动的记录，防止在 IP 伪造或大规模
	// 分布式请求下 map 无限膨胀导致 OOM。
	maxTrackedVisitors = 100_000
	// visitorTTL 是一个来源在多久没有请求后被清理。
	visitorTTL = 10 * time.Minute
)

type SlidingWindowLimiter struct {
	visitors map[string]*slidingWindow
	mu       sync.Mutex
	limit    int
	window   time.Duration
	stopOnce sync.Once
	stopCh   chan struct{}
}

type slidingWindow struct {
	timestamps []time.Time
	lastSeen   time.Time
}

func NewSlidingWindowLimiter(limit int, window time.Duration) *SlidingWindowLimiter {
	swl := &SlidingWindowLimiter{
		visitors: make(map[string]*slidingWindow),
		limit:    limit,
		window:   window,
		stopCh:   make(chan struct{}),
	}
	go swl.cleanupLoop()
	return swl
}

// cleanupLoop 定期清理过期来源。
//
// 原先的实现只往 map 里加条目、从不删除，只要来源 IP 不断变化
// （例如伪造 X-Forwarded-For）内存就会持续增长直至进程被 OOM 杀掉。
func (swl *SlidingWindowLimiter) cleanupLoop() {
	ticker := time.NewTicker(time.Minute)
	defer ticker.Stop()

	for {
		select {
		case <-swl.stopCh:
			return
		case <-ticker.C:
			swl.evictExpired(visitorTTL)
		}
	}
}

// evictExpired 删除超过 ttl 未活动的来源；若仍超过上限，继续淘汰最久未见的一批。
func (swl *SlidingWindowLimiter) evictExpired(ttl time.Duration) {
	swl.mu.Lock()
	defer swl.mu.Unlock()

	cutoff := time.Now().Add(-ttl)
	for ip, v := range swl.visitors {
		if v.lastSeen.Before(cutoff) {
			delete(swl.visitors, ip)
		}
	}

	if len(swl.visitors) <= maxTrackedVisitors {
		return
	}

	// 超出硬上限：按最后访问时间排序并淘汰最旧的一批，保证内存有界。
	type entry struct {
		ip  string
		seen time.Time
	}
	overflow := len(swl.visitors) - maxTrackedVisitors
	entries := make([]entry, 0, len(swl.visitors))
	for ip, v := range swl.visitors {
		entries = append(entries, entry{ip: ip, seen: v.lastSeen})
	}
	sort.Slice(entries, func(i, j int) bool {
		return entries[i].seen.Before(entries[j].seen)
	})
	for i := 0; i < overflow && i < len(entries); i++ {
		delete(swl.visitors, entries[i].ip)
	}
}

// Stop 停止后台清理协程，供服务优雅退出时调用。
func (swl *SlidingWindowLimiter) Stop() {
	swl.stopOnce.Do(func() {
		close(swl.stopCh)
	})
}

func (swl *SlidingWindowLimiter) Middleware() gin.HandlerFunc {
	return func(c *gin.Context) {
		ip := c.ClientIP()
		now := time.Now()
		cutoff := now.Add(-swl.window)

		swl.mu.Lock()

		v, exists := swl.visitors[ip]
		if !exists {
			// 已满且是全新来源时先腾出空间，避免无上限增长。
			if len(swl.visitors) >= maxTrackedVisitors {
				swl.evictExpired(visitorTTL)
			}
			swl.visitors[ip] = &slidingWindow{
				timestamps: []time.Time{now},
				lastSeen:   now,
			}
			swl.mu.Unlock()
			c.Next()
			return
		}

		// 原地过滤掉窗口外的旧时间戳，避免每次请求都分配新切片。
		kept := v.timestamps[:0]
		for _, t := range v.timestamps {
			if t.After(cutoff) {
				kept = append(kept, t)
			}
		}
		v.timestamps = kept
		v.lastSeen = now

		if len(v.timestamps) >= swl.limit {
			swl.mu.Unlock()
			response.Error(c, errors.NewRateLimitError("Too many requests"))
			c.Abort()
			return
		}

		v.timestamps = append(v.timestamps, now)
		swl.mu.Unlock()
		c.Next()
	}
}
