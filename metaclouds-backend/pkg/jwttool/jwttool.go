package jwttool

import (
	"fmt"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

type TokenGenerator struct {
	secret          string
	defaultExpHours int
}

type TokenClaims struct {
	UserID   uint   `json:"user_id"`
	Username string `json:"username"`
	Email    string `json:"email"`
	Role     string `json:"role"`
	TenantID uint   `json:"tenant_id"`
}

func NewTokenGenerator(secret string, defaultExpHours int) *TokenGenerator {
	return &TokenGenerator{
		secret:          secret,
		defaultExpHours: defaultExpHours,
	}
}

func (tg *TokenGenerator) GenerateToken(claims TokenClaims) (string, int64, error) {
	return tg.GenerateTokenWithExpiration(claims, time.Duration(tg.defaultExpHours)*time.Hour)
}

func (tg *TokenGenerator) GenerateTokenWithExpiration(claims TokenClaims, expiration time.Duration) (string, int64, error) {
	expirationTime := time.Now().Add(expiration)

	jwtClaims := jwt.MapClaims{
		"user_id":   claims.UserID,
		"username":  claims.Username,
		"email":     claims.Email,
		"role":      claims.Role,
		"tenant_id": claims.TenantID,
		"exp":       expirationTime.Unix(),
		"iat":       time.Now().Unix(),
		"jti":       fmt.Sprintf("%d", time.Now().UnixNano()),
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, jwtClaims)

	tokenString, err := token.SignedString([]byte(tg.secret))
	if err != nil {
		return "", 0, err
	}

	return tokenString, expirationTime.Unix(), nil
}

func GenerateTokenFromMapClaims(claims jwt.MapClaims, secret string, expirationHours int) (string, error) {
	expirationTime := time.Now().Add(time.Duration(expirationHours) * time.Hour)
	claims["exp"] = expirationTime.Unix()
	claims["iat"] = time.Now().Unix()
	claims["jti"] = fmt.Sprintf("%d", time.Now().UnixNano())

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)

	return token.SignedString([]byte(secret))
}
