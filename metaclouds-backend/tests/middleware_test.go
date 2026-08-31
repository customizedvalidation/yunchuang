package tests

import (
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/gin-gonic/gin"
	"metaclouds-backend/pkg/middleware"
)

func TestRateLimiter(t *testing.T) {
	gin.SetMode(gin.TestMode)

	rl := middleware.NewRateLimiter(3, 1*time.Second)

	r := gin.New()
	r.Use(rl.Middleware())
	r.GET("/test", func(c *gin.Context) {
		c.Status(http.StatusOK)
	})

	for i := 0; i < 5; i++ {
		req := httptest.NewRequest(http.MethodGet, "/test", nil)
		w := httptest.NewRecorder()
		r.ServeHTTP(w, req)

		if i < 3 {
			if w.Code != http.StatusOK {
				t.Errorf("Expected status 200 on request %d, got %d", i, w.Code)
			}
		} else {
			if w.Code != http.StatusTooManyRequests {
				t.Errorf("Expected status 429 on request %d, got %d", i, w.Code)
			}
		}
	}
}

func TestCircuitBreaker(t *testing.T) {
	gin.SetMode(gin.TestMode)

	cb := middleware.NewCircuitBreaker(2, 1, 1*time.Second)

	r := gin.New()
	r.Use(cb.Middleware())
	r.GET("/test", func(c *gin.Context) {
		c.Status(http.StatusInternalServerError)
	})

	for i := 0; i < 3; i++ {
		req := httptest.NewRequest(http.MethodGet, "/test", nil)
		w := httptest.NewRecorder()
		r.ServeHTTP(w, req)

		if i < 2 {
			if w.Code != http.StatusInternalServerError {
				t.Errorf("Expected status 500 on request %d, got %d", i, w.Code)
			}
		} else {
			if w.Code != http.StatusServiceUnavailable {
				t.Errorf("Expected status 503 on request %d, got %d", i, w.Code)
			}
		}
	}

	time.Sleep(2 * time.Second)

	req := httptest.NewRequest(http.MethodGet, "/test", nil)
	w := httptest.NewRecorder()
	r.ServeHTTP(w, req)

	if w.Code != http.StatusInternalServerError {
		t.Errorf("Expected status 500 after timeout, got %d", w.Code)
	}
}

func TestValidationMiddleware(t *testing.T) {
	type TestStruct struct {
		Name  string `json:"name" validate:"required"`
		Email string `json:"email" validate:"required,email"`
	}

	testCases := []struct {
		name    string
		input   TestStruct
		wantErr bool
	}{
		{"valid", TestStruct{Name: "test", Email: "test@example.com"}, false},
		{"missing name", TestStruct{Email: "test@example.com"}, true},
		{"invalid email", TestStruct{Name: "test", Email: "invalid"}, true},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			err := middleware.ValidateStruct(tc.input)
			if (err != nil) != tc.wantErr {
				t.Errorf("ValidateStruct() error = %v, wantErr %v", err, tc.wantErr)
			}
		})
	}
}