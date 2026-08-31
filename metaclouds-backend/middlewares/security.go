package middlewares

import (
	"io"
	"net"
	"net/http"
	"regexp"
	"strings"
	"time"

	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/logger"
	"metaclouds-backend/pkg/response"

	"github.com/gin-gonic/gin"
)

var (
	sqlInjectionPattern = regexp.MustCompile(`(?i)(\b(select|insert|update|delete|drop|truncate|union|exec|execute|sp_|xp_|xp_|dbms_)\b.*\b(from|into|table|where|union|join|on|having|group\s+by|order\s+by|limit|offset)\b|\b(and|or)\b\s*[=<>!]+\s*['"]|\b(union\s+select|select\s+.+from|insert\s+into|update\s+.+set|delete\s+from|drop\s+table)\b)`)
	xssPattern          = regexp.MustCompile(`(?i)(<script[^>]*>.*?</script>|<iframe[^>]*>.*?</iframe>|<img[^>]*on\w+\s*=|javascript:|vbscript:|data:text/html|expression\(|on\w+\s*=\s*["']?[^"']*["']?)`)
	sensitiveHeaders    = []string{"authorization", "cookie", "x-api-key"}
)

func SecurityFilter() gin.HandlerFunc {
	return func(c *gin.Context) {
		start := time.Now()
		clientIP := c.ClientIP()
		method := c.Request.Method
		path := c.Request.URL.Path
		query := c.Request.URL.RawQuery

		logger.DebugWithCtx(c, "Security filter started",
			"client_ip", clientIP,
			"method", method,
			"path", path,
			"query", query)

		if !isValidIP(clientIP) {
			logger.WarnWithCtx(c, "Security filter blocked - Invalid client IP address",
				"client_ip", clientIP,
				"method", method,
				"path", path,
				"duration", time.Since(start))
			response.Error(c, errors.BadRequest("Invalid client IP"))
			c.Abort()
			return
		}

		if err := checkSQLInjection(c); err != nil {
			logger.WarnWithCtx(c, "Security filter blocked - SQL injection attempt detected",
				"client_ip", clientIP,
				"method", method,
				"path", path,
				"query", query,
				"duration", time.Since(start))
			response.Error(c, errors.BadRequest("Invalid input detected"))
			c.Abort()
			return
		}

		if err := checkXSS(c); err != nil {
			logger.WarnWithCtx(c, "Security filter blocked - XSS attempt detected",
				"client_ip", clientIP,
				"method", method,
				"path", path,
				"query", query,
				"duration", time.Since(start))
			response.Error(c, errors.BadRequest("Invalid input detected"))
			c.Abort()
			return
		}

		logger.DebugWithCtx(c, "Security filter passed",
			"client_ip", clientIP,
			"method", method,
			"path", path,
			"duration", time.Since(start))

		c.Next()
	}
}

func isValidIP(ip string) bool {
	if ip == "" {
		return false
	}
	return net.ParseIP(ip) != nil
}

func checkSQLInjection(c *gin.Context) error {
	for _, param := range c.Request.URL.Query() {
		for _, value := range param {
			if sqlInjectionPattern.MatchString(value) {
				return errors.BadRequest("SQL injection detected")
			}
		}
	}

	for _, param := range c.Params {
		if sqlInjectionPattern.MatchString(param.Value) {
			return errors.BadRequest("SQL injection detected")
		}
	}

	return checkRequestBodyForPattern(c, sqlInjectionPattern)
}

func checkXSS(c *gin.Context) error {
	for _, param := range c.Request.URL.Query() {
		for _, value := range param {
			if xssPattern.MatchString(value) {
				return errors.BadRequest("XSS attempt detected")
			}
		}
	}

	for _, param := range c.Params {
		if xssPattern.MatchString(param.Value) {
			return errors.BadRequest("XSS attempt detected")
		}
	}

	return checkRequestBodyForPattern(c, xssPattern)
}

func checkRequestBodyForPattern(c *gin.Context, pattern *regexp.Regexp) error {
	contentType := c.Request.Header.Get("Content-Type")
	if !strings.Contains(contentType, "application/json") && 
	   !strings.Contains(contentType, "application/x-www-form-urlencoded") &&
	   !strings.Contains(contentType, "multipart/form-data") {
		return nil
	}

	body, err := io.ReadAll(c.Request.Body)
	if err != nil {
		return nil
	}

	c.Request.Body.Close()
	c.Request.Body = io.NopCloser(strings.NewReader(string(body)))

	if pattern.Match(body) {
		return errors.BadRequest("Malicious input detected")
	}

	return nil
}

func SanitizeInput(input string) string {
	input = strings.ReplaceAll(input, "<", "&lt;")
	input = strings.ReplaceAll(input, ">", "&gt;")
	input = strings.ReplaceAll(input, "\"", "&quot;")
	input = strings.ReplaceAll(input, "'", "&#39;")
	input = strings.ReplaceAll(input, "&", "&amp;")
	return input
}

func RateLimitByIP() gin.HandlerFunc {
	return func(c *gin.Context) {
		clientIP := c.ClientIP()

		if clientIP == "" {
			response.Error(c, errors.BadRequest("Client IP not found"))
			c.Abort()
			return
		}

		c.Next()
	}
}

func MaskSensitiveHeaders(headers http.Header) map[string]string {
	result := make(map[string]string)
	for key, values := range headers {
		lowerKey := strings.ToLower(key)
		isSensitive := false
		for _, sensitive := range sensitiveHeaders {
			if lowerKey == sensitive || strings.Contains(lowerKey, "token") || strings.Contains(lowerKey, "secret") {
				isSensitive = true
				break
			}
		}

		if isSensitive && len(values) > 0 {
			result[key] = maskValue(values[0])
		} else {
			result[key] = values[0]
		}
	}
	return result
}

func maskValue(value string) string {
	if len(value) <= 8 {
		return "[REDACTED]"
	}
	return value[:4] + "..." + value[len(value)-4:]
}