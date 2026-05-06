package kernel

import (
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"time"
)

// ValidateConfig validates a Mihomo configuration file
func ValidateConfig(resourcesDir, configPath, binDir string) error {
	mihomoPath := filepath.Join(binDir, "mihomo")
	if _, err := os.Stat(mihomoPath); os.IsNotExist(err) {
		return fmt.Errorf("mihomo binary not found at %s", mihomoPath)
	}

	cmd := exec.Command(mihomoPath, "-d", resourcesDir, "-f", configPath, "-t")
	out, err := cmd.CombinedOutput()
	output := string(out)
	if err != nil {
		return fmt.Errorf("config validation failed: %s\n%s", err.Error(), output)
	}
	return nil
}

// StartKernel starts the Mihomo kernel process and returns the PID
func StartKernel(binDir, resourcesDir, configPath, logsDir, secretPath string) (int, error) {
	mihomoPath := filepath.Join(binDir, "mihomo")

	if _, err := os.Stat(mihomoPath); os.IsNotExist(err) {
		// Try system mihomo
		mihomoPath = "mihomo"
	}

	os.MkdirAll(logsDir, 0755)

	logFile, err := os.Create(filepath.Join(logsDir, "mihomo.log"))
	if err != nil {
		return 0, fmt.Errorf("failed to create log file: %v", err)
	}

	cmd := exec.Command(mihomoPath,
		"-d", resourcesDir,
		"-f", configPath,
	)
	cmd.Stdout = logFile
	cmd.Stderr = logFile
	cmd.Dir = resourcesDir

	// Read secret and set as env if exists
	if secret, err := os.ReadFile(secretPath); err == nil {
		cmd.Env = append(os.Environ(), "MIHOMO_API_SECRET="+string(secret))
	}

	if err := cmd.Start(); err != nil {
		return 0, fmt.Errorf("failed to start mihomo: %v", err)
	}

	return cmd.Process.Pid, nil
}

// WaitForReady polls the Mihomo API until it responds
func WaitForReady(port, timeoutMs int) error {
	url := fmt.Sprintf("http://127.0.0.1:%d", port)
	deadline := time.Now().Add(time.Duration(timeoutMs) * time.Millisecond)

	client := &http.Client{Timeout: 500 * time.Millisecond}

	for time.Now().Before(deadline) {
		resp, err := client.Get(url)
		if err == nil {
			resp.Body.Close()
			return nil
		}
		time.Sleep(500 * time.Millisecond)
	}
	return fmt.Errorf("mihomo did not start within %dms", timeoutMs)
}
