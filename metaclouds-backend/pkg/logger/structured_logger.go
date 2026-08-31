package logger

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"sync"
	"time"

	"metaclouds-backend/pkg/tracing"
)

type LogLevel int

const (
	LevelDebug LogLevel = iota
	LevelInfo
	LevelWarn
	LevelError
	LevelPanic
)

func (l LogLevel) String() string {
	switch l {
	case LevelDebug:
		return "DEBUG"
	case LevelInfo:
		return "INFO"
	case LevelWarn:
		return "WARN"
	case LevelError:
		return "ERROR"
	case LevelPanic:
		return "PANIC"
	default:
		return "INFO"
	}
}

type StructuredLogger struct {
	mu        sync.Mutex
	writer    io.Writer
	level     LogLevel
	service   string
	version   string
	sampleRate float64
}

func NewStructuredLogger(service, version string, level LogLevel, sampleRate float64) *StructuredLogger {
	return &StructuredLogger{
		writer:    os.Stdout,
		level:     level,
		service:   service,
		version:   version,
		sampleRate: sampleRate,
	}
}

type logEntry struct {
	Timestamp   string                 `json:"timestamp"`
	Level       string                 `json:"level"`
	Service     string                 `json:"service"`
	Version     string                 `json:"version"`
	TraceID     string                 `json:"trace_id,omitempty"`
	SpanID      string                 `json:"span_id,omitempty"`
	RequestID   string                 `json:"request_id,omitempty"`
	Message     string                 `json:"message"`
	Error       string                 `json:"error,omitempty"`
	Stack       string                 `json:"stack,omitempty"`
	Fields      map[string]interface{} `json:"fields,omitempty"`
	Caller      string                 `json:"caller,omitempty"`
}

func (l *StructuredLogger) shouldSample() bool {
	if l.sampleRate >= 1.0 {
		return true
	}
	return false
}

func (l *StructuredLogger) Debug(ctx context.Context, msg string, fields ...interface{}) {
	if l.level > LevelDebug {
		return
	}
	if !l.shouldSample() {
		return
	}
	l.log(ctx, LevelDebug, msg, nil, fields...)
}

func (l *StructuredLogger) Info(ctx context.Context, msg string, fields ...interface{}) {
	if l.level > LevelInfo {
		return
	}
	l.log(ctx, LevelInfo, msg, nil, fields...)
}

func (l *StructuredLogger) Warn(ctx context.Context, msg string, fields ...interface{}) {
	if l.level > LevelWarn {
		return
	}
	l.log(ctx, LevelWarn, msg, nil, fields...)
}

func (l *StructuredLogger) Error(ctx context.Context, msg string, err error, fields ...interface{}) {
	if l.level > LevelError {
		return
	}
	l.log(ctx, LevelError, msg, err, fields...)
}

func (l *StructuredLogger) Panic(ctx context.Context, msg string, err error, fields ...interface{}) {
	l.log(ctx, LevelPanic, msg, err, fields...)
	panic(fmt.Sprintf("%s: %v", msg, err))
}

func (l *StructuredLogger) log(ctx context.Context, level LogLevel, msg string, err error, fields ...interface{}) {
	entry := logEntry{
		Timestamp: time.Now().UTC().Format(time.RFC3339),
		Level:     level.String(),
		Service:   l.service,
		Version:   l.version,
		Message:   msg,
	}

	if ctx != nil {
		entry.TraceID = tracing.GetTraceID(ctx)
		entry.SpanID = tracing.GetSpanID(ctx)
		if requestID, ok := ctx.Value("request_id").(string); ok {
			entry.RequestID = requestID
		}
	}

	if err != nil {
		entry.Error = err.Error()
	}

	if len(fields) > 0 {
		entry.Fields = make(map[string]interface{})
		for i := 0; i < len(fields); i += 2 {
			if i+1 < len(fields) {
				key, ok := fields[i].(string)
				if ok {
					entry.Fields[key] = fields[i+1]
				}
			}
		}
	}

	l.mu.Lock()
	defer l.mu.Unlock()

	data, _ := json.Marshal(entry)
	fmt.Fprintln(l.writer, string(data))
}

func (l *StructuredLogger) SetLevel(level LogLevel) {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.level = level
}

func (l *StructuredLogger) SetWriter(writer io.Writer) {
	l.mu.Lock()
	defer l.mu.Unlock()
	l.writer = writer
}