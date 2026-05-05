package main

import (
	"fmt"
	"os"
	"os/exec"
	"strings"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	"github.com/yequdesu/linux-cli-tui-clash/internal/sub"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var subUpdateCmd = &cobra.Command{
	Use:   "update [id]",
	Short: "Update subscription",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		meta, _ := config.LoadProfiles(cfg.ProfilesMeta())
		if len(meta.Profiles) == 0 {
			ilog.Warn("no subscriptions")
			return
		}
		id := meta.Use
		if len(args) > 0 {
			id = parseID(args)
			if id == 0 {
				return
			}
		}
		p := meta.FindByID(id)
		if p == nil {
			ilog.Warn("subscription id %d not found", id)
			return
		}
		ilog.Info("updating: [%d] %s", id, p.URL)
		if err := sub.Download(p.URL, cfg.TempPath(), cfg.ClashSubUA); err != nil {
			ilog.Warn("download failed: %v", err)
			logSub(fmt.Sprintf("update failed: [%d]", id))
			return
		}
		if err := validateConfigFile(cfg, cfg.TempPath()); err != nil {
			ilog.Info("trying conversion...")
			if err := sub.ConvertDownload(cfg, p.URL, cfg.TempPath()); err != nil {
				ilog.Warn("update failed: %v", err)
				logSub(fmt.Sprintf("update failed: [%d]", id))
				return
			}
		}
		data, _ := os.ReadFile(cfg.TempPath())
		os.WriteFile(p.Path, data, 0644)
		logSub(fmt.Sprintf("updated: [%d]", id))
		ilog.Ok("subscription updated: [%d]", id)

		if meta.Use == id {
			switchSubscription(cfg, id)
		}

		if subUpdateAuto {
			runAutoUpdate()
		}
	},
}

var subUpdateAuto bool

func init() {
	subUpdateCmd.Flags().BoolVar(&subUpdateAuto, "auto", false, "Enable auto-update via cron")
}

func runAutoUpdate() {
	if _, err := exec.LookPath("crontab"); err != nil {
		ilog.Warn("crontab not available, cannot set auto-update")
		return
	}

	clashctlBin := "/usr/local/bin/clashctl"
	if _, err := os.Stat(clashctlBin); os.IsNotExist(err) {
		clashctlBin = "clashctl"
	}

	cronLine := fmt.Sprintf("0 */12 * * * %s sub update --cron", clashctlBin)

	existing, _ := exec.Command("crontab", "-l").Output()
	existingStr := string(existing)
	if !containsLine(existingStr, "clashctl sub update") {
		newCrontab := existingStr
		if newCrontab != "" && newCrontab[len(newCrontab)-1] != '\n' {
			newCrontab += "\n"
		}
		newCrontab += cronLine + "\n"

		cmd := exec.Command("crontab", "-")
		cmd.Stdin = strings.NewReader(newCrontab)
		if err := cmd.Run(); err != nil {
			ilog.Warn("failed to install crontab: %v", err)
			return
		}
		ilog.Ok("auto-update enabled (every 12 hours via cron)")
	} else {
		ilog.Info("auto-update already enabled")
	}
}

func containsLine(text, search string) bool {
	for _, line := range strings.Split(text, "\n") {
		if strings.Contains(line, search) {
			return true
		}
	}
	return false
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
