package errors

import (
	"fmt"
	"net/http"
)

// ClashError 定义 Clash-Terminal 错误类型
type ClashError struct {
	Code    int    // 错误码
	Message string // 用户友好的错误消息
	Err     error  // 原始错误
}

func (e *ClashError) Error() string {
	if e.Err != nil {
		return fmt.Sprintf("%s: %v", e.Message, e.Err)
	}
	return e.Message
}

func (e *ClashError) Unwrap() error {
	return e.Err
}

// 预定义错误码
const (
	ErrCodeKernelNotRunning = 1001
	ErrCodeConfigNotFound   = 1002
	ErrCodeAPIRequestFailed = 1003
	ErrCodeInvalidInput     = 1004
	ErrCodePermissionDenied = 1005
)

// 预定义错误
var (
	ErrKernelNotRunning = &ClashError{
		Code:    ErrCodeKernelNotRunning,
		Message: "Mihomo kernel is not running",
	}

	ErrConfigNotFound = &ClashError{
		Code:    ErrCodeConfigNotFound,
		Message: "Configuration file not found",
	}

	ErrPermissionDenied = &ClashError{
		Code:    ErrCodePermissionDenied,
		Message: "Permission denied (try running with sudo)",
	}
)

// NewAPIError 创建 API 请求错误
func NewAPIError(resp *http.Response, err error) *ClashError {
	return &ClashError{
		Code:    ErrCodeAPIRequestFailed,
		Message: fmt.Sprintf("API request failed (status: %d)", resp.StatusCode),
		Err:     err,
	}
}

// NewValidationError 创建输入验证错误
func NewValidationError(field, reason string) *ClashError {
	return &ClashError{
		Code:    ErrCodeInvalidInput,
		Message: fmt.Sprintf("Invalid %s: %s", field, reason),
	}
}
