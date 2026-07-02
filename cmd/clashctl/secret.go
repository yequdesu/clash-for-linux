package main

import (
	"fmt"

	"github.com/spf13/cobra"

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
				ilog.Info("current secret: set (hidden)")
				ilog.Info("use 'clashctl secret show' to print it")
			} else {
				ilog.Warn("no secret set")
			}
			return
		}
		if args[0] == "show" {
			info := readRuntimeInfo(cfg)
			if info.secret == "" {
				ilog.Fatal("no secret set")
			}
			ilog.Warn("printing API secret to terminal")
			ilog.Info("current secret: %s", info.secret)
			return
		}
		newSecret := args[0]
		if err := setAPISecret(newSecret); err != nil {
			ilog.Fatal("%v", err)
		}
		ilog.Ok("secret updated (restart kernel to apply)")
	},
}

func setAPISecret(newSecret string) error {
	if newSecret == "" {
		return fmt.Errorf("secret cannot be empty")
	}
	return applyMixinUpdates(map[string]any{"secret": newSecret})
}
