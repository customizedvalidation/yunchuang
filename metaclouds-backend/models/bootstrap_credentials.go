package models

import (
	"context"
	"crypto/rand"
	"fmt"
	"math/big"
	"os"
	"strings"

	"metaclouds-backend/pkg/logger"
)

// minBootstrapPasswordLen 是引导账户口令的最小长度。低于此长度的
// 配置值会被拒绝，避免运维图省事填入弱口令。
const minBootstrapPasswordLen = 12

// generatedPasswordLen 是自动生成的一次性口令长度。
const generatedPasswordLen = 24

// bootstrapPasswordCharset 排除了容易混淆的字符（0/O、1/l/I）以及
// 引号、反斜杠、空格，避免口令被人工转录错误或在 shell/YAML/.env
// 中触发解析歧义。
const bootstrapPasswordCharset = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*-_=+"

// productionEnvNames 列出被视为生产环境的取值。
var productionEnvNames = map[string]bool{
	"production": true,
	"prod":       true,
}

// isProductionEnv 判断当前是否运行在生产环境。
//
// 直接读取环境变量而不经过 config.Load()，因为引导默认账户发生在
// 配置装载的早期阶段，且这里只需要一个布尔判断，不值得引入
// config 装载的完整校验副作用。
func isProductionEnv() bool {
	for _, key := range []string{"ENVIRONMENT", "SERVER_ENV", "GO_ENV"} {
		if productionEnvNames[strings.ToLower(strings.TrimSpace(os.Getenv(key)))] {
			return true
		}
	}
	return false
}

// randomPassword 用密码学安全随机源生成指定长度的口令。
func randomPassword(length int) (string, error) {
	if length <= 0 {
		return "", fmt.Errorf("password length must be positive, got %d", length)
	}
	max := big.NewInt(int64(len(bootstrapPasswordCharset)))
	buf := make([]byte, length)
	for i := range buf {
		n, err := rand.Int(rand.Reader, max)
		if err != nil {
			return "", fmt.Errorf("failed to read secure random source: %w", err)
		}
		buf[i] = bootstrapPasswordCharset[n.Int64()]
	}
	return string(buf), nil
}

// bootstrapPassword 返回用于创建默认账户的口令。
//
// 语义分三种情况：
//   - 环境变量已设置且长度达标：直接使用。
//   - 未设置且运行在生产环境：返回错误，拒绝启动。绝不静默创建一个
//     口令可被预测的管理员账户。
//   - 未设置且运行在非生产环境：生成一次性随机口令并记录到日志，
//     方便本地开发登录。
//
// 此函数取代了原先「未配置则回退到硬编码常量」的实现。那些常量已经
// 随源码泄露，任何知晓仓库内容的人都能用它登录管理员账户。
func bootstrapPassword(envKey, accountName string) (string, error) {
	if v := strings.TrimSpace(os.Getenv(envKey)); v != "" {
		if len(v) < minBootstrapPasswordLen {
			return "", fmt.Errorf(
				"%s is too short: got %d characters, need at least %d",
				envKey, len(v), minBootstrapPasswordLen,
			)
		}
		return v, nil
	}

	if isProductionEnv() {
		return "", fmt.Errorf(
			"%s must be set in production; refusing to bootstrap account %q with an auto-generated password",
			envKey, accountName,
		)
	}

	generated, err := randomPassword(generatedPasswordLen)
	if err != nil {
		return "", fmt.Errorf("failed to generate bootstrap password for %q: %w", accountName, err)
	}

	// 仅非生产环境记录口令。生产路径在上面已经返回错误，不会走到这里。
	logger.WarnWithCtx(context.Background(),
		"Generated a one-time bootstrap password because the environment variable is unset",
		"account", accountName,
		"env_var", envKey,
		"password", generated,
		"hint", "set "+envKey+" to control this credential; this value is not persisted anywhere else",
	)

	return generated, nil
}
