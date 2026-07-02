package main

import (
	"github.com/spf13/cobra"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var proxyCmd = &cobra.Command{
	Use:   "proxy [on|off|desktop]",
	Short: "Print shell proxy env commands or manage desktop proxy",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if len(args) == 0 {
			showProxyStatus()
			return
		}
		switch args[0] {
		case "on":
			printProxyExports(cfg)
		case "off":
			printProxyUnsets()
		case "desktop":
			handleDesktopProxy(args[1:])
		default:
			ilog.Fatal("usage: clashctl proxy [on|off|desktop on|desktop off|desktop status]")
		}
	},
}
