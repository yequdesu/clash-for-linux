package config

import (
	"bytes"
	"fmt"
	"os"
	"os/exec"

	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

func MergeConfig(cfg *EnvConfig) error {
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
		if backupData != nil {
			os.WriteFile(cfg.RuntimePath(), backupData, 0644)
		}
		return fmt.Errorf("config validation failed: %w", err)
	}
	return nil
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
