package main

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var subUseCmd = &cobra.Command{
	Use:   "use <id>",
	Short: "Switch to a subscription",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		id, err := parseID(args)
		if err != nil {
			ilog.Fatal("%v", err)
		}
		if err := switchSubscription(cfg, id); err != nil {
			ilog.Fatal("%v", err)
		}
	},
}

func switchSubscription(cfg *config.EnvConfig, id int) error {
	var profile config.Profile
	if err := config.WithProfilesLock(cfg, func() error {
		meta, err := config.LoadProfiles(cfg.ProfilesMeta())
		if err != nil {
			return err
		}
		if len(meta.Profiles) == 0 {
			return fmt.Errorf("no subscriptions available, add one first")
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
	snapshots, err := snapshotFiles(cfg.ConfigPath(), cfg.RuntimePath())
	if err != nil {
		return fmt.Errorf("snapshot current config: %w", err)
	}
	data, err := os.ReadFile(profile.Path)
	if err != nil {
		return fmt.Errorf("cannot read profile file: %w", err)
	}
	if err := config.AtomicWriteFile(cfg.ConfigPath(), data, 0644); err != nil {
		return fmt.Errorf("write config: %w", err)
	}
	if err := mergeRuntimeConfig(cfg, false); err != nil {
		restoreSnapshots(snapshots)
		return fmt.Errorf("merge failed, restored previous config: %w", err)
	}

	svc := newSubscriptionService(cfg)
	wasRunning := svc.IsRunning()
	if wasRunning {
		if err := svc.Stop(); err != nil {
			restoreSnapshots(snapshots)
			return fmt.Errorf("stop current kernel: %w", err)
		}
	}
	subscriptionInfo("starting kernel with new config...")
	if err := svc.Start(); err != nil {
		restoreSnapshots(snapshots)
		if wasRunning {
			_ = svc.Start()
		} else if svc.IsRunning() {
			_ = svc.Stop()
		}
		return fmt.Errorf("start failed, restored previous config: %w", err)
	}
	if err := config.WithProfilesLock(cfg, func() error {
		meta, err := config.LoadProfiles(cfg.ProfilesMeta())
		if err != nil {
			return err
		}
		if meta.FindByID(id) == nil {
			return fmt.Errorf("subscription id %d not found", id)
		}
		meta.Use = id
		return saveProfiles(cfg.ProfilesMeta(), meta)
	}); err != nil {
		restoreSnapshots(snapshots)
		if svc.IsRunning() {
			_ = svc.Stop()
		}
		if wasRunning {
			_ = svc.Start()
		}
		return fmt.Errorf("save active subscription failed, restored previous config: %w", err)
	}
	logSub(fmt.Sprintf("switched to: [%d] %s", id, profile.URL))
	if svc.IsRunning() {
		subscriptionOk("subscription activated: [%d] on :%s", id, svc.ProxyPort())
	} else {
		subscriptionOk("subscription set: [%d] — run 'clashctl start' to start kernel", id)
	}
	return nil
}

var subscriptionQuiet bool

func withSubscriptionQuiet(quiet bool, fn func() error) error {
	previous := subscriptionQuiet
	subscriptionQuiet = quiet
	defer func() { subscriptionQuiet = previous }()
	return fn()
}

func subscriptionInfo(format string, args ...any) {
	if subscriptionQuiet {
		return
	}
	ilog.Info(format, args...)
}

func subscriptionOk(format string, args ...any) {
	if subscriptionQuiet {
		return
	}
	ilog.Ok(format, args...)
}

func subscriptionWarn(format string, args ...any) {
	if subscriptionQuiet {
		return
	}
	ilog.Warn(format, args...)
}

type subscriptionService interface {
	IsRunning() bool
	Stop() error
	Start() error
	ProxyPort() string
}

var newSubscriptionService = func(cfg *config.EnvConfig) subscriptionService {
	return kernel.NewServiceManager(cfg)
}

var mergeRuntimeConfig = config.MergeConfig
