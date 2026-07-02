package main

import (
	"fmt"
	"os"
	"os/exec"
	"strconv"
	"strings"
	"time"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
	"github.com/yequdesu/linux-cli-tui-clash/internal/sub"
)

var subUpdateCmd = &cobra.Command{
	Use:   "update [id]",
	Short: "Update subscription",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if err := updateSubscription(args); err != nil {
			handleSubUpdateError(err)
		}
	},
}

func handleSubUpdateError(err error) {
	if subUpdateCron {
		exitProcess(1)
		return
	}
	subscriptionWarn("%v", err)
	exitProcess(1)
}

func updateSubscription(args []string) error {
	return withSubscriptionQuiet(subUpdateCron, func() error {
		return ilog.WithQuiet(subUpdateCron, func() error {
			return updateSubscriptionCore(args)
		})
	})
}

func updateSubscriptionCore(args []string) error {
	var id int
	if len(args) > 0 {
		parsed, err := strconv.Atoi(strings.TrimSpace(args[0]))
		if err != nil || parsed <= 0 {
			return fmt.Errorf("invalid subscription id")
		}
		id = parsed
	}

	var profile config.Profile
	if err := config.WithProfilesLock(cfg, func() error {
		meta, err := config.LoadProfiles(cfg.ProfilesMeta())
		if err != nil {
			return err
		}
		if len(meta.Profiles) == 0 {
			return fmt.Errorf("no subscriptions")
		}
		if id == 0 {
			id = meta.Use
		}
		p := meta.FindByID(id)
		if p == nil {
			return fmt.Errorf("subscription id %d not found", id)
		}
		profile = *p
		return nil
	}); err != nil {
		return err
	}

	tempPath, err := newTempConfigPath(cfg)
	if err != nil {
		return fmt.Errorf("create temp config failed: %w", err)
	}
	defer os.Remove(tempPath)

	subscriptionInfo("updating: [%d] %s", id, profile.URL)
	if err := sub.Download(profile.URL, tempPath, cfg.ClashSubUA); err != nil {
		logSub(fmt.Sprintf("update failed: [%d]", id))
		return fmt.Errorf("download failed: %w", err)
	}
	if err := validateConfigFile(cfg, tempPath); err != nil {
		subscriptionInfo("trying conversion...")
		if err := sub.ConvertDownload(cfg, profile.URL, tempPath); err != nil {
			logSub(fmt.Sprintf("update failed: [%d]", id))
			return fmt.Errorf("update failed: %w", err)
		}
		if err := validateConfigFile(cfg, tempPath); err != nil {
			logSub(fmt.Sprintf("update failed: [%d]", id))
			return fmt.Errorf("converted config validation failed: %w", err)
		}
	}
	data, err := os.ReadFile(tempPath)
	if err != nil {
		logSub(fmt.Sprintf("update failed: [%d]", id))
		return fmt.Errorf("read updated config failed: %w", err)
	}

	commit, err := saveUpdatedProfileData(cfg, id, data)
	if err != nil {
		logSub(fmt.Sprintf("metadata update failed: [%d]", id))
		return err
	}

	if commit.currentUse == id {
		err := withSubscriptionQuiet(subUpdateCron, func() error {
			return switchSubscription(cfg, id)
		})
		if err != nil {
			commit.rollback()
			logSub(fmt.Sprintf("update activation failed: [%d]", id))
			return fmt.Errorf("update activation failed, restored previous profile: %w", err)
		}
	}
	if subUpdateCron {
		if err := logSubStrict(fmt.Sprintf("updated: [%d]", id)); err != nil {
			return err
		}
	} else {
		logSub(fmt.Sprintf("updated: [%d]", id))
	}
	subscriptionOk("subscription updated: [%d]", id)

	if subUpdateAuto {
		if err := runAutoUpdate(); err != nil {
			return err
		}
	}
	return nil
}

type profileUpdateCommit struct {
	currentUse int
	rollback   func()
}

func saveUpdatedProfileData(cfg *config.EnvConfig, id int, data []byte) (profileUpdateCommit, error) {
	commit := profileUpdateCommit{rollback: func() {}}
	if err := config.WithProfilesLock(cfg, func() error {
		meta, err := config.LoadProfiles(cfg.ProfilesMeta())
		if err != nil {
			return err
		}
		p := meta.FindByID(id)
		if p == nil {
			return fmt.Errorf("subscription id %d not found", id)
		}
		snapshots, err := snapshotFiles(p.Path, cfg.ProfilesMeta())
		if err != nil {
			return fmt.Errorf("snapshot profile: %w", err)
		}
		if err := config.AtomicWriteFile(p.Path, data, 0644); err != nil {
			return fmt.Errorf("write updated config failed: %w", err)
		}
		p.Updated = time.Now().Format("2006-01-02 15:04:05")
		commit.currentUse = meta.Use
		if err := saveProfiles(cfg.ProfilesMeta(), meta); err != nil {
			restoreSnapshots(snapshots)
			return fmt.Errorf("save profiles metadata failed, restored profile file: %w", err)
		}
		commit.rollback = func() {
			restoreSnapshots(snapshots)
		}
		return nil
	}); err != nil {
		return profileUpdateCommit{}, err
	}
	return commit, nil
}

var subUpdateAuto bool
var subUpdateCron bool

func init() {
	subUpdateCmd.Flags().BoolVar(&subUpdateAuto, "auto", false, "Enable auto-update via cron")
	subUpdateCmd.Flags().BoolVar(&subUpdateCron, "cron", false, "Run from cron without prompts")
}

func runAutoUpdate() error {
	if _, err := exec.LookPath("crontab"); err != nil {
		return fmt.Errorf("crontab not available, cannot set auto-update")
	}

	clashctlBin := "/usr/local/bin/clashctl"
	if _, err := os.Stat(clashctlBin); os.IsNotExist(err) {
		clashctlBin = "clashctl"
	}

	cronLine := fmt.Sprintf("0 */12 * * * %s sub update --cron", clashctlBin)

	existing, _ := exec.Command("crontab", "-l").Output()
	newCrontab, changed := mergeAutoUpdateCrontab(string(existing), cronLine)
	if !changed {
		subscriptionInfo("auto-update already enabled")
		return nil
	}
	cmd := exec.Command("crontab", "-")
	cmd.Stdin = strings.NewReader(newCrontab)
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("failed to install crontab: %w", err)
	}
	subscriptionOk("auto-update enabled (every 12 hours via cron)")
	return nil
}

func mergeAutoUpdateCrontab(existing, cronLine string) (string, bool) {
	var lines []string
	enabled := false
	changed := false

	trimmed := strings.TrimRight(existing, "\n")
	if trimmed != "" {
		for _, line := range strings.Split(trimmed, "\n") {
			stripped := strings.TrimSpace(line)
			if stripped == "" || strings.HasPrefix(stripped, "#") {
				lines = append(lines, line)
				continue
			}
			if strings.Contains(line, "clashctl sub update") {
				if strings.Contains(line, "--cron") {
					enabled = true
					lines = append(lines, line)
				} else {
					changed = true
				}
				continue
			}
			lines = append(lines, line)
		}
	}

	if !enabled {
		lines = append(lines, cronLine)
		changed = true
	}
	if len(lines) == 0 {
		return "", changed
	}
	return strings.Join(lines, "\n") + "\n", changed
}

func validateConfigFile(cfg *config.EnvConfig, path string) error {
	if _, err := os.Stat(cfg.KernelBin()); os.IsNotExist(err) {
		return nil
	}
	cmd := exec.Command(cfg.KernelBin(), "-d", cfg.ResourcesDir(), "-f", path, "-t")
	out, err := cmd.CombinedOutput()
	if err != nil {
		return fmt.Errorf("%s: %s", err.Error(), string(out))
	}
	return nil
}
