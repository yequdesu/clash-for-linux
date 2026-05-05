package main

import (
	"fmt"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var configCmd = &cobra.Command{
	Use:   "config <edit|view|raw|merge>",
	Short: "Manage configuration",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if len(args) == 0 {
			ilog.Warn("usage: clashctl config <edit|view|raw|merge>")
			return
		}
		switch args[0] {
		case "edit":
			if err := launchEditor(cfg.MixinPath()); err != nil {
				ilog.Warn("editor failed: %v", err)
				return
			}
			if err := config.MergeConfig(cfg, false); err != nil {
				ilog.Warn("merge failed: %v", err)
				return
			}
			ilog.Ok("config updated (restart to apply)")
		case "view":
			data, err := readFileString(cfg.RuntimePath())
			if err != nil {
				ilog.Warn("cannot read runtime config")
				return
			}
			fmt.Print(data)
		case "raw":
			data, err := readFileString(cfg.ConfigPath())
			if err != nil {
				ilog.Warn("cannot read subscription config")
				return
			}
			fmt.Print(data)
		case "merge":
			autoFix, _ := cmd.Flags().GetBool("autofix")
			if err := config.MergeConfig(cfg, autoFix); err != nil {
				ilog.Warn("merge failed: %v", err)
				return
			}
			ilog.Ok("config merged")
		default:
			ilog.Warn("usage: clashctl config <edit|view|raw|merge>")
		}
	},
}

func init() {
	configCmd.Flags().Bool("autofix", false, "auto-fix proxy group reference mismatches")
}
