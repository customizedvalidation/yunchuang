package controllers

import (
	"errors"
	"net/http"
	"testing"

	"github.com/gin-gonic/gin"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	appErrors "metaclouds-backend/pkg/errors"
	"metaclouds-backend/models"
	"metaclouds-backend/services"
)

func TestAuthController_Login(t *testing.T) {
	tests := []struct {
		name           string
		requestBody    interface{}
		setupMock      func(m *mockAuthService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "登录成功",
			requestBody: map[string]string{
				"username": "testuser",
				"password": "password123",
			},
			setupMock: func(m *mockAuthService) {
				m.loginFunc = func(req services.LoginRequest) (*services.LoginResponse, error) {
					assert.Equal(t, "testuser", req.Username)
					return &services.LoginResponse{Token: "fake-jwt-token"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "请求体为空-绑定失败返回500",
			requestBody:    nil,
			setupMock:      func(m *mockAuthService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name: "缺少必填字段-用户名",
			requestBody: map[string]string{
				"password": "password123",
			},
			setupMock:      func(m *mockAuthService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name: "服务层返回未授权错误",
			requestBody: map[string]string{
				"username": "wronguser",
				"password": "wrongpass",
			},
			setupMock: func(m *mockAuthService) {
				m.loginFunc = func(req services.LoginRequest) (*services.LoginResponse, error) {
					return nil, appErrors.Unauthorized("invalid username or password")
				}
			},
			expectedStatus: http.StatusUnauthorized,
			expectSuccess:  false,
		},
		{
			name: "服务层返回内部错误",
			requestBody: map[string]string{
				"username": "testuser",
				"password": "password123",
			},
			setupMock: func(m *mockAuthService) {
				m.loginFunc = func(req services.LoginRequest) (*services.LoginResponse, error) {
					return nil, errors.New("database connection failed")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockAuthService{}
			tc.setupMock(mock)
			controller := NewAuthController(mock)

			c, w := newTestContext(http.MethodPost, "/api/v1/auth/login", tc.requestBody)
			controller.Login(c)

			if tc.expectSuccess {
				resp := assertSuccessResponse(t, w, tc.expectedStatus)
				data, ok := resp["data"].(map[string]interface{})
				require.True(t, ok)
				assert.Equal(t, "fake-jwt-token", data["token"])
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestAuthController_Register(t *testing.T) {
	tests := []struct {
		name           string
		requestBody    interface{}
		setupMock      func(m *mockAuthService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name: "注册成功",
			requestBody: map[string]interface{}{
				"username": "newuser",
				"email":    "new@example.com",
				"password": "password123",
			},
			setupMock: func(m *mockAuthService) {
				m.registerFunc = func(req services.RegisterRequest) (*models.UserResponse, error) {
					assert.Equal(t, "newuser", req.Username)
					return &models.UserResponse{ID: 1, Username: "newuser"}, nil
				}
			},
			expectedStatus: http.StatusCreated,
			expectSuccess:  true,
		},
		{
			name:           "请求体为空",
			requestBody:    nil,
			setupMock:      func(m *mockAuthService) {},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
		{
			name: "邮箱格式无效",
			requestBody: map[string]interface{}{
				"username": "newuser",
				"email":    "not-an-email",
				"password": "password123",
			},
			setupMock:      func(m *mockAuthService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name: "密码太短",
			requestBody: map[string]interface{}{
				"username": "newuser",
				"email":    "new@example.com",
				"password": "123",
			},
			setupMock:      func(m *mockAuthService) {},
			expectedStatus: http.StatusBadRequest,
			expectSuccess:  false,
		},
		{
			name: "用户名已存在-冲突",
			requestBody: map[string]interface{}{
				"username": "existing",
				"email":    "new@example.com",
				"password": "password123",
			},
			setupMock: func(m *mockAuthService) {
				m.registerFunc = func(req services.RegisterRequest) (*models.UserResponse, error) {
					return nil, appErrors.Conflict("username already exists")
				}
			},
			expectedStatus: http.StatusConflict,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockAuthService{}
			tc.setupMock(mock)
			controller := NewAuthController(mock)

			c, w := newTestContext(http.MethodPost, "/api/v1/auth/register", tc.requestBody)
			controller.Register(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestAuthController_Refresh(t *testing.T) {
	tests := []struct {
		name           string
		setContext     bool
		userID         uint
		setupMock      func(m *mockAuthService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:       "刷新令牌成功",
			setContext: true,
			userID:     1,
			setupMock: func(m *mockAuthService) {
				m.refreshFunc = func(userID uint) (*services.RefreshResponse, error) {
					assert.Equal(t, uint(1), userID)
					return &services.RefreshResponse{Token: "new-token"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "未认证-上下文中无user_id",
			setContext:     false,
			setupMock:      func(m *mockAuthService) {},
			expectedStatus: http.StatusUnauthorized,
			expectSuccess:  false,
		},
		{
			name:       "用户不存在",
			setContext: true,
			userID:     999,
			setupMock: func(m *mockAuthService) {
				m.refreshFunc = func(userID uint) (*services.RefreshResponse, error) {
					return nil, appErrors.NotFound("user not found")
				}
			},
			expectedStatus: http.StatusNotFound,
			expectSuccess:  false,
		},
		{
			name:       "服务层内部错误",
			setContext: true,
			userID:     1,
			setupMock: func(m *mockAuthService) {
				m.refreshFunc = func(userID uint) (*services.RefreshResponse, error) {
					return nil, errors.New("token generation failed")
				}
			},
			expectedStatus: http.StatusInternalServerError,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockAuthService{}
			tc.setupMock(mock)
			controller := NewAuthController(mock)

			c, w := newTestContext(http.MethodPost, "/api/v1/auth/refresh", nil)
			if tc.setContext {
				c.Set("user_id", tc.userID)
			}
			controller.Refresh(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestAuthController_GetProfile(t *testing.T) {
	tests := []struct {
		name           string
		setContext     bool
		userID         uint
		setupMock      func(m *mockAuthService)
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:       "获取用户资料成功",
			setContext: true,
			userID:     1,
			setupMock: func(m *mockAuthService) {
				m.getProfileFunc = func(userID uint) (*models.UserResponse, error) {
					return &models.UserResponse{ID: 1, Username: "testuser", Role: "user"}, nil
				}
			},
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "未认证",
			setContext:     false,
			setupMock:      func(m *mockAuthService) {},
			expectedStatus: http.StatusUnauthorized,
			expectSuccess:  false,
		},
		{
			name:       "用户已删除-不存在",
			setContext: true,
			userID:     50,
			setupMock: func(m *mockAuthService) {
				m.getProfileFunc = func(userID uint) (*models.UserResponse, error) {
					return nil, appErrors.NotFound("user not found")
				}
			},
			expectedStatus: http.StatusNotFound,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockAuthService{}
			tc.setupMock(mock)
			controller := NewAuthController(mock)

			c, w := newTestContext(http.MethodGet, "/api/v1/auth/profile", nil)
			if tc.setContext {
				c.Set("user_id", tc.userID)
			}
			controller.GetProfile(c)

			if tc.expectSuccess {
				assertSuccessResponse(t, w, tc.expectedStatus)
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestAuthController_Logout(t *testing.T) {
	// 登出不依赖服务层，应始终成功并清除 Cookie
	mock := &mockAuthService{}
	controller := NewAuthController(mock)

	c, w := newTestContext(http.MethodPost, "/api/v1/auth/logout", nil)
	controller.Logout(c)

	assertSuccessResponse(t, w, http.StatusOK)
}

func TestAuthController_GetCSRFToken(t *testing.T) {
	tests := []struct {
		name           string
		cookieValue    string
		expectedStatus int
		expectSuccess  bool
	}{
		{
			name:           "有CSRF Cookie-返回成功",
			cookieValue:    "test-csrf-token-123",
			expectedStatus: http.StatusOK,
			expectSuccess:  true,
		},
		{
			name:           "无CSRF Cookie-返回未授权",
			cookieValue:    "",
			expectedStatus: http.StatusUnauthorized,
			expectSuccess:  false,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			mock := &mockAuthService{}
			controller := NewAuthController(mock)

			c, w := newTestContext(http.MethodGet, "/api/v1/auth/csrf", nil)
			if tc.cookieValue != "" {
				c.Request.AddCookie(&http.Cookie{Name: csrfCookieName, Value: tc.cookieValue})
			}
			controller.GetCSRFToken(c)

			if tc.expectSuccess {
				resp := assertSuccessResponse(t, w, tc.expectedStatus)
				data, ok := resp["data"].(map[string]interface{})
				require.True(t, ok)
				assert.Equal(t, tc.cookieValue, data["csrf_token"])
			} else {
				assertErrorResponse(t, w, tc.expectedStatus)
			}
		})
	}
}

func TestUserIDFromContext(t *testing.T) {
	tests := []struct {
		name      string
		ctxValue  interface{}
		exists    bool
		expected  uint
		expectOK  bool
	}{
		{"正常uint值", uint(42), true, 42, true},
		{"上下文中不存在", nil, false, 0, false},
		{"类型错误-string", "not-uint", true, 0, false},
		{"值为0", uint(0), true, 0, false},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			c, _ := gin.CreateTestContext(nil)
			if tc.exists {
				c.Set("user_id", tc.ctxValue)
			}
			id, ok := userIDFromContext(c)
			assert.Equal(t, tc.expectOK, ok)
			assert.Equal(t, tc.expected, id)
		})
	}
}

func TestGenerateCSRFToken(t *testing.T) {
	// 生成的令牌应为 64 字符 hex 字符串（32 字节）
	token := generateCSRFToken()
	assert.Len(t, token, 64)

	// 两次调用应生成不同的令牌
	token2 := generateCSRFToken()
	assert.NotEqual(t, token, token2)
}
