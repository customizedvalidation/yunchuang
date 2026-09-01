package middlewares

import (
	"fmt"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/golang-jwt/jwt/v5"

	"metaclouds-backend/config"
	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/logger"
	"metaclouds-backend/pkg/response"
)

type JWTAuthConfig struct {
	Secret          string
	ExpirationHours int
}

func NewJWTAuth(cfg *config.Config) gin.HandlerFunc {
	authConfig := &JWTAuthConfig{
		Secret:          cfg.JWTSecret,
		ExpirationHours: cfg.JWTExpirationHours,
	}
	return jwtAuthHandler(authConfig)
}

func jwtAuthHandler(cfg *JWTAuthConfig) gin.HandlerFunc {
	return func(c *gin.Context) {
		start := time.Now()
		clientIP := c.ClientIP()
		path := c.Request.URL.Path
		method := c.Request.Method

		logger.DebugWithCtx(c, "JWT authentication attempt",
			"client_ip", clientIP,
			"path", path,
			"method", method)

		authHeader := c.GetHeader("Authorization")
		if authHeader == "" {
			logger.WarnWithCtx(c, "JWT authentication failed - missing Authorization header",
				"client_ip", clientIP,
				"path", path,
				"method", method,
				"error", "Authorization header is required")
			response.Error(c, errors.Unauthorized("Authorization header is required"))
			c.Abort()
			return
		}

		parts := strings.SplitN(authHeader, " ", 2)
		if !(len(parts) == 2 && parts[0] == "Bearer") {
			logger.WarnWithCtx(c, "JWT authentication failed - invalid header format",
				"client_ip", clientIP,
				"path", path,
				"method", method,
				"error", "Authorization header format must be Bearer {token}",
				"header", maskToken(authHeader))
			response.Error(c, errors.BadRequest("Authorization header format must be Bearer {token}"))
			c.Abort()
			return
		}

		tokenString := parts[1]
		tokenLength := len(tokenString)

		token, err := jwt.Parse(tokenString, func(token *jwt.Token) (interface{}, error) {
			if _, ok := token.Method.(*jwt.SigningMethodHMAC); !ok {
				logger.WarnWithCtx(c, "JWT authentication failed - unexpected signing method",
					"client_ip", clientIP,
					"path", path,
					"method", method,
					"alg", token.Header["alg"])
				return nil, fmt.Errorf("unexpected signing method: %v", token.Header["alg"])
			}
			return []byte(cfg.Secret), nil
		})

		if err != nil {
			errorType := classifyJWTError(err)
			logger.WarnWithCtx(c, "JWT authentication failed - token parse error",
				"client_ip", clientIP,
				"path", path,
				"method", method,
				"error_type", errorType,
				"error", err.Error(),
				"token_length", tokenLength)
			response.Error(c, errors.Unauthorized(fmt.Sprintf("Invalid or expired token: %s", errorType)))
			c.Abort()
			return
		}

		if !token.Valid {
			logger.WarnWithCtx(c, "JWT authentication failed - invalid token",
				"client_ip", clientIP,
				"path", path,
				"method", method,
				"token_length", tokenLength)
			response.Error(c, errors.Unauthorized("Invalid token"))
			c.Abort()
			return
		}

		claims, ok := token.Claims.(jwt.MapClaims)
		if !ok {
			logger.WarnWithCtx(c, "JWT authentication failed - invalid claims type",
				"client_ip", clientIP,
				"path", path,
				"method", method)
			response.Error(c, errors.Unauthorized("Invalid token claims"))
			c.Abort()
			return
		}

		requiredClaims := []string{"user_id", "username", "role", "exp", "iat", "jti"}
		for _, claim := range requiredClaims {
			if _, exists := claims[claim]; !exists {
				logger.WarnWithCtx(c, "JWT authentication failed - missing required claim",
					"client_ip", clientIP,
					"path", path,
					"method", method,
					"missing_claim", claim)
				response.Error(c, errors.Unauthorized(fmt.Sprintf("Missing required claim: %s", claim)))
				c.Abort()
				return
			}
		}

		// 注：此处曾对临近过期的令牌静默签发新令牌并通过 X-Refresh-Token 响应头返回。
		// 该机制存在三重问题，已移除：
		//   1) 无消费者——前端从未读取该头部，且 CORS ExposeHeaders 未暴露它，跨域下浏览器无法读取；
		//   2) 破坏绝对会话上限——旧令牌不失效，只要持续发请求即可无限续期，被盗令牌可永久存活；
		//   3) 无谓开销——有效期内最后 1 小时内的每个请求都要做一次 HMAC 签名。
		//
		// 关于续期路径，务必按以下语义理解，不要写成"过期后刷新"：
		//   - POST /api/v1/auth/refresh 同样挂在 JWTAuth 之后（见 api/routes.go），
		//     因此只能用【尚未过期】的令牌调用：在有效期内换取一份有效期重新计算的令牌。
		//   - 令牌一旦过期，前端没有任何自救通道，必须重新登录。
		//   - 已知取舍：JWT 无状态且没有黑名单，refresh 签发的新令牌不会让旧令牌失效，
		//     旧令牌在其自身 exp 到来前仍然可用。因此 refresh 只是免除"活跃用户被迫重登"，
		//     它既不能缩短被盗令牌的存活窗口，也不能作为吊销手段；
		//     缩短该窗口需要引入令牌黑名单或短 exp + 服务端会话，此处未做。
		userIDVal, ok := claims["user_id"].(float64)
		if !ok || userIDVal <= 0 || userIDVal != float64(uint(userIDVal)) {
			logger.WarnWithCtx(c, "JWT authentication failed - invalid user_id",
				"client_ip", clientIP,
				"path", path,
				"method", method,
				"user_id_value", userIDVal)
			response.Error(c, errors.Unauthorized("Invalid token claims"))
			c.Abort()
			return
		}
		userID := uint(userIDVal)

		username, ok := claims["username"].(string)
		if !ok || username == "" {
			logger.WarnWithCtx(c, "JWT authentication failed - invalid username",
				"client_ip", clientIP,
				"path", path,
				"method", method)
			response.Error(c, errors.Unauthorized("Invalid token claims"))
			c.Abort()
			return
		}

		role, ok := claims["role"].(string)
		if !ok || role == "" {
			logger.WarnWithCtx(c, "JWT authentication failed - invalid role",
				"client_ip", clientIP,
				"path", path,
				"method", method)
			response.Error(c, errors.Unauthorized("Invalid token claims"))
			c.Abort()
			return
		}

		var tenantID uint
		if tenantIDVal, ok := claims["tenant_id"].(float64); ok {
			if tenantIDVal < 0 || tenantIDVal != float64(uint(tenantIDVal)) {
				logger.WarnWithCtx(c, "JWT authentication failed - invalid tenant_id value",
					"client_ip", clientIP,
					"path", path,
					"method", method,
					"tenant_id_value", tenantIDVal)
				response.Error(c, errors.Unauthorized("Invalid token claims"))
				c.Abort()
				return
			}
			tenantID = uint(tenantIDVal)
		} else if claims["tenant_id"] == nil {
			tenantID = 0
		} else {
			logger.WarnWithCtx(c, "JWT authentication failed - invalid tenant_id type",
				"client_ip", clientIP,
				"path", path,
				"method", method)
			response.Error(c, errors.Unauthorized("Invalid token claims"))
			c.Abort()
			return
		}

		email, _ := claims["email"].(string)

		c.Set("user_id", userID)
		c.Set("username", username)
		c.Set("email", email)
		c.Set("role", role)
		c.Set("tenant_id", tenantID)

		duration := time.Since(start)
		logger.InfoWithCtx(c, "JWT authentication successful",
			"client_ip", clientIP,
			"path", path,
			"method", method,
			"user_id", userID,
			"username", username,
			"role", role,
			"duration", duration)

		c.Next()
	}
}

func classifyJWTError(err error) string {
	switch {
	case strings.Contains(err.Error(), "token is expired"):
		return "token_expired"
	case strings.Contains(err.Error(), "signature is invalid"):
		return "invalid_signature"
	case strings.Contains(err.Error(), "unexpected signing method"):
		return "invalid_signing_method"
	case strings.Contains(err.Error(), "malformed"):
		return "malformed_token"
	case strings.Contains(err.Error(), "keyfunc returned error"):
		return "keyfunc_error"
	default:
		return "unknown_error"
	}
}

func JWTAuth() gin.HandlerFunc {
	cfg, err := config.LoadConfig()
	if err != nil {
		return func(c *gin.Context) {
			response.Error(c, errors.InternalServer("Failed to load config"))
			c.Abort()
		}
	}
	return NewJWTAuth(cfg)
}

func GetUserIDFromContext(c *gin.Context) (uint, error) {
	userID, exists := c.Get("user_id")
	if !exists {
		return 0, errors.Unauthorized("user_id not found in context")
	}
	id, ok := userID.(uint)
	if !ok {
		return 0, errors.BadRequest("user_id is not a valid uint")
	}
	return id, nil
}

func GetUserRoleFromContext(c *gin.Context) (string, error) {
	role, exists := c.Get("role")
	if !exists {
		return "", errors.Unauthorized("role not found in context")
	}
	return role.(string), nil
}

func GetTenantIDFromContext(c *gin.Context) (uint, error) {
	tenantID, exists := c.Get("tenant_id")
	if !exists {
		return 0, errors.Unauthorized("tenant_id not found in context")
	}
	id, ok := tenantID.(uint)
	if !ok {
		return 0, errors.BadRequest("tenant_id is not a valid uint")
	}
	return id, nil
}

func maskToken(token string) string {
	if token == "" {
		return ""
	}
	parts := strings.SplitN(token, " ", 2)
	if len(parts) == 2 && parts[0] == "Bearer" {
		tokenPart := parts[1]
		if len(tokenPart) <= 16 {
			return "Bearer [REDACTED]"
		}
		return "Bearer " + tokenPart[:8] + "..." + tokenPart[len(tokenPart)-8:]
	}
	return "[REDACTED]"
}
