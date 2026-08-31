package errors

import (
	"fmt"
	"net/http"

	"github.com/go-playground/validator/v10"
)

type ErrorCode int

const (
	ErrUnknown ErrorCode = iota
	ErrBadRequest
	ErrUnauthorized
	ErrForbidden
	ErrNotFound
	ErrConflict
	ErrInternalServer
	ErrValidation
	ErrRateLimit
	ErrServiceUnavailable
)

func (e ErrorCode) String() string {
	switch e {
	case ErrBadRequest:
		return "BAD_REQUEST"
	case ErrUnauthorized:
		return "UNAUTHORIZED"
	case ErrForbidden:
		return "FORBIDDEN"
	case ErrNotFound:
		return "NOT_FOUND"
	case ErrConflict:
		return "CONFLICT"
	case ErrInternalServer:
		return "INTERNAL_SERVER_ERROR"
	case ErrValidation:
		return "VALIDATION_ERROR"
	case ErrRateLimit:
		return "RATE_LIMIT_EXCEEDED"
	case ErrServiceUnavailable:
		return "SERVICE_UNAVAILABLE"
	default:
		return "UNKNOWN_ERROR"
	}
}

func (e ErrorCode) HTTPStatus() int {
	switch e {
	case ErrBadRequest:
		return http.StatusBadRequest
	case ErrUnauthorized:
		return http.StatusUnauthorized
	case ErrForbidden:
		return http.StatusForbidden
	case ErrNotFound:
		return http.StatusNotFound
	case ErrConflict:
		return http.StatusConflict
	case ErrInternalServer:
		return http.StatusInternalServerError
	case ErrValidation:
		return http.StatusBadRequest
	case ErrRateLimit:
		return http.StatusTooManyRequests
	case ErrServiceUnavailable:
		return http.StatusServiceUnavailable
	default:
		return http.StatusInternalServerError
	}
}

type AppError struct {
	Code    ErrorCode
	Message string
	Err     error
	Details interface{}
}

func (e *AppError) Error() string {
	if e.Err != nil {
		return fmt.Sprintf("[%s] %s: %v", e.Code.String(), e.Message, e.Err)
	}
	return fmt.Sprintf("[%s] %s", e.Code.String(), e.Message)
}

func (e *AppError) Unwrap() error {
	return e.Err
}

func (e *AppError) HTTPStatus() int {
	return e.Code.HTTPStatus()
}

func New(code ErrorCode, message string) *AppError {
	return &AppError{
		Code:    code,
		Message: message,
	}
}

func NewWithError(code ErrorCode, message string, err error) *AppError {
	return &AppError{
		Code:    code,
		Message: message,
		Err:     err,
	}
}

func NewWithDetails(code ErrorCode, message string, details interface{}) *AppError {
	return &AppError{
		Code:    code,
		Message: message,
		Details: details,
	}
}

func BadRequest(message string) *AppError {
	return New(ErrBadRequest, message)
}

func Unauthorized(message string) *AppError {
	return New(ErrUnauthorized, message)
}

func Forbidden(message string) *AppError {
	return New(ErrForbidden, message)
}

func NotFound(message string) *AppError {
	return New(ErrNotFound, message)
}

func Conflict(message string) *AppError {
	return New(ErrConflict, message)
}

func InternalServer(message string) *AppError {
	return New(ErrInternalServer, message)
}

func Validation(message string) *AppError {
	return New(ErrValidation, message)
}

func RateLimit(message string) *AppError {
	return New(ErrRateLimit, message)
}

func NewRateLimitError(message string) error {
	return New(ErrRateLimit, message)
}

func NewServiceUnavailableError(message string) error {
	return New(ErrServiceUnavailable, message)
}

func FromError(err error) *AppError {
	if appErr, ok := GetAppError(err); ok {
		return appErr
	}
	if _, ok := err.(validator.ValidationErrors); ok {
		return New(ErrValidation, err.Error())
	}
	return New(ErrInternalServer, err.Error())
}

func IsAppError(err error) bool {
	_, ok := err.(*AppError)
	return ok
}

func GetAppError(err error) (*AppError, bool) {
	e, ok := err.(*AppError)
	return e, ok
}
