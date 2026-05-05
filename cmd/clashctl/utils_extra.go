package main

import (
	"os"
	"os/exec"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func readFileString(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	return string(data), nil
}

func execCommand(name string, args ...string) *exec.Cmd {
	return exec.Command(name, args...)
}

func initConfig(cfg *config.EnvConfig) error {
	_ = os.MkdirAll(cfg.ResourcesDir(), 0755)
	_ = os.MkdirAll(cfg.ProfilesDir(), 0755)
	_ = os.MkdirAll(cfg.ConfigsDir(), 0755)
	return nil
}
