package config

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"
	"regexp"
	"strings"

	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var proxyNotFoundRe = regexp.MustCompile(`proxy\s*\[([^\]]+)\]\s*not\s*found`)

func MergeConfig(cfg *EnvConfig, autoFix bool) error {
	ilog.Info("merging config...")

	configPath := cfg.ConfigPath()
	if _, err := os.Stat(configPath); os.IsNotExist(err) {
		os.WriteFile(configPath, []byte("{}\n"), 0644)
	}

	backupData, _ := os.ReadFile(cfg.RuntimePath())

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

	if err := os.WriteFile(cfg.RuntimePath(), out, 0644); err != nil {
		return fmt.Errorf("write runtime: %w", err)
	}

	if err := validateConfig(cfg, cfg.RuntimePath()); err != nil {
		errMsg := err.Error()
		missingGroup := extractMissingProxyGroup(errMsg)

		if missingGroup != "" && autoFix {
			suggestion := findProxyGroupSuggestion(cfg)
			if suggestion != "" {
				if fixErr := applyProxyGroupFix(cfg, missingGroup, suggestion); fixErr != nil {
					ilog.Info("auto-fix failed: %v", fixErr)
				} else if validateConfig(cfg, cfg.RuntimePath()) == nil {
					ilog.Info("auto-fixed MATCH rule target: %q → %q", missingGroup, suggestion)
					return nil
				}
			}
		}

		if missingGroup != "" {
			suggestion := findProxyGroupSuggestion(cfg)
			if backupData != nil {
				os.WriteFile(cfg.RuntimePath(), backupData, 0644)
			}
			if suggestion != "" {
				return fmt.Errorf("config validation failed: rule references unknown proxy group %q — did you mean %q?\n  use 'clashctl config merge --autofix' to auto-correct", missingGroup, suggestion)
			}
			return fmt.Errorf("config validation failed: rule references unknown proxy group %q (no known proxy group found in subscription)", missingGroup)
		}

		if backupData != nil {
			os.WriteFile(cfg.RuntimePath(), backupData, 0644)
		}
		return fmt.Errorf("config validation failed: %w", err)
	}
	return nil
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

func applyProxyGroupFix(cfg *EnvConfig, missingGroup, replacement string) error {
	data, err := os.ReadFile(cfg.RuntimePath())
	if err != nil {
		return err
	}

	pattern := "MATCH," + missingGroup
	replacementStr := "MATCH," + replacement

	if !strings.Contains(string(data), pattern) {
		return fmt.Errorf("pattern %q not found in runtime config", pattern)
	}

	newData := strings.ReplaceAll(string(data), pattern, replacementStr)
	return os.WriteFile(cfg.RuntimePath(), []byte(newData), 0644)
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
