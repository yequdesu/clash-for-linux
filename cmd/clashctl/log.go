package main

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"

	"github.com/spf13/cobra"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var logCmd = &cobra.Command{
	Use:   "log",
	Short: "View kernel logs",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()

		logFile := cfg.LogFile()
		if fi, err := os.Stat(logFile); err == nil && fi.Size() > 0 {
			c := exec.Command("tail", "-n", "50", logFile)
			c.Stdin = os.Stdin
			c.Stdout = os.Stdout
			c.Stderr = os.Stderr
			if err := c.Run(); err != nil {
				ilog.Fatal("tail log failed: %v", err)
			}
			fmt.Printf("\n[i] full log: %s\n", logFile)
			return
		}

		legacyLog := filepath.Join(cfg.ResourcesDir(), cfg.KernelName+".log")
		if fi, err := os.Stat(legacyLog); err == nil && fi.Size() > 0 {
			c := exec.Command("tail", "-n", "50", legacyLog)
			c.Stdin = os.Stdin
			c.Stdout = os.Stdout
			c.Stderr = os.Stderr
			if err := c.Run(); err != nil {
				ilog.Fatal("tail legacy log failed: %v", err)
			}
			fmt.Printf("\n[i] full log: %s\n", legacyLog)
			return
		}

		if _, err := exec.LookPath("journalctl"); err == nil {
			c := exec.Command("journalctl", "-u", cfg.ServiceName, "-n", "50", "--no-pager")
			c.Stdin = os.Stdin
			c.Stdout = os.Stdout
			c.Stderr = os.Stderr
			if err := c.Run(); err == nil {
				return
			}
		}

		ilog.Warn("no logs found — kernel may not have been started yet")
		ilog.Info("log path: %s", cfg.LogFile())
	},
}
