package main

import (
	"fmt"
	"os"
	"os/exec"
	"strings"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var tunCmd = &cobra.Command{
	Use:   "tun [on|off]",
	Short: "Toggle Tun mode",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if len(args) == 0 {
			showTunStatus(cfg)
			return
		}
		switch args[0] {
		case "on":
			if os.Getuid() != 0 {
				ilog.Fatal("Tun mode requires root. Run with sudo.")
			}

			if err := exec.Command(cfg.YQBin(), "-i",
				".tun.enable = true", cfg.MixinPath(),
			).Run(); err != nil {
				ilog.Warn("set tun failed: %v", err)
				return
			}
			if err := config.MergeConfig(cfg, false); err != nil {
				ilog.Warn("merge: %v", err)
				return
			}

			svc := kernel.NewServiceManager(cfg)
			svc.Stop()

			ensureSetcap(cfg.KernelBin())

			if err := svc.Start(); err != nil {
				if strings.Contains(err.Error(), "exit status 5") || strings.Contains(err.Error(), "died") {
					ilog.Warn("Tun start failed — kernel may lack capability or config issue")
					ilog.Info("try: sudo setcap cap_net_admin,cap_net_raw+ep %s", cfg.KernelBin())
					ilog.Info("check: clashctl log")
				} else {
					ilog.Warn("Tun start failed: %v", err)
				}
				exec.Command(cfg.YQBin(), "-i", ".tun.enable = false", cfg.MixinPath()).Run()
				config.MergeConfig(cfg, false)
				svc.Start()
				return
			}
			if dev, err := verifyTunDevice(); err != nil {
				ilog.Warn("TUN device not detected — %v", err)
				ilog.Info("check: sudo setcap cap_net_admin,cap_net_raw+ep %s", cfg.KernelBin())
				ilog.Info("check: kernel log via 'clashctl log'")
			} else {
				ilog.Ok("TUN device: %s", dev)
			}
			ilog.Ok("Tun mode enabled")
		case "off":
			if os.Getuid() != 0 {
				ilog.Fatal("Tun mode requires root. Run with sudo.")
			}
			exec.Command(cfg.YQBin(), "-i", ".tun.enable = false", cfg.MixinPath()).Run()
			config.MergeConfig(cfg, false)
			svc := kernel.NewServiceManager(cfg)
			svc.Stop()
			if err := svc.Start(); err != nil {
				ilog.Warn("restart after tun off: %v", err)
			}
			ilog.Ok("Tun mode disabled")
		default:
			ilog.Warn("usage: clashctl tun [on|off]")
		}
	},
}

func ensureSetcap(bin string) {
	if _, err := exec.LookPath("setcap"); err != nil {
		return
	}
	out, err := exec.Command("getcap", bin).Output()
	if err == nil && strings.Contains(string(out), "cap_net_admin") {
		return
	}
	exec.Command("setcap", "cap_net_admin,cap_net_raw+ep", bin).Run()
}

func showTunStatus(cfg *config.EnvConfig) {
	out, err := runYQOutput(".tun.enable // \"false\"", cfg.RuntimePath())
	if err != nil {
		ilog.Info("Tun status: unknown")
		return
	}
	if out == "true\n" {
		ilog.Ok("Tun status: enabled")
	} else {
		ilog.Info("Tun status: disabled")
	}
}

func verifyTunDevice() (string, error) {
	out, err := exec.Command("ip", "link", "show").Output()
	if err != nil {
		return "", err
	}
	for _, line := range strings.Split(string(out), "\n") {
		if strings.Contains(line, "utun") || strings.Contains(line, "tun") {
			fields := strings.Fields(line)
			if len(fields) >= 2 {
				name := strings.TrimSuffix(fields[1], ":")
				return name, nil
			}
		}
	}
	return "", fmt.Errorf("no TUN device found")
}
