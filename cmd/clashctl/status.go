package main

import (
	"fmt"
	"net"
	"os"
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
		apiPort := info.apiPortOrDefault()
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
			if apiOpen(info) {
				ilog.Ok("api: %s", info.apiAddress())
			} else {
				if running {
					ilog.Warn("api: %s  NOT listening", info.apiAddress())
				} else {
					ilog.Info("api: %s", info.apiAddress())
				}
			}
		}

		if running {
			api := kernel.NewClient(info.apiBaseURL(), info.secret)
			if ver, err := api.GetVersion(); err == nil {
				ilog.Info("version: %s", ver)
			}
		}

		if info.secret != "" {
			ilog.Info("api key: set")
		} else {
			ilog.Warn("api key: empty")
		}

		subscriptions, err := statusSubscriptionSummary(cfg)
		if err != nil {
			ilog.Warn("subscriptions: cannot read profiles metadata: %v", err)
		} else if subscriptions != "" {
			ilog.Info("%s", subscriptions)
		}

		hp := os.Getenv("http_proxy")
		if hp != "" {
			ilog.Ok("proxy env: on (%s)", hp)
		}

		showTunStatus(cfg)
	},
}

func statusSubscriptionSummary(cfg *config.EnvConfig) (string, error) {
	meta, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		return "", err
	}
	if len(meta.Profiles) == 0 {
		return "", nil
	}
	current := meta.GetCurrent()
	if current != nil {
		return fmt.Sprintf("subscriptions: %d active ([%d] %s)", len(meta.Profiles), current.ID, shortenURL(current.URL)), nil
	}
	return fmt.Sprintf("subscriptions: %d (none active — run 'clashctl sub use <id>')", len(meta.Profiles)), nil
}

type runtimeInfo struct {
	proxyPort string
	apiHost   string
	apiPort   string
	secret    string
	tun       bool
}

func readRuntimeInfo(cfg *config.EnvConfig) runtimeInfo {
	parsed := config.LoadRuntimeInfo(cfg)
	return runtimeInfo{
		proxyPort: parsed.ProxyPort,
		apiHost:   parsed.APIHost,
		apiPort:   parsed.APIPort,
		secret:    parsed.Secret,
		tun:       parsed.TunEnabled,
	}
}

func (r runtimeInfo) configRuntimeInfo() config.RuntimeInfo {
	return config.RuntimeInfo{APIHost: r.apiHost, APIPort: r.apiPort}
}

func (r runtimeInfo) apiPortOrDefault() string {
	return r.configRuntimeInfo().APIPortOrDefault("9090")
}

func (r runtimeInfo) apiAddress() string {
	return r.configRuntimeInfo().APIAddress("9090")
}

func (r runtimeInfo) apiBaseURL() string {
	return r.configRuntimeInfo().APIBaseURL("9090")
}

func apiOpen(info runtimeInfo) bool {
	return tcpOpen(info.apiAddress())
}

func portOpen(port string) bool {
	return tcpOpen(net.JoinHostPort("127.0.0.1", port))
}

func tcpOpen(address string) bool {
	conn, err := net.DialTimeout("tcp", address, 1*time.Second)
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
