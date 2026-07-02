package config

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"

	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
	"gopkg.in/yaml.v3"
)

var proxyNotFoundRe = regexp.MustCompile(`proxy\s*\[([^\]]+)\]\s*not\s*found`)

func MergeConfig(cfg *EnvConfig, autoFix bool) error {
	ilog.Info("merging config...")

	configPath := cfg.ConfigPath()
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		if err := AtomicWriteFile(configPath, []byte("{}\n"), 0644); err != nil {
			return fmt.Errorf("initialize config: %w", err)
		}
	}

	yqExpr := fmt.Sprintf(`
select(fileIndex==0) as $config |
select(fileIndex==1) as $mixin |

$mixin |= del(._custom) |
(($config // {}) * $mixin) as $runtime |
$runtime |

.rules = (
  ($mixin.rules.prefix // []) +
  ($config.rules // []) +
  ($mixin.rules.suffix // [])
) |

.proxies = (
  ($mixin.proxies.prefix // []) +
  (
    ($config.proxies // []) as $configList |
    ($mixin.proxies.override // []) as $overrideList |
    $configList | map(
      . as $configItem |
      (
        $overrideList[] | select(.name == $configItem.name)
      ) // $configItem
    )
  ) +
  ($mixin.proxies.suffix // [])
) |

.proxy-groups = (
  ($mixin.proxy-groups.prefix // []) +
  (
    ($config.proxy-groups // []) as $configList |
    ($mixin.proxy-groups.override // []) as $overrideList |
    $configList | map(
      . as $configItem |
      (
        $overrideList[] | select(.name == $configItem.name)
      ) // $configItem
    )
  ) +
  ($mixin.proxy-groups.suffix // [])
) |

($mixin.proxy-groups.inject // {}) as $inj |
.proxy-groups[] |= (
  . as $g |
  ($inj | .[$g.name] // []) as $extra |
  .proxies = (.proxies + $extra | unique)
)
`)

	cmd := exec.Command(cfg.YQBin(), "eval-all", yqExpr, configPath, cfg.MixinPath())
	var stderr bytes.Buffer
	cmd.Stderr = &stderr

	out, err := cmd.Output()
	if err != nil {
		return fmt.Errorf("yq merge: %s", stderr.String())
	}

	tmpRuntime, err := writeRuntimeCandidate(cfg, out)
	if err != nil {
		return err
	}
	defer os.Remove(tmpRuntime)

	if err := validateConfig(cfg, tmpRuntime); err != nil {
		errMsg := err.Error()
		missingGroup := extractMissingProxyGroup(errMsg)

		if missingGroup != "" && autoFix {
			suggestion := findProxyGroupSuggestion(cfg)
			if suggestion != "" {
				if fixErr := applyProxyGroupFix(tmpRuntime, missingGroup, suggestion); fixErr != nil {
					ilog.Info("auto-fix failed: %v", fixErr)
				} else if validateConfig(cfg, tmpRuntime) == nil {
					fixedData, readErr := os.ReadFile(tmpRuntime)
					if readErr != nil {
						return fmt.Errorf("read fixed runtime: %w", readErr)
					}
					if writeErr := AtomicWriteFile(cfg.RuntimePath(), fixedData, 0644); writeErr != nil {
						return fmt.Errorf("write runtime: %w", writeErr)
					}
					ilog.Info("auto-fixed MATCH rule target: %q → %q", missingGroup, suggestion)
					return nil
				}
			}
		}

		if missingGroup != "" {
			suggestion := findProxyGroupSuggestion(cfg)
			if suggestion != "" {
				return fmt.Errorf("config validation failed: rule references unknown proxy group %q — did you mean %q?\n  use 'clashctl config merge --autofix' to auto-correct", missingGroup, suggestion)
			}
			return fmt.Errorf("config validation failed: rule references unknown proxy group %q (no known proxy group found in subscription)", missingGroup)
		}

		return fmt.Errorf("config validation failed: %w", err)
	}
	if err := AtomicWriteFile(cfg.RuntimePath(), out, 0644); err != nil {
		return fmt.Errorf("write runtime: %w", err)
	}
	return nil
}

func writeRuntimeCandidate(cfg *EnvConfig, data []byte) (string, error) {
	dir := filepath.Dir(cfg.RuntimePath())
	if err := os.MkdirAll(dir, 0755); err != nil {
		return "", fmt.Errorf("create runtime dir: %w", err)
	}
	tmp, err := os.CreateTemp(dir, ".runtime-*.yaml")
	if err != nil {
		return "", fmt.Errorf("create runtime temp: %w", err)
	}
	tmpPath := tmp.Name()
	if _, err := tmp.Write(data); err != nil {
		tmp.Close()
		os.Remove(tmpPath)
		return "", fmt.Errorf("write runtime temp: %w", err)
	}
	if err := tmp.Chmod(0644); err != nil {
		tmp.Close()
		os.Remove(tmpPath)
		return "", fmt.Errorf("chmod runtime temp: %w", err)
	}
	if err := tmp.Sync(); err != nil {
		tmp.Close()
		os.Remove(tmpPath)
		return "", fmt.Errorf("sync runtime temp: %w", err)
	}
	if err := tmp.Close(); err != nil {
		os.Remove(tmpPath)
		return "", fmt.Errorf("close runtime temp: %w", err)
	}
	return tmpPath, nil
}

func extractMissingProxyGroup(errMsg string) string {
	matches := proxyNotFoundRe.FindStringSubmatch(errMsg)
	if len(matches) >= 2 {
		return matches[1]
	}
	return ""
}

func findProxyGroupSuggestion(cfg *EnvConfig) string {
	if name := firstYQ(cfg, ".proxy-groups[] | select(.type == \"select\") | .name"); name != "" {
		return name
	}
	return firstYQ(cfg, ".proxy-groups[0].name")
}

func firstYQ(cfg *EnvConfig, expr string) string {
	cmd := exec.Command(cfg.YQBin(), "eval", expr, cfg.ConfigPath())
	out, err := cmd.Output()
	if err != nil {
		return ""
	}
	for _, line := range strings.Split(strings.TrimSpace(string(out)), "\n") {
		line = strings.TrimSpace(strings.Trim(line, "\""))
		if line != "" {
			return line
		}
	}
	return ""
}

func applyProxyGroupFix(path, missingGroup, replacement string) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return err
	}

	var doc yaml.Node
	if err := yaml.Unmarshal(data, &doc); err != nil {
		return fmt.Errorf("parse runtime yaml: %w", err)
	}

	rules := mappingValue(&doc, "rules")
	if rules == nil || rules.Kind != yaml.SequenceNode {
		return fmt.Errorf("rules list not found in runtime config")
	}

	changed := false
	for _, rule := range rules.Content {
		if rule.Kind != yaml.ScalarNode {
			continue
		}
		fixed, ok := fixMatchRule(rule.Value, missingGroup, replacement)
		if ok {
			rule.Value = fixed
			changed = true
		}
	}
	if !changed {
		return fmt.Errorf("MATCH rule target %q not found in runtime config", missingGroup)
	}

	out, err := yaml.Marshal(&doc)
	if err != nil {
		return fmt.Errorf("marshal runtime yaml: %w", err)
	}
	return AtomicWriteFile(path, out, 0644)
}

func mappingValue(doc *yaml.Node, key string) *yaml.Node {
	root := doc
	if root.Kind == yaml.DocumentNode && len(root.Content) > 0 {
		root = root.Content[0]
	}
	if root.Kind != yaml.MappingNode {
		return nil
	}
	for i := 0; i+1 < len(root.Content); i += 2 {
		if root.Content[i].Value == key {
			return root.Content[i+1]
		}
	}
	return nil
}

func fixMatchRule(rule, missingGroup, replacement string) (string, bool) {
	parts := strings.Split(rule, ",")
	if len(parts) < 2 || strings.TrimSpace(parts[0]) != "MATCH" || strings.TrimSpace(parts[1]) != missingGroup {
		return rule, false
	}
	parts[1] = replacement
	return strings.Join(parts, ","), true
}

func validateConfig(cfg *EnvConfig, configPath string) error {
	if _, err := os.Stat(cfg.KernelBin()); os.IsNotExist(err) {
		return nil
	}
	cmd := exec.Command(cfg.KernelBin(), "-d", cfg.ResourcesDir(), "-f", configPath, "-t")
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("%s: %s", err.Error(), string(out))
	}
	return nil
}
