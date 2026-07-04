package main

import (
	"os"
	"os/exec"
	"os/user"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
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
	_ = os.MkdirAll(cfg.LogDir(), 0755)
	return nil
}

func newTempConfigPath(cfg *config.EnvConfig) (string, error) {
	if err := os.MkdirAll(cfg.ResourcesDir(), 0755); err != nil {
		return "", err
	}
	f, err := os.CreateTemp(cfg.ResourcesDir(), "temp-*.yaml")
	if err != nil {
		return "", err
	}
	path := f.Name()
	if err := f.Close(); err != nil {
		_ = os.Remove(path)
		return "", err
	}
	return path, nil
}

func rerunWithSudoOrFatal(action string) {
	if runningAsRoot() {
		return
	}
	if !stdinIsTerminal() {
		ilog.Fatal("%s requires sudo. Run: sudo clashctl %s", action, action)
	}
	sudo, err := exec.LookPath("sudo")
	if err != nil {
		ilog.Fatal("%s requires sudo, but sudo was not found", action)
	}
	exe, err := os.Executable()
	if err != nil {
		exe = "clashctl"
	}
	args := append([]string{"-E", exe}, os.Args[1:]...)
	cmd := exec.Command(sudo, args...)
	cmd.Stdin = os.Stdin
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	if err := cmd.Run(); err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			os.Exit(exitErr.ExitCode())
		}
		ilog.Fatal("sudo %s failed: %v", action, err)
	}
	os.Exit(0)
}

func runningAsRoot() bool {
	current, err := user.Current()
	return err == nil && current.Uid == "0"
}

func stdinIsTerminal() bool {
	info, err := os.Stdin.Stat()
	return err == nil && (info.Mode()&os.ModeCharDevice) != 0
}
