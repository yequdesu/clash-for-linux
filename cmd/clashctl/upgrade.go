package main

import (
	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var upgradeCmd = &cobra.Command{
	Use:   "upgrade",
	Short: "Upgrade kernel to latest version (via API)",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		info := readRuntimeInfo(cfg)
		api := kernel.NewClient(info.apiBaseURL(), info.secret)
		if err := api.Upgrade(""); err != nil {
			ilog.Fatal("upgrade failed: %v", err)
		}
	},
}
