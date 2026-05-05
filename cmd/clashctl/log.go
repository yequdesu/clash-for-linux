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

		logFile := filepath.Join(cfg.ResourcesDir(), cfg.KernelName+".log")
		if fi, err := os.Stat(logFile); err == nil && fi.Size() > 0 {
			c := exec.Command("tail", "-n", "50", logFile)
			c.Stdin = os.Stdin
			c.Stdout = os.Stdout
			c.Stderr = os.Stderr
			c.Run()
			fmt.Printf("\n[i] full log: %s\n", logFile)
			return
		}

		sysLog := "/var/log/" + cfg.KernelName + ".log"
		if fi, err := os.Stat(sysLog); err == nil && fi.Size() > 0 {
			c := exec.Command("tail", "-n", "50", sysLog)
			c.Stdin = os.Stdin
			c.Stdout = os.Stdout
			c.Stderr = os.Stderr
			c.Run()
			fmt.Printf("\n[i] full log: %s\n", sysLog)
			return
		}

		if _, err := exec.LookPath("journalctl"); err == nil {
			c := exec.Command("journalctl", "-u", cfg.KernelName, "-n", "50", "--no-pager")
			c.Stdin = os.Stdin
			c.Stdout = os.Stdout
			c.Stderr = os.Stderr
			if err := c.Run(); err == nil {
				return
			}
		}

		ilog.Warn("no logs found — kernel may not have been started yet")
		ilog.Info("log path (nohup): %s", filepath.Join(cfg.ResourcesDir(), cfg.KernelName+".log"))
	},
}
