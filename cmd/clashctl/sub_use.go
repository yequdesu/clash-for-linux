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
		id := parseID(args)
		if id == 0 {
			return
		}
		switchSubscription(cfg, id)
	},
}

func switchSubscription(cfg *config.EnvConfig, id int) {
	meta, _ := config.LoadProfiles(cfg.ProfilesMeta())
	if len(meta.Profiles) == 0 {
		ilog.Warn("no subscriptions available, add one first")
		return
	}
	p := meta.FindByID(id)
	if p == nil {
		ilog.Warn("subscription id %d not found", id)
		return
	}
	data, err := os.ReadFile(p.Path)
	if err != nil {
		ilog.Warn("cannot read profile file")
		return
	}
	if err := os.WriteFile(cfg.ConfigPath(), data, 0644); err != nil {
		ilog.Warn("write config: %v", err)
		return
	}
	if err := config.MergeConfig(cfg); err != nil {
		ilog.Warn("merge failed: %v", err)
		return
	}

	svc := kernel.NewServiceManager(cfg)
	if svc.IsRunning() {
		svc.Stop()
	}
	ilog.Info("starting kernel with new config...")
	if err := svc.Start(); err != nil {
		ilog.Warn("start failed: %v (config saved, will retry on next start)", err)
	}
	meta.Use = id
	config.SaveProfiles(cfg.ProfilesMeta(), meta)
	logSub(fmt.Sprintf("switched to: [%d] %s", id, p.URL))
	if svc.IsRunning() {
		ilog.Ok("subscription activated: [%d] on :%s", id, svc.ProxyPort())
	} else {
		ilog.Ok("subscription set: [%d] — run 'clashctl start' to start kernel", id)
	}
}
