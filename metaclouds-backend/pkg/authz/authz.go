package authz

import (
	"github.com/gin-gonic/gin"

	"metaclouds-backend/pkg/errors"
)

type Permission string

const (
	PermissionClusterRead   Permission = "cluster:read"
	PermissionClusterWrite  Permission = "cluster:write"
	PermissionResourceRead  Permission = "resource:read"
	PermissionResourceWrite Permission = "resource:write"
	PermissionJobRead       Permission = "job:read"
	PermissionJobWrite      Permission = "job:write"
	PermissionTenantRead    Permission = "tenant:read"
	PermissionTenantWrite   Permission = "tenant:write"
	PermissionAdmin         Permission = "admin"
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
		PermissionTenantRead,
	},
	RoleUser: {
		PermissionClusterRead,
		PermissionResourceRead,
		PermissionJobRead,
	},
}

func GetRolePermissions(role Role) []Permission {
	return rolePermissions[role]
}

func HasPermission(role Role, permission Permission) bool {
	permissions := rolePermissions[role]
	for _, p := range permissions {
		if p == permission || p == PermissionAdmin {
			return true
		}
	}
	return false
}

func RequirePermission(permission Permission) gin.HandlerFunc {
	return func(c *gin.Context) {
		roleValue, exists := c.Get("role")
		if !exists {
			c.AbortWithStatusJSON(401, errors.Unauthorized("Unauthorized"))
			return
		}

		role := Role(roleValue.(string))
		if !HasPermission(role, permission) {
			c.AbortWithStatusJSON(403, errors.Forbidden("Insufficient permissions"))
			return
		}

		c.Next()
	}
}

func RequireAdmin() gin.HandlerFunc {
	return RequirePermission(PermissionAdmin)
}

func RequireRole(requiredRoles ...Role) gin.HandlerFunc {
	return func(c *gin.Context) {
		roleValue, exists := c.Get("role")
		if !exists {
			c.AbortWithStatusJSON(401, errors.Unauthorized("Unauthorized"))
			return
		}

		userRole := Role(roleValue.(string))
		isAllowed := false
		for _, role := range requiredRoles {
			if userRole == role || userRole == RoleAdmin {
				isAllowed = true
				break
			}
		}

		if !isAllowed {
			c.AbortWithStatusJSON(403, errors.Forbidden("Insufficient permissions"))
			return
		}

		c.Next()
	}
}