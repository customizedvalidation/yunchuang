package controllers

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/gin-gonic/gin"
	"github.com/stretchr/testify/assert"
)

// init 在测试运行前将 gin 设为 TestMode，避免调试日志干扰测试输出。
func init() {
	gin.SetMode(gin.TestMode)
}

// newTestContext 创建一个用于单元测试的 gin.Context 和 httptest.ResponseRecorder。
// method 和 path 仅用于日志展示，body 为请求体（可为 nil）。
func newTestContext(method, path string, body interface{}) (*gin.Context, *httptest.ResponseRecorder) {
	w := httptest.NewRecorder()
	c, _ := gin.CreateTestContext(w)

	var reqBody *bytes.Reader
	if body != nil {
		jsonBytes, _ := json.Marshal(body)
		reqBody = bytes.NewReader(jsonBytes)
	} else {
		reqBody = bytes.NewReader(nil)
	}

	req := httptest.NewRequest(method, path, reqBody)
	req.Header.Set("Content-Type", "application/json")
	c.Request = req
	return c, w
}

// newTestContextWithParams 创建带路径参数的测试上下文。
func newTestContextWithParams(method, path string, body interface{}, params gin.Params) (*gin.Context, *httptest.ResponseRecorder) {
	c, w := newTestContext(method, path, body)
	c.Params = params
	return c, w
}

// newTestContextWithAuth 创建带 JWT 中间件注入身份信息的测试上下文。
func newTestContextWithAuth(method, path string, body interface{}, userID, tenantID uint, role string) (*gin.Context, *httptest.ResponseRecorder) {
	c, w := newTestContext(method, path, body)
	c.Set("user_id", userID)
	c.Set("tenant_id", tenantID)
	c.Set("role", role)
	return c, w
}

// assertResponse 断言响应状态码并解析响应体为通用 map。
func assertResponse(t *testing.T, w *httptest.ResponseRecorder, expectedStatus int) map[string]interface{} {
	t.Helper()
	assert.Equal(t, expectedStatus, w.Code, "unexpected status code, body: %s", w.Body.String())

	var resp map[string]interface{}
	if w.Body.Len() > 0 {
		err := json.Unmarshal(w.Body.Bytes(), &resp)
		assert.NoError(t, err, "failed to parse response body")
	}
	return resp
}

// assertSuccessResponse 断言响应为成功（success=true）。
func assertSuccessResponse(t *testing.T, w *httptest.ResponseRecorder, expectedStatus int) map[string]interface{} {
	t.Helper()
	resp := assertResponse(t, w, expectedStatus)
	if expectedStatus != http.StatusNoContent {
		success, _ := resp["success"].(bool)
		assert.True(t, success, "expected success=true in response")
	}
	return resp
}

// assertErrorResponse 断言响应为错误（success=false）并返回消息。
func assertErrorResponse(t *testing.T, w *httptest.ResponseRecorder, expectedStatus int) string {
	t.Helper()
	resp := assertResponse(t, w, expectedStatus)
	success, _ := resp["success"].(bool)
	assert.False(t, success, "expected success=false in response")
	msg, _ := resp["message"].(string)
	return msg
}
