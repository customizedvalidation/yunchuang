package logger

import (
	"context"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
)

type Logger struct {
	*log.Logger
	env      string
	logLevel LogLevel
}

type contextKey string

const RequestIDKey contextKey = "request_id"

func ParseLogLevel(levelStr string) LogLevel {
	switch strings.ToLower(levelStr) {
	case "debug":
		return LevelDebug
	case "warn":
		return LevelWarn
	case "error":
		return LevelError
	default:
		return LevelInfo
	}
}

type LoggerConfig struct {
	Env       string
	LogLevel  string
	LogOutput string
	LogPath   string
}

func InitLoggerWithConfig(cfg LoggerConfig) *Logger {
	var logger *log.Logger
	logLevel := ParseLogLevel(cfg.LogLevel)

	var output = os.Stdout
	if cfg.LogOutput == "file" && cfg.LogPath != "" {
		if err := ensureLogDir(cfg.LogPath); err != nil {
			log.Printf("Warning: failed to create log directory: %v, using stdout", err)
		} else {
			file, err := os.OpenFile(cfg.LogPath, os.O_APPEND|os.O_CREATE|os.O_WRONLY, 0644)
			if err != nil {
				log.Printf("Warning: failed to open log file: %v, using stdout", err)
			} else {
				output = file
			}
		}
	}

	flags := log.LstdFlags
	if cfg.Env != "production" {
		flags |= log.Lshortfile
	}

	logger = log.New(output, "", flags)
	return &Logger{Logger: logger, env: cfg.Env, logLevel: logLevel}
}

func InitLogger(env string, logLevelStr string) *Logger {
	return InitLoggerWithConfig(LoggerConfig{
		Env:       env,
		LogLevel:  logLevelStr,
		LogOutput: "console",
		LogPath:   "",
	})
}

func ensureLogDir(logPath string) error {
	dir := filepath.Dir(logPath)
	if dir == "." || dir == "/" || dir == "\\" {
		return nil
	}
	return os.MkdirAll(dir, 0755)
}

func (l *Logger) InfoWithCtx(ctx context.Context, msg string, fields ...interface{}) {
	reqID := getRequestID(ctx)
	if reqID != "" {
		l.Printf("[%s] INFO: %s", reqID, formatMessage(msg, fields...))
	} else {
		l.Printf("INFO: %s", formatMessage(msg, fields...))
	}
}

func (l *Logger) ErrorWithCtx(ctx context.Context, msg string, err error, fields ...interface{}) {
	reqID := getRequestID(ctx)
	if reqID != "" {
		if err != nil {
			allFields := append(fields, "error", err)
			l.Printf("[%s] ERROR: %s", reqID, formatMessage(msg, allFields...))
		} else {
			l.Printf("[%s] ERROR: %s", reqID, formatMessage(msg, fields...))
		}
	} else {
		if err != nil {
			allFields := append(fields, "error", err)
			l.Printf("ERROR: %s", formatMessage(msg, allFields...))
		} else {
			l.Printf("ERROR: %s", formatMessage(msg, fields...))
		}
	}
}

func (l *Logger) WarnWithCtx(ctx context.Context, msg string, fields ...interface{}) {
	reqID := getRequestID(ctx)
	if reqID != "" {
		l.Printf("[%s] WARN: %s", reqID, formatMessage(msg, fields...))
	} else {
		l.Printf("WARN: %s", formatMessage(msg, fields...))
	}
}

func (l *Logger) DebugWithCtx(ctx context.Context, msg string, fields ...interface{}) {
	if l.logLevel <= LevelDebug {
		reqID := getRequestID(ctx)
		if reqID != "" {
			l.Printf("[%s] DEBUG: %s", reqID, formatMessage(msg, fields...))
		} else {
			l.Printf("DEBUG: %s", formatMessage(msg, fields...))
		}
	}
}

func (l *Logger) WithRequestID(reqID string) *Logger {
	return &Logger{Logger: l.Logger, env: l.env}
}

func getRequestID(ctx context.Context) string {
	if ctx == nil {
		return ""
	}
	if reqID, ok := ctx.Value(RequestIDKey).(string); ok {
		return reqID
	}
	return ""
}

func formatMessage(msg string, fields ...interface{}) string {
	if len(fields) == 0 {
		return msg
	}
	result := msg
	for i := 0; i < len(fields); i += 2 {
		if i+1 < len(fields) {
			result += fmt.Sprintf(" %v=%v", fields[i], fields[i+1])
		} else {
			result += fmt.Sprintf(" %v", fields[i])
		}
	}
	return result
}

func GinMiddleware(logger *Logger) gin.HandlerFunc {
	return func(c *gin.Context) {
		start := time.Now()
		path := c.Request.URL.Path
		query := c.Request.URL.RawQuery
		reqID := c.GetHeader("X-Request-ID")
		if reqID == "" {
			reqID = c.GetString("request_id")
		}

		c.Next()

		// logger 为 nil（未调用 InitGlobalLogger 的场景，如测试）时安全跳过，
		// 避免在请求后处理阶段空指针 panic。
		if logger == nil {
			return
		}

		duration := time.Since(start)
		statusCode := c.Writer.Status()

		if reqID != "" {
			logger.Printf("[%s] HTTP %d %s %s?%s - %s", reqID, statusCode, c.Request.Method, path, query, duration)
		} else {
			logger.Printf("HTTP %d %s %s?%s - %s", statusCode, c.Request.Method, path, query, duration)
		}
	}
}

var Global *Logger

func InitGlobalLogger(env string, logLevel string) {
	Global = InitLogger(env, logLevel)
}

func InitGlobalLoggerWithConfig(cfg LoggerConfig) {
	Global = InitLoggerWithConfig(cfg)
}

func InfoWithCtx(ctx context.Context, msg string, fields ...interface{}) {
	if Global != nil {
		Global.InfoWithCtx(ctx, msg, fields...)
	} else {
		log.Println(formatMessage(msg, fields...))
	}
}

func ErrorWithCtx(ctx context.Context, msg string, err error, fields ...interface{}) {
	if Global != nil {
		Global.ErrorWithCtx(ctx, msg, err, fields...)
	} else {
		if err != nil {
			log.Printf("[ERROR] %s", formatMessage(msg+": %v", append(fields, err)...))
		} else {
			log.Printf("[ERROR] %s", formatMessage(msg, fields...))
		}
	}
}

func WarnWithCtx(ctx context.Context, msg string, fields ...interface{}) {
	if Global != nil {
		Global.WarnWithCtx(ctx, msg, fields...)
	} else {
		log.Printf("[WARN] %s", formatMessage(msg, fields...))
	}
}

func DebugWithCtx(ctx context.Context, msg string, fields ...interface{}) {
	if Global != nil {
		Global.DebugWithCtx(ctx, msg, fields...)
	} else {
		log.Printf("[DEBUG] %s", formatMessage(msg, fields...))
	}
}
