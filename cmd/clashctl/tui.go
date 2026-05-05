package main

import (
	"os"

	"github.com/spf13/cobra"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var tuiCmd = &cobra.Command{
	Use:   "tui",
	Short: "Launch TUI dashboard (if installed)",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		tuiBin := "/usr/local/bin/clash-tui"
		if _, err := os.Stat(tuiBin); os.IsNotExist(err) {
			ilog.Fatal("clash-tui is not installed. Install it separately.")
		}
		ilog.Info("launching TUI...")
		c := execCommand(tuiBin)
		c.Stdin = os.Stdin
		c.Stdout = os.Stdout
		c.Stderr = os.Stderr
		if err := c.Run(); err != nil {
			ilog.Warn("TUI exited with error: %v", err)
		}
	},
}
