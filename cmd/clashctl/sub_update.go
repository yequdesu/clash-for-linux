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
	if subUpdateAuto {
		return runAutoUpdate()
	}
	if subUpdateScheduled {
		return updateScheduledSubscriptions()
	}

	var id int
	if len(args) > 0 {
		parsed, err := strconv.Atoi(strings.TrimSpace(args[0]))
		if err != nil || parsed <= 0 {
			return fmt.Errorf("invalid subscription id")
		}
		id = parsed
	}
	return updateSubscriptionByID(id)
}

func updateSubscriptionByID(id int) error {
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

	userAgent := cfg.ClashSubUA
	if strings.TrimSpace(profile.UserAgent) != "" {
		userAgent = strings.TrimSpace(profile.UserAgent)
	}
	downloadOpts, downloadPath, err := subscriptionDownloadOptions(cfg, profile)
	if err != nil {
		return err
	}

	subscriptionInfo("updating: [%d] %s", id, profile.URL)
	subscriptionInfo("update network: %s", downloadPath)
	if err := sub.DownloadWithOptions(profile.URL, tempPath, userAgent, downloadOpts); err != nil {
		logSub(fmt.Sprintf("update failed: [%d]", id))
		return fmt.Errorf("download failed: %w", err)
	}
	switch strings.ToLower(strings.TrimSpace(profile.ConvertMode)) {
	case "force":
		subscriptionInfo("conversion forced by profile metadata...")
		if err := sub.ConvertDownload(cfg, profile.URL, tempPath); err != nil {
			logSub(fmt.Sprintf("update failed: [%d]", id))
			return fmt.Errorf("update failed: %w", err)
		}
		if err := validateConfigFile(cfg, tempPath); err != nil {
			logSub(fmt.Sprintf("update failed: [%d]", id))
			return fmt.Errorf("converted config validation failed: %w", err)
		}
	case "off":
		if err := validateConfigFile(cfg, tempPath); err != nil {
			logSub(fmt.Sprintf("update failed: [%d]", id))
			return fmt.Errorf("config validation failed: %w", err)
		}
	default:
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

	return nil
}

func subscriptionDownloadOptions(cfg *config.EnvConfig, profile config.Profile) (sub.DownloadOptions, string, error) {
	mode := strings.ToLower(strings.TrimSpace(profile.UpdateProxy))
	if mode == "" {
		mode = "auto"
	}
	switch mode {
	case "direct":
		return sub.DownloadOptions{ProxyMode: sub.ProxyModeDirect}, "direct", nil
	case "system":
		return sub.DownloadOptions{ProxyMode: sub.ProxyModeSystem}, "system proxy env", nil
	case "core", "auto":
		info := config.LoadRuntimeInfo(cfg)
		port := info.ProxyPort
		if port == "" {
			port = "7890"
		}
		coreProxyURL := fmt.Sprintf("http://127.0.0.1:%s", port)
		if portOpen(port) {
			return sub.DownloadOptions{ProxyMode: sub.ProxyModeCore, ProxyURL: coreProxyURL}, "core " + coreProxyURL, nil
		}
		if mode == "core" {
			return sub.DownloadOptions{}, "", fmt.Errorf("subscription update_proxy=core requires kernel proxy port %s to be listening", port)
		}
		return sub.DownloadOptions{ProxyMode: sub.ProxyModeSystem}, "system proxy env (core port not listening)", nil
	default:
		return sub.DownloadOptions{}, "", fmt.Errorf("unsupported subscription update_proxy %q", profile.UpdateProxy)
	}
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
		now := timeNow()
		p.Updated = now.Format(profileTimeFormat)
		p.LastUpdated = p.Updated
		p.LastError = ""
		p.NextUpdate = nextProfileUpdateString(*p, now)
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
var subUpdateScheduled bool

func init() {
	subUpdateCmd.Flags().BoolVar(&subUpdateAuto, "auto", false, "Enable auto-update via cron")
	subUpdateCmd.Flags().BoolVar(&subUpdateCron, "cron", false, "Run from cron without prompts")
	subUpdateCmd.Flags().BoolVar(&subUpdateScheduled, "scheduled", false, "Update only subscriptions due by per-profile policy")
}

func runAutoUpdate() error {
	if _, err := exec.LookPath("crontab"); err != nil {
		return fmt.Errorf("crontab not available, cannot set auto-update")
	}

	clashctlBin := "/usr/local/bin/clashctl"
	if _, err := os.Stat(clashctlBin); os.IsNotExist(err) {
		clashctlBin = "clashctl"
	}

	cronLine := fmt.Sprintf("*/10 * * * * %s sub update --scheduled --cron", clashctlBin)

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
	subscriptionOk("auto-update enabled (scheduled profiles checked every 10 minutes via cron)")
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
				if strings.Contains(line, "--cron") && strings.Contains(line, "--scheduled") {
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

func updateScheduledSubscriptions() error {
	now := timeNow()
	var due []int
	if err := config.WithProfilesLock(cfg, func() error {
		meta, err := config.LoadProfiles(cfg.ProfilesMeta())
		if err != nil {
			return err
		}
		for _, p := range meta.Profiles {
			if profileDueForUpdate(p, now) {
				due = append(due, p.ID)
			}
		}
		return nil
	}); err != nil {
		return err
	}
	if len(due) == 0 {
		subscriptionInfo("no subscriptions due for scheduled update")
		return nil
	}

	var failures []string
	for _, id := range due {
		if err := updateSubscriptionByID(id); err != nil {
			failures = append(failures, fmt.Sprintf("[%d] %v", id, err))
			if recordErr := recordProfileUpdateError(cfg, id, err, now); recordErr != nil {
				failures = append(failures, fmt.Sprintf("[%d] record error failed: %v", id, recordErr))
			}
		}
	}
	if len(failures) > 0 {
		return fmt.Errorf("scheduled update failed: %s", strings.Join(failures, "; "))
	}
	return nil
}

func recordProfileUpdateError(cfg *config.EnvConfig, id int, updateErr error, now time.Time) error {
	return updateProfileMetadata(cfg, id, func(p *config.Profile) error {
		p.LastError = fmt.Sprintf("%s %v", now.Format(profileTimeFormat), updateErr)
		p.NextUpdate = nextProfileRetryString(*p, now)
		return nil
	})
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
