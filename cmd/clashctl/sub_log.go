package main

import (
	"fmt"
	"os"
	"time"

	"github.com/spf13/cobra"

	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var subLogCmd = &cobra.Command{
	Use:   "log",
	Short: "View subscription operation log",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		data, err := os.ReadFile(cfg.ProfilesLog())
		if err != nil {
			ilog.Info("no subscription log yet")
			return
		}
		fmt.Print(string(data))
	},
}

func logSub(msg string) {
	f, err := os.OpenFile(cfg.ProfilesLog(), os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	if err != nil {
		return
	}
	defer f.Close()
	ts := time.Now().Format("2006-01-02 15:04:05")
	fmt.Fprintf(f, "%s %s\n", ts, msg)
}
