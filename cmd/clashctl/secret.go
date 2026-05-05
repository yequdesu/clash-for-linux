package main

import (
	"os/exec"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var secretCmd = &cobra.Command{
	Use:   "secret [new-secret]",
	Short: "Show or set API secret",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if len(args) == 0 {
			info := readRuntimeInfo(cfg)
			if info.secret != "" {
				ilog.Info("current secret: %s", info.secret)
			} else {
				ilog.Warn("no secret set")
			}
			return
		}
		newSecret := args[0]
		if err := exec.Command(cfg.YQBin(), "-i",
			".secret = \""+newSecret+"\"", cfg.MixinPath(),
		).Run(); err != nil {
			ilog.Warn("set secret failed: %v", err)
			return
		}
		if err := config.MergeConfig(cfg, false); err != nil {
			ilog.Warn("merge failed: %v", err)
			return
		}
		ilog.Ok("secret updated (restart kernel to apply)")
	},
}
