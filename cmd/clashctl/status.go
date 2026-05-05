package main

import (
	"fmt"
	"net"
	"os"
	"strings"
	"time"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var statusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show kernel status",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		svc := kernel.NewServiceManager(cfg)
		running := svc.IsRunning()
		if running {
			ilog.Ok("kernel: running")
			if pid := svc.PID(); pid > 0 {
				ilog.Info("pid: %d", pid)
			}
			if uptime := svc.Uptime(); uptime != "" {
				ilog.Info("uptime: %s", uptime)
			}
		} else {
			ilog.Warn("kernel: stopped")
		}

		info := readRuntimeInfo(cfg)
		apiPort := info.apiPort
		if apiPort == "" {
			apiPort = "9090"
		}
		proxyPort := info.proxyPort
		if proxyPort == "" {
			proxyPort = "7890"
		}

		if proxyPort != "" {
			if portOpen(proxyPort) {
				ilog.Ok("proxy port: %s", proxyPort)
			} else {
				if running {
					ilog.Warn("proxy port: %s  NOT listening", proxyPort)
				} else {
					ilog.Info("proxy port: %s", proxyPort)
				}
			}
		}

		if apiPort != "" {
			if portOpen(apiPort) {
				ilog.Ok("api port: %s", apiPort)
			} else {
				if running {
					ilog.Warn("api port: %s  NOT listening", apiPort)
				} else {
					ilog.Info("api port: %s", apiPort)
				}
			}
		}

		if running {
			api := kernel.NewClient(fmt.Sprintf("http://127.0.0.1:%s", apiPort), info.secret)
			if ver, err := api.GetVersion(); err == nil {
				ilog.Info("version: %s", ver)
			}
		}

		if info.secret != "" {
			ilog.Info("api key: %s", info.secret)
		}

		meta, _ := config.LoadProfiles(cfg.ProfilesMeta())
		if len(meta.Profiles) > 0 {
			current := meta.GetCurrent()
			if current != nil {
				ilog.Info("subscriptions: %d active ([%d] %s)", len(meta.Profiles), current.ID, shortenURL(current.URL))
			} else {
				ilog.Info("subscriptions: %d (none active — run 'clashctl sub use <id>')", len(meta.Profiles))
			}
		}

		hp := os.Getenv("http_proxy")
		if hp != "" {
			ilog.Ok("proxy env: on (%s)", hp)
		}

		showTunStatus(cfg)
	},
}

type runtimeInfo struct {
	proxyPort string
	apiPort   string
	secret    string
}

func readRuntimeInfo(cfg *config.EnvConfig) runtimeInfo {
	var info runtimeInfo
	data, err := os.ReadFile(cfg.RuntimePath())
	if err != nil {
		return info
	}
	for _, line := range strings.Split(string(data), "\n") {
		line = strings.TrimSpace(line)
		switch {
		case strings.HasPrefix(line, "mixed-port:"):
			info.proxyPort = strings.TrimSpace(strings.TrimPrefix(line, "mixed-port:"))
			info.proxyPort = strings.Trim(info.proxyPort, `"'`)
		case strings.HasPrefix(line, "port:"):
			if info.proxyPort == "" {
				info.proxyPort = strings.TrimSpace(strings.TrimPrefix(line, "port:"))
				info.proxyPort = strings.Trim(info.proxyPort, `"'`)
			}
		case strings.HasPrefix(line, "external-controller:"):
			addr := strings.TrimSpace(strings.TrimPrefix(line, "external-controller:"))
			addr = strings.Trim(addr, `"'`)
			if idx := strings.LastIndex(addr, ":"); idx >= 0 {
				info.apiPort = addr[idx+1:]
				info.apiPort = strings.Trim(info.apiPort, `"'`)
			}
		case strings.HasPrefix(line, "secret:"):
			info.secret = strings.TrimSpace(strings.TrimPrefix(line, "secret:"))
		}
	}
	return info
}

func portOpen(port string) bool {
	conn, err := net.DialTimeout("tcp", "127.0.0.1:"+port, 1*time.Second)
	if err != nil {
		return false
	}
	conn.Close()
	return true
}

func shortenURL(url string) string {
	if len(url) > 50 {
		return url[:47] + "..."
	}
	return url
}
