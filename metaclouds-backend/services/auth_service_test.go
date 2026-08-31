package services

import (
	"testing"

	"github.com/stretchr/testify/assert"
	"golang.org/x/crypto/bcrypt"

	"metaclouds-backend/config"
	"metaclouds-backend/models"
)

func TestAuthService_Login_Success(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
		JWTExpirationHours: 24,
	}

	db := models.MustNewMemoryStore()
	hashedPassword, _ := bcrypt.GenerateFromPassword([]byte("testpassword"), bcrypt.DefaultCost)
	user := &models.User{
		ID:       1,
		Username: "testuser",
		Email:    "test@example.com",
		Password: string(hashedPassword),
		Role:     "user",
		TenantID: 1,
	}
	db.Mu.Lock()
	db.AddUserWithIndex(user)
	db.Mu.Unlock()

	var redisClient interface{} = nil

	service := NewAuthService(db, redisClient, cfg)

	req := LoginRequest{
		Username: "testuser",
		Password: "testpassword",
	}

	resp, err := service.Login(req)

	assert.NoError(t, err)
	assert.NotNil(t, resp)
	assert.NotEmpty(t, resp.Token)
	assert.Equal(t, "testuser", resp.User.Username)
	assert.Equal(t, "test@example.com", resp.User.Email)
}

func TestAuthService_Login_InvalidPassword(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
		JWTExpirationHours: 24,
	}

	db := models.MustNewMemoryStore()
	hashedPassword, _ := bcrypt.GenerateFromPassword([]byte("testpassword"), bcrypt.DefaultCost)
	user := &models.User{
		ID:       1,
		Username: "testuser",
		Email:    "test@example.com",
		Password: string(hashedPassword),
		Role:     "user",
		TenantID: 1,
	}
	db.Mu.Lock()
	db.AddUserWithIndex(user)
	db.Mu.Unlock()

	var redisClient interface{} = nil

	service := NewAuthService(db, redisClient, cfg)

	req := LoginRequest{
		Username: "testuser",
		Password: "wrongpassword",
	}

	resp, err := service.Login(req)

	assert.Error(t, err)
	assert.Nil(t, resp)
	assert.Contains(t, err.Error(), "invalid username or password")
}

func TestAuthService_Login_UserNotFound(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
		JWTExpirationHours: 24,
	}

	db := models.MustNewMemoryStore()
	var redisClient interface{} = nil

	service := NewAuthService(db, redisClient, cfg)

	req := LoginRequest{
		Username: "nonexistent",
		Password: "anypassword",
	}

	resp, err := service.Login(req)

	assert.Error(t, err)
	assert.Nil(t, resp)
	assert.Contains(t, err.Error(), "invalid username or password")
}

func TestAuthService_Register_Success(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
		JWTExpirationHours: 24,
	}

	db := models.MustNewMemoryStore()
	var redisClient interface{} = nil

	service := NewAuthService(db, redisClient, cfg)

	req := RegisterRequest{
		Username: "newuser",
		Email:    "new@example.com",
		Password: "newpassword",
	}

	resp, err := service.Register(req)

	assert.NoError(t, err)
	assert.NotNil(t, resp)
	assert.Equal(t, "newuser", resp.Username)
	assert.Equal(t, "new@example.com", resp.Email)
	assert.NotEmpty(t, resp.ID)

	db.Mu.RLock()
	_, exists := db.Users[resp.ID]
	db.Mu.RUnlock()
	assert.True(t, exists)
}

func TestAuthService_Register_DuplicateUsername(t *testing.T) {
	cfg := &config.Config{
		JWTSecret:          "test-secret",
		JWTExpirationHours: 24,
	}

	db := models.MustNewMemoryStore()
	hashedPassword, _ := bcrypt.GenerateFromPassword([]byte("password"), bcrypt.DefaultCost)
	user := &models.User{
		ID:       1,
		Username: "existing",
		Email:    "existing@example.com",
		Password: string(hashedPassword),
		Role:     "user",
		TenantID: 1,
	}
	db.Mu.Lock()
	db.AddUserWithIndex(user)
	db.Mu.Unlock()

	var redisClient interface{} = nil

	service := NewAuthService(db, redisClient, cfg)

	req := RegisterRequest{
		Username: "existing",
		Email:    "new@example.com",
		Password: "password",
	}

	resp, err := service.Register(req)

	assert.Error(t, err)
	assert.Nil(t, resp)
	assert.Contains(t, err.Error(), "username already exists")
}
