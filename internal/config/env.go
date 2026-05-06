package config

import (
	"os"
	"path/filepath"
	"strconv"
	"strings"
)

// EnvConfig holds the .env configuration
type EnvConfig struct {
	BaseDir      string
	BinDir       string
	ResourcesDir string
	LogsDir      string
	RuntimeDir   string
	MixedPort    int
	SocksPort    int
	Controller   string
}

// DefaultEnvConfig returns default environment configuration
func DefaultEnvConfig() *EnvConfig {
	home, _ := os.UserHomeDir()
	base := filepath.Join(home, ".clashctl")
	return &EnvConfig{
		BaseDir:      base,
		BinDir:       filepath.Join(base, "bin"),
		ResourcesDir: filepath.Join(base, "resources"),
		LogsDir:      filepath.Join(base, "logs"),
		RuntimeDir:   filepath.Join(base, "runtime"),
		MixedPort:    7897,
		SocksPort:    7897,
		Controller:   "127.0.0.1:9090",
	}
}

// GetPort returns the port from the controller address
func (e *EnvConfig) GetPort() int {
	parts := strings.Split(e.Controller, ":")
	if len(parts) >= 2 {
		if p, err := strconv.Atoi(parts[1]); err == nil {
			return p
		}
	}
	return 9090
}

// LoadEnv loads the .env configuration file
func LoadEnv(path string) (*EnvConfig, error) {
	cfg := DefaultEnvConfig()

	data, err := os.ReadFile(path)
	if err != nil {
		return cfg, nil
	}

	lines := strings.Split(string(data), "\n")
	for _, line := range lines {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		parts := strings.SplitN(line, "=", 2)
		if len(parts) != 2 {
			continue
		}
		key := strings.TrimSpace(parts[0])
		val := strings.TrimSpace(parts[1])

		switch key {
		case "CLASH_BASE_DIR":
			cfg.BaseDir = val
		case "CLASH_BIN_DIR":
			cfg.BinDir = val
		case "CLASH_RESOURCES_DIR":
			cfg.ResourcesDir = val
		case "CLASH_LOGS_DIR":
			cfg.LogsDir = val
		case "CLASH_RUNTIME_DIR":
			cfg.RuntimeDir = val
		case "CLASH_CONTROLLER":
			cfg.Controller = val
		}
	}
	return cfg, nil
}
