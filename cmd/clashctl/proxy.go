package main

import (
	"github.com/spf13/cobra"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var proxyCmd = &cobra.Command{
	Use:   "proxy [on|off]",
	Short: "Manage system proxy environment variables",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if len(args) == 0 {
			showProxyStatus()
			return
		}
		switch args[0] {
		case "on":
			setSystemProxy(cfg)
			ilog.Ok("system proxy enabled")
		case "off":
			unsetSystemProxy()
			ilog.Ok("system proxy disabled")
		default:
			ilog.Warn("usage: clashctl proxy [on|off]")
		}
	},
}
