package main

import (
	"os/exec"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var startAllowSSHTunRisk bool

var startCmd = &cobra.Command{
	Use:   "start",
	Short: "Start proxy kernel",
	Long:  "Start the Clash/Mihomo proxy kernel and wait for port readiness.",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		svc := kernel.NewServiceManager(cfg)
		if portOpen(svc.ProxyPort()) {
			ilog.Info("kernel already running on :%s", svc.ProxyPort())
			printEnvHint()
			return
		}

		if err := ensureSafeKernelStartFromSSH(cfg, startAllowSSHTunRisk, false); err != nil {
			ilog.Fatal("%v", err)
		}

		if svc.IsRunning() {
			ilog.Info("killing stale kernel process...")
			if err := svc.Stop(); err != nil {
				ilog.Fatal("stop stale kernel failed: %v", err)
			}
		}

		ilog.Info("starting kernel...")
		if err := svc.Start(); err != nil {
			ilog.Fatal("kernel failed: %v", err)
		}
		ilog.Ok("kernel started on :%s", svc.ProxyPort())
		printEnvHint()
	},
}

func init() {
	startCmd.Flags().BoolVar(&startAllowSSHTunRisk, "allow-ssh-tun-risk", false, "allow starting TUN auto-route from an SSH session")
}

func printEnvHint() {
	ilog.Info("load proxy env: eval $(clashctl env)")
}

func runYQ(args ...string) error {
	return exec.Command(cfg.YQBin(), args...).Run()
}

func runYQOutput(args ...string) (string, error) {
	out, err := exec.Command(cfg.YQBin(), args...).Output()
	return string(out), err
}
