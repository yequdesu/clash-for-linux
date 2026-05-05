package main

import (
	"fmt"

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
		api := kernel.NewClient(
			fmt.Sprintf("http://127.0.0.1:%s", info.apiPort),
			info.secret,
		)
		if err := api.Upgrade(""); err != nil {
			ilog.Warn("upgrade failed: %v", err)
			return
		}
	},
}
