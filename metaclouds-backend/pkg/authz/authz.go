package authz

import (
	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/errors"
	"metaclouds-backend/pkg/logger"
	"metaclouds-backend/pkg/response"
)

type Permission string

const (
	PermissionClusterRead      Permission = "cluster:read"
	PermissionClusterWrite     Permission = "cluster:write"
	PermissionResourceRead     Permission = "resource:read"
	PermissionResourceWrite    Permission = "resource:write"
	PermissionJobRead          Permission = "job:read"
	PermissionJobWrite         Permission = "job:write"
	PermissionJobSubmit        Permission = "job:submit"
	PermissionTenantRead       Permission = "tenant:read"
	PermissionTenantWrite      Permission = "tenant:write"
	PermissionMonitoringRead   Permission = "monitoring:read"
	PermissionMonitoringWrite  Permission = "monitoring:write"
	PermissionAccelRead        Permission = "acceleration:read"
	PermissionAccelWrite       Permission = "acceleration:write"
	PermissionSecurityRead     Permission = "security:read"
	PermissionSecurityWrite    Permission = "security:write"
	PermissionAdmin            Permission = "admin"
)

type Role string

const (
	RoleAdmin     Role = "admin"
	RoleManager   Role = "manager"
	RoleUser      Role = "user"
)

var rolePermissions = map[Role][]Permission{
	RoleAdmin: {
		PermissionClusterRead,
		PermissionClusterWrite,
		PermissionResourceRead,
		PermissionResourceWrite,
		PermissionJobRead,
		PermissionJobWrite,
		PermissionTenantRead,
		PermissionTenantWrite,
		PermissionAdmin,
	},
	RoleManager: {
		PermissionClusterRead,
		PermissionClusterWrite,
		PermissionResourceRead,
		PermissionResourceWrite,
		PermissionJobRead,
		PermissionJobWrite,
		PermissionJobSubmit,
		PermissionTenantRead,
		PermissionMonitoringRead,
		PermissionMonitoringWrite,
		PermissionAccelRead,
		PermissionAccelWrite,
		PermissionSecurityRead,
	},
	RoleUser: {
		PermissionClusterRead,
		PermissionResourceRead,
		PermissionJobRead,
		PermissionMonitoringRead,
		PermissionAccelRead,
		PermissionSecurityRead,
	},
}

func GetRolePermissions(role Role) []Permission {
	return rolePermissions[role]
}

// HasPermission 判断角色是否具备某项权限。
//
// 未知角色（不在 rolePermissions 中）一律返回 false —— 权限判定保持 fail-closed，
// 避免新增角色时因漏配而意外获得越权访问。
func HasPermission(role Role, permission Permission) bool {
	if role == RoleAdmin {
		return true
	}

	for _, p := range rolePermissions[role] {
		if p == permission {
			return true
		}
	}
	return false
}

// roleFromContext 从 gin 上下文中取出 JWT 中间件写入的角色。
func roleFromContext(c *gin.Context) (Role, bool) {
	roleValue, exists := c.Get("role")
	if !exists {
		return "", false
	}

	roleStr, ok := roleValue.(string)
	if !ok || roleStr == "" {
		return "", false
	}
	return Role(roleStr), true
}

// abortWithError 统一错误响应契约。
//
// 此前直接 c.AbortWithStatusJSON(status, appErr) 会把 AppError 结构体整体序列化为
// {"Code":..,"Message":..,"Err":..,"Details":..}，与全局 Response{Success,Message,Code,Timestamp}
// 契约不一致，前端按统一结构解析错误时会取不到 message。改用 response.Error 保证一致。
func abortWithError(c *gin.Context, err *errors.AppError) {
	response.Error(c, err)
	c.Abort()
}

// RequirePermission 要求请求方具备指定权限，否则中断请求。
func RequirePermission(permission Permission) gin.HandlerFunc {
	return func(c *gin.Context) {
		role, ok := roleFromContext(c)
		if !ok {
			abortWithError(c, errors.Unauthorized("Authorization header is required"))
			return
		}

		if !HasPermission(role, permission) {
			logger.WarnWithCtx(c, "Authorization denied - insufficient permission",
				"role", string(role),
				"required_permission", string(permission),
				"path", c.Request.URL.Path)
			abortWithError(c, errors.Forbidden("Insufficient permissions"))
			return
		}

		c.Next()
	}
}

func RequireAdmin() gin.HandlerFunc {
	return RequirePermission(PermissionAdmin)
}

// RequireRole 要求请求方属于指定角色之一（admin 始终放行）。
func RequireRole(requiredRoles ...Role) gin.HandlerFunc {
	return func(c *gin.Context) {
		role, ok := roleFromContext(c)
		if !ok {
			abortWithError(c, errors.Unauthorized("Authorization header is required"))
			return
		}

		isAllowed := role == RoleAdmin
		if !isAllowed {
			for _, r := range requiredRoles {
				if role == r {
					isAllowed = true
					break
				}
			}
		}

		if !isAllowed {
			logger.WarnWithCtx(c, "Authorization denied - role not allowed",
				"role", string(role),
				"required_roles", requiredRoles,
				"path", c.Request.URL.Path)
			abortWithError(c, errors.Forbidden("Insufficient permissions"))
			return
		}

		c.Next()
	}
}