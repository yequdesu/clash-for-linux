package main

import (
	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var restartAllowSSHTunRisk bool

var restartCmd = &cobra.Command{
	Use:   "restart",
	Short: "Restart proxy kernel",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		svc := kernel.NewServiceManager(cfg)
		if err := ensureSafeKernelStartFromSSH(cfg, restartAllowSSHTunRisk, false); err != nil {
			ilog.Fatal("%v", err)
		}
		if svc.IsRunning() {
			if err := svc.Stop(); err != nil {
				ilog.Fatal("stop failed: %v", err)
			}
		}
		ilog.Info("starting kernel...")
		if err := svc.Start(); err != nil {
			ilog.Fatal("kernel failed: %v", err)
		}
		ilog.Ok("kernel restarted on :%s", svc.ProxyPort())
		printEnvHint()
	},
}

func init() {
	restartCmd.Flags().BoolVar(&restartAllowSSHTunRisk, "allow-ssh-tun-risk", false, "allow restarting TUN auto-route from an SSH session")
}
