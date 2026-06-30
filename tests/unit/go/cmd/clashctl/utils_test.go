package main_test

import (
	"fmt"
	"net/url"
	"os"
	"testing"
	"time"
)

// 模拟 formatDuration 函数
func formatDuration(d time.Duration) string {
	days := int(d.Hours()) / 24
	hours := int(d.Hours()) % 24
	minutes := int(d.Minutes()) % 60
	if days > 0 {
		return fmt.Sprintf("%dd %dh %dm", days, hours, minutes)
	}
	if hours > 0 {
		return fmt.Sprintf("%dh %dm", hours, minutes)
	}
	return fmt.Sprintf("%dm", minutes)
}

// 模拟 formatBytes 函数
func formatBytes(bytes uint64) string {
	const unit = 1024
	if bytes < unit {
		return fmt.Sprintf("%d B", bytes)
	}
	div, exp := uint64(unit), 0
	for n := bytes / unit; n >= unit; n /= unit {
		div *= unit
		exp++
	}
	switch exp {
	case 0:
		return fmt.Sprintf("%.1f KB", float64(bytes)/float64(div))
	case 1:
		return fmt.Sprintf("%.1f MB", float64(bytes)/float64(div))
	case 2:
		return fmt.Sprintf("%.1f GB", float64(bytes)/float64(div))
	default:
		return fmt.Sprintf("%.1f TB", float64(bytes)/float64(div))
	}
}

func TestFormatDuration(t *testing.T) {
	tests := []struct {
		name     string
		duration time.Duration
		expected string
	}{
		{
			name:     "seconds",
			duration: 30 * time.Second,
			expected: "0m",
		},
		{
			name:     "minutes",
			duration: 5 * time.Minute,
			expected: "5m",
		},
		{
			name:     "hours and minutes",
			duration: 2*time.Hour + 30*time.Minute,
			expected: "2h 30m",
		},
		{
			name:     "days, hours and minutes",
			duration: 2*24*time.Hour + 3*time.Hour + 15*time.Minute,
			expected: "2d 3h 15m",
		},
		{
			name:     "exact hours",
			duration: 3 * time.Hour,
			expected: "3h 0m",
		},
		{
			name:     "exact days",
			duration: 5 * 24 * time.Hour,
			expected: "5d 0h 0m",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := formatDuration(tt.duration)
			if result != tt.expected {
				t.Errorf("Expected '%s', got '%s'", tt.expected, result)
			}
		})
	}
}

func TestFormatBytes(t *testing.T) {
	tests := []struct {
		name     string
		bytes    uint64
		expected string
	}{
		{
			name:     "bytes",
			bytes:    500,
			expected: "500 B",
		},
		{
			name:     "kilobytes",
			bytes:    1024,
			expected: "1.0 KB",
		},
		{
			name:     "megabytes",
			bytes:    1048576,
			expected: "1.0 MB",
		},
		{
			name:     "gigabytes",
			bytes:    1073741824,
			expected: "1.0 GB",
		},
		{
			name:     "terabytes",
			bytes:    1099511627776,
			expected: "1.0 TB",
		},
		{
			name:     "zero",
			bytes:    0,
			expected: "0 B",
		},
		{
			name:     "1.5 KB",
			bytes:    1536,
			expected: "1.5 KB",
		},
		{
			name:     "2.5 MB",
			bytes:    2621440,
			expected: "2.5 MB",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := formatBytes(tt.bytes)
			if result != tt.expected {
				t.Errorf("Expected '%s', got '%s'", tt.expected, result)
			}
		})
	}
}

func TestFormatSpeed(t *testing.T) {
	// 模拟 formatSpeed 函数
	formatSpeed := func(bytesPerSec uint64) string {
		return formatBytes(bytesPerSec) + "/s"
	}

	tests := []struct {
		name         string
		bytesPerSec  uint64
		expected     string
	}{
		{
			name:         "bytes per second",
			bytesPerSec:  500,
			expected:     "500 B/s",
		},
		{
			name:         "kilobytes per second",
			bytesPerSec:  1024,
			expected:     "1.0 KB/s",
		},
		{
			name:         "megabytes per second",
			bytesPerSec:  1048576,
			expected:     "1.0 MB/s",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := formatSpeed(tt.bytesPerSec)
			if result != tt.expected {
				t.Errorf("Expected '%s', got '%s'", tt.expected, result)
			}
		})
	}
}

func TestValidateURL(t *testing.T) {
	// 模拟 validateURL 函数
	validateURL := func(rawURL string) error {
		if rawURL == "" {
			return fmt.Errorf("URL cannot be empty")
		}

		parsed, err := url.Parse(rawURL)
		if err != nil {
			return fmt.Errorf("invalid URL format: %w", err)
		}

		if parsed.Scheme != "http" && parsed.Scheme != "https" {
			return fmt.Errorf("URL must use http or https scheme")
		}

		if parsed.Host == "" {
			return fmt.Errorf("URL must have a host")
		}

		return nil
	}

	tests := []struct {
		name    string
		url     string
		wantErr bool
	}{
		{
			name:    "valid http URL",
			url:     "http://example.com",
			wantErr: false,
		},
		{
			name:    "valid https URL",
			url:     "https://example.com/path",
			wantErr: false,
		},
		{
			name:    "empty URL",
			url:     "",
			wantErr: true,
		},
		{
			name:    "invalid scheme",
			url:     "ftp://example.com",
			wantErr: true,
		},
		{
			name:    "no host",
			url:     "http://",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validateURL(tt.url)
			if (err != nil) != tt.wantErr {
				t.Errorf("validateURL(%q) error = %v, wantErr %v", tt.url, err, tt.wantErr)
			}
		})
	}
}

func TestValidatePath(t *testing.T) {
	// 模拟 validatePath 函数
	validatePath := func(path string) error {
		if path == "" {
			return fmt.Errorf("path cannot be empty")
		}

		// 检查路径是否存在
		if _, err := os.Stat(path); os.IsNotExist(err) {
			return fmt.Errorf("path does not exist: %s", path)
		}

		return nil
	}

	// 创建临时文件
	tmpFile, err := os.CreateTemp("", "test")
	if err != nil {
		t.Fatalf("Failed to create temp file: %v", err)
	}
	defer os.Remove(tmpFile.Name())
	tmpFile.Close()

	tests := []struct {
		name    string
		path    string
		wantErr bool
	}{
		{
			name:    "valid path",
			path:    tmpFile.Name(),
			wantErr: false,
		},
		{
			name:    "empty path",
			path:    "",
			wantErr: true,
		},
		{
			name:    "nonexistent path",
			path:    "/nonexistent/path",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validatePath(tt.path)
			if (err != nil) != tt.wantErr {
				t.Errorf("validatePath(%q) error = %v, wantErr %v", tt.path, err, tt.wantErr)
			}
		})
	}
}

func TestValidatePort(t *testing.T) {
	// 模拟 validatePort 函数
	validatePort := func(port int) error {
		if port < 1 || port > 65535 {
			return fmt.Errorf("port must be between 1 and 65535")
		}
		return nil
	}

	tests := []struct {
		name    string
		port    int
		wantErr bool
	}{
		{
			name:    "valid port",
			port:    8080,
			wantErr: false,
		},
		{
			name:    "min port",
			port:    1,
			wantErr: false,
		},
		{
			name:    "max port",
			port:    65535,
			wantErr: false,
		},
		{
			name:    "zero port",
			port:    0,
			wantErr: true,
		},
		{
			name:    "negative port",
			port:    -1,
			wantErr: true,
		},
		{
			name:    "port too high",
			port:    65536,
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validatePort(tt.port)
			if (err != nil) != tt.wantErr {
				t.Errorf("validatePort(%d) error = %v, wantErr %v", tt.port, err, tt.wantErr)
			}
		})
	}
}
