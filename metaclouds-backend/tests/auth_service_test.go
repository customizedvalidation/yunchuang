package tests

import (
	"metaclouds-backend/config"
	"metaclouds-backend/models"
	"metaclouds-backend/services"
	"testing"

	"github.com/stretchr/testify/assert"
	"golang.org/x/crypto/bcrypt"
)

func TestAuthService_Login(t *testing.T) {
	// 使用内存存储替代 SQLite（避免 CGO 依赖）
	db := models.MustNewMemoryStore()

	// 创建测试用户（密码需要已加密）
	hashedPassword, _ := bcrypt.GenerateFromPassword([]byte("password123"), bcrypt.DefaultCost)
	user := &models.User{
		Username: "testuser",
		Email:    "test@example.com",
		Password: string(hashedPassword),
		Role:     "user",
	}
	db.AddUserWithIndex(user)

	// 创建配置
	cfg := &config.Config{
		JWTSecret:          "test-secret",
		JWTExpirationHours: 24,
	}

	// 创建认证服务（使用内存存储）
	authService := services.NewAuthService(db, nil, cfg)

	// 测试登录
	req := services.LoginRequest{
		Username: "testuser",
		Password: "password123",
	}

	resp, err := authService.Login(req)
	assert.NoError(t, err)
	assert.NotEmpty(t, resp.Token)
	assert.Equal(t, "testuser", resp.User.Username)

	// 测试错误的密码
	req.Password = "wrongpassword"
	resp, err = authService.Login(req)
	assert.Error(t, err)
	assert.Nil(t, resp)

	// 测试不存在的用户
	req.Username = "nonexistent"
	req.Password = "password123"
	resp, err = authService.Login(req)
	assert.Error(t, err)
	assert.Nil(t, resp)
}

func TestAuthService_Register(t *testing.T) {
	// 使用内存存储替代 SQLite（避免 CGO 依赖）
	db := models.MustNewMemoryStore()

	// 创建配置
	cfg := &config.Config{
		JWTSecret:          "test-secret",
		JWTExpirationHours: 24,
	}

	// 创建认证服务（使用内存存储）
	authService := services.NewAuthService(db, nil, cfg)

	// 测试注册
	req := services.RegisterRequest{
		Username: "newuser",
		Email:    "new@example.com",
		Password: "password123",
		Role:     "user",
	}

	user, err := authService.Register(req)
	assert.NoError(t, err)
	assert.Equal(t, "newuser", user.Username)
	assert.Equal(t, "new@example.com", user.Email)
	assert.Equal(t, "user", user.Role)

	// 测试用户名已存在
	user, err = authService.Register(req)
	assert.Error(t, err)
	assert.Nil(t, user)

	// 测试邮箱已存在
	req.Username = "anotheruser"
	user, err = authService.Register(req)
	assert.Error(t, err)
	assert.Nil(t, user)
}
