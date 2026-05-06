package config

import (
	"fmt"
	"os"
	"os/exec"
	"strings"
)

// MergeConfig merges config.yaml and mixin.yaml into runtime.yaml using yq
func MergeConfig(configPath, mixinPath, outputPath string) error {
	// Use yq if available
	yqPath, err := exec.LookPath("yq")
	if err != nil {
		// Try local yq
		home, _ := os.UserHomeDir()
		localYq := home + "/.clashctl/bin/yq"
		if _, err := os.Stat(localYq); err == nil {
			yqPath = localYq
		} else {
			// Fall back to simple concat + manual merge
			return mergeConfigSimple(configPath, mixinPath, outputPath)
		}
	}

	// yq eval-all '. as $item ireduce({}; . * $item)' config.yaml mixin.yaml
	cmd := exec.Command(yqPath,
		"eval-all", ". as $item ireduce({}; . * $item)",
		configPath, mixinPath,
	)

	output, err := cmd.Output()
	if err != nil {
		return mergeConfigSimple(configPath, mixinPath, outputPath)
	}

	return os.WriteFile(outputPath, output, 0644)
}

// mergeConfigSimple performs basic YAML merge without yq
func mergeConfigSimple(configPath, mixinPath, outputPath string) error {
	configData, err := os.ReadFile(configPath)
	if err != nil {
		return fmt.Errorf("cannot read config.yaml: %v", err)
	}

	mixinData, err := os.ReadFile(mixinPath)
	if err != nil {
		// If mixin.yaml doesn't exist, just use config.yaml
		return os.WriteFile(outputPath, configData, 0644)
	}

	// Simple approach: copy config first, then append mixin content
	combined := append(configData, '\n')
	combined = append(combined, mixinData...)

	return os.WriteFile(outputPath, combined, 0644)
}

// SetTUNMode enables or disables TUN in mixin.yaml
func SetTUNMode(mixinPath string, enable bool) error {
	data, err := os.ReadFile(mixinPath)
	if err != nil {
		return fmt.Errorf("cannot read mixin.yaml: %v", err)
	}

	content := string(data)

	var newContent string
	if enable {
		newContent = strings.Replace(content, "enable: false", "enable: true", 1)
	} else {
		newContent = strings.Replace(content, "enable: true", "enable: false", 1)
	}

	return os.WriteFile(mixinPath, []byte(newContent), 0644)
}

// GetTUNMode reads TUN mode status from mixin.yaml
func GetTUNMode(mixinPath string) (bool, error) {
	data, err := os.ReadFile(mixinPath)
	if err != nil {
		return false, err
	}
	content := string(data)
	return strings.Contains(content, "enable: true"), nil
}
