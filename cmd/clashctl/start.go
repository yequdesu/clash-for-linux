package main

import (
	"os/exec"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var startCmd = &cobra.Command{
	Use:   "start",
	Short: "Start proxy kernel",
	Long:  "Start the Clash/Mihomo proxy kernel and wait for port readiness.",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		svc := kernel.NewServiceManager(cfg)
		if portOpen(svc.ProxyPort()) {
			ilog.Info("kernel already running on :%s", svc.ProxyPort())
			printEnvHint()
			return
		}

		if svc.IsRunning() {
			ilog.Info("killing stale kernel process...")
			svc.Stop()
		}

		ilog.Info("starting kernel...")
		if err := svc.Start(); err != nil {
			ilog.Fatal("kernel failed: %v", err)
		}
		ilog.Ok("kernel started on :%s", svc.ProxyPort())
		printEnvHint()
	},
}

func printEnvHint() {
	ilog.Info("load proxy env: eval $(clashctl env)")
}

func runYQ(args ...string) error {
	return exec.Command(cfg.YQBin(), args...).Run()
}

func runYQOutput(args ...string) (string, error) {
	out, err := exec.Command(cfg.YQBin(), args...).Output()
	return string(out), err
}
