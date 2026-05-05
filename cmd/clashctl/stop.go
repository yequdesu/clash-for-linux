package main

import (
	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var stopCmd = &cobra.Command{
	Use:   "stop",
	Short: "Stop proxy kernel",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		svc := kernel.NewServiceManager(cfg)
		if !svc.IsRunning() {
			ilog.Info("kernel not running")
			unsetSystemProxy()
			return
		}
		if err := svc.Stop(); err != nil {
			ilog.Warn("stop failed: %v", err)
		}
		ilog.Ok("kernel stopped")
		unsetSystemProxy()
		ilog.Ok("proxy environment cleared")
	},
}
