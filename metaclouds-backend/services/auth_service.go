package services

import (
	"context"
	"crypto/rand"
	"encoding/hex"
	"strconv"
	"sync"
	"time"

	"golang.org/x/crypto/bcrypt"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/jwttool"
	"metaclouds-backend/pkg/logger"
)

const (
	// maxFailedLogins 是在触发锁定前允许的连续登录失败次数。
	maxFailedLogins = 5
	// loginLockoutDuration 是连续登录失败过多后的锁定时长。
	loginLockoutDuration = 15 * time.Minute
)

// loginAttempt 记录单个账户的失败登录状态，用于缓解暴力破解。
type loginAttempt struct {
	failures  int
	lockedUntil time.Time
}

type AuthService struct {
	db             *models.MemoryStore
	config         *config.Config
	tokenGenerator *jwttool.TokenGenerator

	attemptsMu sync.Mutex
	attempts   map[string]*loginAttempt

	// dummyHash 是一个与任何账户都不对应的 bcrypt 哈希。当用户不存在时用它做一次
	// 等价耗时的比较，防止通过响应时间探测用户名是否存在。
	dummyHash []byte
}

func NewAuthService(db interface{}, redisClient interface{}, cfg *config.Config) *AuthService {
	memoryStore, err := models.GetDBStore(db, "AuthService")
	if err != nil {
		logger.ErrorWithCtx(nil, "Failed to initialize AuthService", err)
		return nil
	}

	// 生成一次性的哑哈希。失败时降级为 nil：此时仅丧失时序一致性，
	// 不影响正常登录功能。
	dummy, hashErr := bcrypt.GenerateFromPassword([]byte(randomToken()), bcrypt.DefaultCost)
	if hashErr != nil {
		logger.WarnWithCtx(nil, "Failed to generate dummy password hash", hashErr)
	}

	return &AuthService{
		db:             memoryStore,
		config:         cfg,
		tokenGenerator: jwttool.NewTokenGenerator(cfg.JWTSecret, cfg.JWTExpirationHours),
		attempts:       make(map[string]*loginAttempt),
		dummyHash:      dummy,
	}
}

// randomToken 生成一个仅用于哑哈希的随机字符串。
func randomToken() string {
	buf := make([]byte, 32)
	if _, err := rand.Read(buf); err != nil {
		// 回退到时间戳，仍然不可预测到可被利用的程度。
		return strconv.FormatInt(time.Now().UnixNano(), 10)
	}
	return hex.EncodeToString(buf)
}

// isLocked 报告该账户当前是否处于锁定状态，同时清理已过期的记录。
func (s *AuthService) isLocked(username string) (bool, time.Duration) {
	s.attemptsMu.Lock()
	defer s.attemptsMu.Unlock()

	attempt, exists := s.attempts[username]
	if !exists {
		return false, 0
	}
	if attempt.lockedUntil.IsZero() {
		return false, 0
	}
	if remaining := time.Until(attempt.lockedUntil); remaining > 0 {
		return true, remaining
	}

	// 锁定已到期，重置计数以便用户重试。
	attempt.lockedUntil = time.Time{}
	attempt.failures = 0
	return false, 0
}

// recordFailure 记录一次登录失败，达到阈值后锁定账户。
func (s *AuthService) recordFailure(username string) {
	s.attemptsMu.Lock()
	defer s.attemptsMu.Unlock()

	attempt, exists := s.attempts[username]
	if !exists {
		attempt = &loginAttempt{}
		s.attempts[username] = attempt
	}
	attempt.failures++
	if attempt.failures >= maxFailedLogins {
		attempt.lockedUntil = time.Now().Add(loginLockoutDuration)
		attempt.failures = 0
	}
}

// recordSuccess 登录成功后清除该账户的失败记录。
func (s *AuthService) recordSuccess(username string) {
	s.attemptsMu.Lock()
	defer s.attemptsMu.Unlock()
	delete(s.attempts, username)
}

type LoginRequest struct {
	Username string `json:"username" binding:"required"`
	Password string `json:"password" binding:"required"`
}

type RegisterRequest struct {
	Username string `json:"username" binding:"required"`
	Email    string `json:"email" binding:"required,email"`
	Password string `json:"password" binding:"required,min=6"`
	Role     string `json:"role"`
	TenantID uint   `json:"tenant_id"`
}

type LoginResponse struct {
	Token     string              `json:"token"`
	User      models.UserResponse `json:"user"`
	ExpiresAt int64               `json:"expires_at"`
}

func (s *AuthService) Login(req LoginRequest) (*LoginResponse, error) {
	ctx := context.Background()

	// 先检查账户锁定，避免对已锁定账户继续执行昂贵的 bcrypt 校验。
	if locked, remaining := s.isLocked(req.Username); locked {
		logger.WarnWithCtx(ctx, "Login rejected - account temporarily locked",
			"username", req.Username,
			"retry_after", remaining)
		return nil, errors.NewRateLimitError(
			"account temporarily locked due to repeated failed logins")
	}

	s.db.Mu.RLock()
	user := s.db.UsersByUsername[req.Username]
	// 在持锁期间取出密码哈希副本，避免解锁后读取仍在被其他协程修改的对象。
	var passwordHash string
	if user != nil {
		passwordHash = user.Password
	}
	s.db.Mu.RUnlock()

	// 用户不存在时仍执行一次 bcrypt 比较，使响应耗时与"密码错误"接近，
	// 降低通过响应时间枚举有效用户名的风险。
	if passwordHash == "" {
		passwordHash = string(s.dummyHash)
	}
	if err := bcrypt.CompareHashAndPassword([]byte(passwordHash), []byte(req.Password)); err != nil {
		s.recordFailure(req.Username)
		return nil, errors.Unauthorized("invalid username or password")
	}
	if user == nil {
		s.recordFailure(req.Username)
		return nil, errors.Unauthorized("invalid username or password")
	}

	claims := jwttool.TokenClaims{
		UserID:   user.ID,
		Username: user.Username,
		Email:    user.Email,
		Role:     user.Role,
		TenantID: user.TenantID,
	}
	token, expiresAt, err := s.tokenGenerator.GenerateToken(claims)
	if err != nil {
		return nil, errors.InternalServer("failed to generate token")
	}

	s.recordSuccess(req.Username)

	return &LoginResponse{
		Token:     token,
		User:      user.ToResponse(),
		ExpiresAt: expiresAt,
	}, nil
}

// Register 处理用户自注册。
//
// 安全约束：自注册账号一律被赋予 "user" 角色，请求体中携带的 role 字段会被忽略。
// 否则攻击者可以通过注册接口直接把自己提升为 admin 并接管平台。
func (s *AuthService) Register(req RegisterRequest) (*models.UserResponse, error) {
	ctx := context.Background()

	if req.Role != "" && req.Role != "user" {
		logger.WarnWithCtx(ctx, "Register request attempted to self-assign a privileged role; downgrading to 'user'",
			"username", req.Username,
			"requested_role", req.Role)
	}

	// bcrypt 计算成本较高，放在临界区之外执行，避免长时间独占存储锁。
	hashedPassword, err := bcrypt.GenerateFromPassword([]byte(req.Password), bcrypt.DefaultCost)
	if err != nil {
		return nil, errors.InternalServer("failed to hash password")
	}

	// 唯一性检查与插入必须在同一把写锁内完成，否则并发注册会同时通过检查。
	s.db.Mu.Lock()
	defer s.db.Mu.Unlock()

	if _, exists := s.db.UsersByUsername[req.Username]; exists {
		return nil, errors.Conflict("username already exists")
	}
	if _, exists := s.db.UsersByEmail[req.Email]; exists {
		return nil, errors.Conflict("email already exists")
	}

	// 校验租户确实存在且处于可用状态，防止把账号挂到不存在的租户上。
	if req.TenantID > 0 {
		tenant, exists := s.db.Tenants[req.TenantID]
		if !exists {
			return nil, errors.NotFound("tenant not found")
		}
		if tenant.Status != "active" {
			return nil, errors.BadRequest("tenant is not active")
		}
	}

	user := &models.User{
		ID:        s.db.UserSeq,
		Username:  req.Username,
		Email:     req.Email,
		Password:  string(hashedPassword),
		Role:      "user",
		TenantID:  req.TenantID,
		CreatedAt: time.Now(),
		UpdatedAt: time.Now(),
	}
	s.db.AddUserWithIndex(user)
	s.db.UserSeq++

	resp := user.ToResponse()
	return &resp, nil
}

type RefreshResponse struct {
	Token     string              `json:"token"`
	User      models.UserResponse `json:"user"`
	ExpiresAt int64               `json:"expires_at"`
}

func (s *AuthService) Refresh(userID uint) (*RefreshResponse, error) {
	s.db.Mu.RLock()
	user, exists := s.db.Users[userID]
	s.db.Mu.RUnlock()

	if !exists {
		return nil, errors.NotFound("user not found")
	}

	claims := jwttool.TokenClaims{
		UserID:   user.ID,
		Username: user.Username,
		Email:    user.Email,
		Role:     user.Role,
		TenantID: user.TenantID,
	}
	token, expiresAt, err := s.tokenGenerator.GenerateToken(claims)
	if err != nil {
		return nil, errors.InternalServer("failed to generate token")
	}

	return &RefreshResponse{
		Token:     token,
		User:      user.ToResponse(),
		ExpiresAt: expiresAt,
	}, nil
}
