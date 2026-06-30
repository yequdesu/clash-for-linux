package config_test

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/yequdesu/clashctl/internal/config"
)

func TestDefaultEnvConfig(t *testing.T) {
	cfg := config.DefaultEnvConfig()

	// 验证默认值
	if cfg.BaseDir == "" {
		t.Error("Expected BaseDir to be set")
	}

	if cfg.BinDir == "" {
		t.Error("Expected BinDir to be set")
	}

	if cfg.ResourcesDir == "" {
		t.Error("Expected ResourcesDir to be set")
	}

	if cfg.LogsDir == "" {
		t.Error("Expected LogsDir to be set")
	}

	if cfg.RuntimeDir == "" {
		t.Error("Expected RuntimeDir to be set")
	}

	if cfg.MixedPort != 7897 {
		t.Errorf("Expected MixedPort 7897, got %d", cfg.MixedPort)
	}

	if cfg.SocksPort != 7897 {
		t.Errorf("Expected SocksPort 7897, got %d", cfg.SocksPort)
	}

	if cfg.Controller != "127.0.0.1:9090" {
		t.Errorf("Expected Controller '127.0.0.1:9090', got '%s'", cfg.Controller)
	}
}

func TestGetPort(t *testing.T) {
	tests := []struct {
		name       string
		controller string
		expected   int
	}{
		{
			name:       "valid port",
			controller: "127.0.0.1:9090",
			expected:   9090,
		},
		{
			name:       "custom port",
			controller: "0.0.0.0:8080",
			expected:   8080,
		},
		{
			name:       "no port",
			controller: "127.0.0.1",
			expected:   9090,
		},
		{
			name:       "invalid port",
			controller: "127.0.0.1:abc",
			expected:   9090,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			cfg := &config.EnvConfig{
				Controller: tt.controller,
			}
			port := cfg.GetPort()
			if port != tt.expected {
				t.Errorf("Expected port %d, got %d", tt.expected, port)
			}
		})
	}
}

func TestLoadEnv(t *testing.T) {
	// 创建临时目录
	tmpDir := t.TempDir()

	// 创建测试 .env 文件
	envContent := `CLASH_BASE_DIR=/tmp/clashctl
CLASH_BIN_DIR=/tmp/clashctl/bin
CLASH_RESOURCES_DIR=/tmp/clashctl/resources
CLASH_LOGS_DIR=/tmp/clashctl/logs
CLASH_RUNTIME_DIR=/tmp/clashctl/runtime
CLASH_CONTROLLER=127.0.0.1:8080
`
	envFile := filepath.Join(tmpDir, ".env")
	if err := os.WriteFile(envFile, []byte(envContent), 0644); err != nil {
		t.Fatalf("Failed to create test .env file: %v", err)
	}

	// 执行测试
	cfg, err := config.LoadEnv(envFile)

	// 验证结果
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if cfg.BaseDir != "/tmp/clashctl" {
		t.Errorf("Expected BaseDir '/tmp/clashctl', got '%s'", cfg.BaseDir)
	}

	if cfg.BinDir != "/tmp/clashctl/bin" {
		t.Errorf("Expected BinDir '/tmp/clashctl/bin', got '%s'", cfg.BinDir)
	}

	if cfg.ResourcesDir != "/tmp/clashctl/resources" {
		t.Errorf("Expected ResourcesDir '/tmp/clashctl/resources', got '%s'", cfg.ResourcesDir)
	}

	if cfg.LogsDir != "/tmp/clashctl/logs" {
		t.Errorf("Expected LogsDir '/tmp/clashctl/logs', got '%s'", cfg.LogsDir)
	}

	if cfg.RuntimeDir != "/tmp/clashctl/runtime" {
		t.Errorf("Expected RuntimeDir '/tmp/clashctl/runtime', got '%s'", cfg.RuntimeDir)
	}

	if cfg.Controller != "127.0.0.1:8080" {
		t.Errorf("Expected Controller '127.0.0.1:8080', got '%s'", cfg.Controller)
	}
}

func TestLoadEnvMissing(t *testing.T) {
	// 测试文件不存在的情况
	cfg, err := config.LoadEnv("/nonexistent/.env")

	// 应该返回默认配置，没有错误
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if cfg == nil {
		t.Fatal("Expected default config, got nil")
	}

	// 验证返回默认值
	defaultCfg := config.DefaultEnvConfig()
	if cfg.BaseDir != defaultCfg.BaseDir {
		t.Errorf("Expected default BaseDir, got '%s'", cfg.BaseDir)
	}
}

func TestLoadEnvInvalidFormat(t *testing.T) {
	// 创建临时目录
	tmpDir := t.TempDir()

	// 创建无效格式的 .env 文件
	envContent := `INVALID_FORMAT
ANOTHER_INVALID
`
	envFile := filepath.Join(tmpDir, ".env")
	if err := os.WriteFile(envFile, []byte(envContent), 0644); err != nil {
		t.Fatalf("Failed to create test .env file: %v", err)
	}

	// 执行测试
	cfg, err := config.LoadEnv(envFile)

	// 应该返回默认配置，没有错误（忽略无效行）
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if cfg == nil {
		t.Fatal("Expected default config, got nil")
	}
}

func TestLoadEnvWithComments(t *testing.T) {
	// 创建临时目录
	tmpDir := t.TempDir()

	// 创建带注释的 .env 文件
	envContent := `# This is a comment
CLASH_BASE_DIR=/tmp/clashctl
# Another comment
CLASH_CONTROLLER=127.0.0.1:8080
`
	envFile := filepath.Join(tmpDir, ".env")
	if err := os.WriteFile(envFile, []byte(envContent), 0644); err != nil {
		t.Fatalf("Failed to create test .env file: %v", err)
	}

	// 执行测试
	cfg, err := config.LoadEnv(envFile)

	// 验证结果
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if cfg.BaseDir != "/tmp/clashctl" {
		t.Errorf("Expected BaseDir '/tmp/clashctl', got '%s'", cfg.BaseDir)
	}

	if cfg.Controller != "127.0.0.1:8080" {
		t.Errorf("Expected Controller '127.0.0.1:8080', got '%s'", cfg.Controller)
	}
}

func TestLoadEnvWithEmptyLines(t *testing.T) {
	// 创建临时目录
	tmpDir := t.TempDir()

	// 创建带空行的 .env 文件
	envContent := `
CLASH_BASE_DIR=/tmp/clashctl

CLASH_CONTROLLER=127.0.0.1:8080

`
	envFile := filepath.Join(tmpDir, ".env")
	if err := os.WriteFile(envFile, []byte(envContent), 0644); err != nil {
		t.Fatalf("Failed to create test .env file: %v", err)
	}

	// 执行测试
	cfg, err := config.LoadEnv(envFile)

	// 验证结果
	if err != nil {
		t.Fatalf("Unexpected error: %v", err)
	}

	if cfg.BaseDir != "/tmp/clashctl" {
		t.Errorf("Expected BaseDir '/tmp/clashctl', got '%s'", cfg.BaseDir)
	}

	if cfg.Controller != "127.0.0.1:8080" {
		t.Errorf("Expected Controller '127.0.0.1:8080', got '%s'", cfg.Controller)
	}
}
