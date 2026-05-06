package config

import (
	"fmt"
	"os"
	"os/exec"

	"gopkg.in/yaml.v3"
)

// MergeConfig merges config.yaml and mixin.yaml into runtime.yaml
func MergeConfig(configPath, mixinPath, outputPath string) error {
	// Try yq first (best deep merge)
	yqPath, err := exec.LookPath("yq")
	if err != nil {
		home, _ := os.UserHomeDir()
		localYq := home + "/.clashctl/bin/yq"
		if _, statErr := os.Stat(localYq); statErr == nil {
			yqPath = localYq
		}
	}

	if yqPath != "" {
		cmd := exec.Command(yqPath,
			"eval-all", ". as $item ireduce({}; . * $item)",
			configPath, mixinPath,
		)
		output, err := cmd.Output()
		if err == nil {
			return os.WriteFile(outputPath, output, 0644)
		}
		// yq failed, fall through to Go merge
	}

	return mergeYAML(configPath, mixinPath, outputPath)
}

// mergeYAML does a proper deep merge using gopkg.in/yaml.v3
func mergeYAML(configPath, mixinPath, outputPath string) error {
	var configMap map[string]interface{}
	var mixinMap map[string]interface{}

	configData, err := os.ReadFile(configPath)
	if err != nil {
		return fmt.Errorf("cannot read config.yaml: %v", err)
	}
	if err := yaml.Unmarshal(configData, &configMap); err != nil {
		return fmt.Errorf("cannot parse config.yaml: %v", err)
	}

	mixinData, err := os.ReadFile(mixinPath)
	if err != nil {
		// No mixin, just use config as-is
		return os.WriteFile(outputPath, configData, 0644)
	}
	if err := yaml.Unmarshal(mixinData, &mixinMap); err != nil {
		// Mixin invalid, use config as-is
		return os.WriteFile(outputPath, configData, 0644)
	}

	deepMerge(configMap, mixinMap)

	output, err := yaml.Marshal(configMap)
	if err != nil {
		return fmt.Errorf("cannot marshal merged config: %v", err)
	}

	return os.WriteFile(outputPath, output, 0644)
}

// deepMerge merges src into dst recursively. src values override dst.
func deepMerge(dst, src map[string]interface{}) {
	for key, srcVal := range src {
		if dstVal, ok := dst[key]; ok {
			srcMap, srcIsMap := srcVal.(map[string]interface{})
			dstMap, dstIsMap := dstVal.(map[string]interface{})
			if srcIsMap && dstIsMap {
				deepMerge(dstMap, srcMap)
				continue
			}
		}
		dst[key] = srcVal
	}
}

// SetTUNMode enables or disables TUN in the config file
func SetTUNMode(configPath string, enable bool) error {
	data, err := os.ReadFile(configPath)
	if err != nil {
		return fmt.Errorf("cannot read config: %v", err)
	}

	var cfg map[string]interface{}
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return fmt.Errorf("cannot parse config: %v", err)
	}

	if tun, ok := cfg["tun"].(map[string]interface{}); ok {
		tun["enable"] = enable
	} else {
		cfg["tun"] = map[string]interface{}{
			"enable":                enable,
			"stack":                 "gvisor",
			"auto-route":            true,
			"auto-detect-interface": true,
		}
	}

	output, err := yaml.Marshal(cfg)
	if err != nil {
		return fmt.Errorf("cannot marshal config: %v", err)
	}

	return os.WriteFile(configPath, output, 0644)
}

// GetTUNMode reads TUN mode status from config
func GetTUNMode(configPath string) (bool, error) {
	data, err := os.ReadFile(configPath)
	if err != nil {
		return false, err
	}

	var cfg map[string]interface{}
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return false, err
	}

	if tun, ok := cfg["tun"].(map[string]interface{}); ok {
		if enable, ok := tun["enable"].(bool); ok {
			return enable, nil
		}
	}
	return false, nil
}
