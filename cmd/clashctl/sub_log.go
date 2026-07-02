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
			if os.IsNotExist(err) {
				ilog.Info("no subscription log yet")
				return
			}
			ilog.Fatal("cannot read subscription log: %v", err)
		}
		fmt.Print(string(data))
	},
}

func logSub(msg string) {
	if err := writeSubLog(cfg.ProfilesLog(), msg); err != nil {
		subscriptionWarn("subscription audit log not written: %v", err)
	}
}

func logSubStrict(msg string) error {
	if err := writeSubLog(cfg.ProfilesLog(), msg); err != nil {
		return fmt.Errorf("subscription audit log not written: %w", err)
	}
	return nil
}

func writeSubLog(path, msg string) error {
	f, err := os.OpenFile(path, os.O_CREATE|os.O_WRONLY|os.O_APPEND, 0644)
	if err != nil {
		return err
	}
	ts := time.Now().Format("2006-01-02 15:04:05")
	if _, err := fmt.Fprintf(f, "%s %s\n", ts, msg); err != nil {
		_ = f.Close()
		return err
	}
	return f.Close()
}
