package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
)

var (
	clashBaseDir      = expandPath("~/.clashctl")
	clashBinDir       = filepath.Join(clashBaseDir, "bin")
	clashResourcesDir = filepath.Join(clashBaseDir, "resources")
	clashLogsDir      = filepath.Join(clashBaseDir, "logs")
	clashRuntimeDir   = filepath.Join(clashBaseDir, "runtime")
)

func init() {
	if dir := os.Getenv("CLASH_BASE_DIR"); dir != "" {
		clashBaseDir = dir
		clashBinDir = filepath.Join(clashBaseDir, "bin")
		clashResourcesDir = filepath.Join(clashBaseDir, "resources")
		clashLogsDir = filepath.Join(clashBaseDir, "logs")
		clashRuntimeDir = filepath.Join(clashBaseDir, "runtime")
	}
}

func expandPath(path string) string {
	if strings.HasPrefix(path, "~/") {
		home, err := os.UserHomeDir()
		if err != nil {
			return path
		}
		return filepath.Join(home, path[2:])
	}
	return path
}

func ensureDir(dir string) error {
	return os.MkdirAll(dir, 0755)
}

func runCmd(name string, args ...string) (string, error) {
	cmd := exec.Command(name, args...)
	out, err := cmd.CombinedOutput()
	return strings.TrimSpace(string(out)), err
}

func runCmdSudo(name string, args ...string) (string, error) {
	sudoArgs := append([]string{name}, args...)
	cmd := exec.Command("sudo", sudoArgs...)
	out, err := cmd.CombinedOutput()
	return strings.TrimSpace(string(out)), err
}

func fileExists(path string) bool {
	_, err := os.Stat(path)
	return err == nil
}

func pidFilePath() string {
	return filepath.Join(clashRuntimeDir, "mihomo.pid")
}

func secretFilePath() string {
	return filepath.Join(clashRuntimeDir, "api_secret")
}

func logFilePath() string {
	return filepath.Join(clashLogsDir, "mihomo.log")
}

func getEnvFilePath() string {
	return filepath.Join(clashBaseDir, ".env")
}

func readPID() string {
	data, err := os.ReadFile(pidFilePath())
	if err != nil {
		return ""
	}
	return strings.TrimSpace(string(data))
}

func isRunning() bool {
	pid := readPID()
	if pid == "" {
		return false
	}
	_, err := os.Stat(fmt.Sprintf("/proc/%s", pid))
	return err == nil
}

func writeFile(path, content string) error {
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return err
	}
	return os.WriteFile(path, []byte(content), 0644)
}

func readFile(path string) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}
	return strings.TrimSpace(string(data)), nil
}

// formatDuration formats a duration for display
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

// formatBytes formats bytes to human readable
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

// formatSpeed formats bytes per second to human readable
func formatSpeed(bytesPerSec uint64) string {
	return formatBytes(bytesPerSec) + "/s"
}

// green, red, yellow, cyan, blue, gray helper colors for terminal
const (
	colorReset  = "\033[0m"
	colorGreen  = "\033[32m"
	colorRed    = "\033[31m"
	colorYellow = "\033[33m"
	colorCyan   = "\033[36m"
	colorBlue   = "\033[34m"
	colorGray   = "\033[90m"
)

func green(s string) string  { return colorGreen + s + colorReset }
func red(s string) string    { return colorRed + s + colorReset }
func yellow(s string) string { return colorYellow + s + colorReset }
func cyan(s string) string   { return colorCyan + s + colorReset }
func blue(s string) string   { return colorBlue + s + colorReset }
func gray(s string) string   { return colorGray + s + colorReset }
func bold(s string) string   { return "\033[1m" + s + "\033[0m" }
