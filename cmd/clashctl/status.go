package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strconv"
	"time"

	"github.com/spf13/cobra"
	"github.com/yequdesu/clashctl/internal/config"
	"github.com/yequdesu/clashctl/internal/kernel"
)

var (
	statusJSON bool
)

var statusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show proxy running status",
	Long:  "Show detailed proxy running status including kernel version, traffic, connections, and more.",
	Run:   runStatus,
}

func init() {
	statusCmd.Flags().BoolVar(&statusJSON, "json", false, "Output in JSON format")
	rootCmd.AddCommand(statusCmd)
}

type StatusInfo struct {
	Running           bool   `json:"running"`
	PID              int    `json:"pid,omitempty"`
	Uptime           string `json:"uptime,omitempty"`
	Version          string `json:"version,omitempty"`
	Mode             string `json:"mode,omitempty"`
	MixedPort        int    `json:"mixed_port,omitempty"`
	SocksPort        int    `json:"socks_port,omitempty"`
	TUN              bool   `json:"tun"`
	ActiveConns      int    `json:"active_connections"`
	TotalConns       int    `json:"total_connections"`
	UploadSpeed      uint64 `json:"upload_speed"`
	DownloadSpeed    uint64 `json:"download_speed"`
	TotalUpload      uint64 `json:"total_upload"`
	TotalDownload    uint64 `json:"total_download"`
	Memory           uint64 `json:"memory_bytes"`
	SubscriptionName string `json:"subscription_name,omitempty"`
	SubscriptionID   int    `json:"subscription_id,omitempty"`
}

func runStatus(cmd *cobra.Command, args []string) {
	info := StatusInfo{}

	envConfig, _ := config.LoadEnv(getEnvFilePath())

	running := isRunning()
	info.Running = running

	if running {
		pidStr := readPID()
		info.PID, _ = strconv.Atoi(pidStr)

		port := envConfig.GetPort()
		apiClient := kernel.NewAPIClient(fmt.Sprintf("http://127.0.0.1:%d", port), readSecret())

		version, err := apiClient.GetVersion()
		if err == nil {
			info.Version = version
		}

		// Get traffic info
		traffic, err := apiClient.GetTraffic()
		if err == nil {
			info.UploadSpeed = traffic.Up
			info.DownloadSpeed = traffic.Down
		}

		// Get connections
		conns, err := apiClient.GetConnections()
		if err == nil {
			info.TotalConns = len(conns)
			for _, c := range conns {
				if !c.UploadClosed || !c.DownloadClosed {
					info.ActiveConns++
				}
			}
		}

		// Get memory
		mem, err := apiClient.GetMemory()
		if err == nil {
			info.Memory = mem.InUse
		}

		// Get config
		runtimeConfig, err := apiClient.GetConfig()
		if err == nil {
			info.Mode = runtimeConfig.Mode
			info.MixedPort = runtimeConfig.MixedPort
			info.SocksPort = runtimeConfig.SocksPort
			if runtimeConfig.TUN != nil {
				info.TUN = runtimeConfig.TUN.Enable
			}
		}
	}

	// Get subscription info
	profilesCfg, _ := config.LoadProfiles(filepath.Join(clashResourcesDir, "profiles.yaml"))
	if cfg := config.GetActiveProfile(profilesCfg); cfg != nil {
		info.SubscriptionName = cfg.Name
		info.SubscriptionID = cfg.ID
	}

	if statusJSON {
		data, _ := json.MarshalIndent(info, "", "  ")
		fmt.Println(string(data))
		return
	}

	// Text output
	statusDot := gray("○")
	statusText := gray("Stopped")
	if running {
		statusDot = green("●")
		statusText = green("Running")
	}
	fmt.Printf("Status: %s  %s", statusDot, statusText)
	if running {
		fmt.Printf("  (PID: %d", info.PID)
		// estimate uptime from pid file mtime
		if stat, err := os.Stat(pidFilePath()); err == nil {
			uptime := time.Since(stat.ModTime())
			fmt.Printf(", Uptime: %s", formatDuration(uptime))
		}
		fmt.Print(")")
	}
	fmt.Println()

	if info.Version != "" {
		fmt.Printf("Kernel: %s\n", cyan(info.Version))
	}

	if info.Mode != "" {
		modeDisplay := info.Mode
		if info.Mode == "rule" {
			modeDisplay = "Rule (Rules)"
		} else if info.Mode == "global" {
			modeDisplay = "Global"
		} else if info.Mode == "direct" {
			modeDisplay = "Direct"
		}
		fmt.Printf("Mode: %s\n", modeDisplay)
	}

	if info.MixedPort > 0 {
		fmt.Printf("Port: Mixed: %s | SOCKS5: %s\n",
			cyan(fmt.Sprintf("%d", info.MixedPort)),
			cyan(fmt.Sprintf("%d", info.SocksPort)))
	}

	tunDot := gray("○")
	tunText := gray("Disabled")
	if info.TUN {
		tunDot = green("●")
		tunText = green("Enabled")
	}
	fmt.Printf("TUN: %s  %s\n", tunDot, tunText)

	fmt.Printf("Connections: %d active / %d total\n",
		cyan(fmt.Sprintf("%d", info.ActiveConns)),
		cyan(fmt.Sprintf("%d", info.TotalConns)))

	upSpeed := formatSpeed(info.UploadSpeed)
	downSpeed := formatSpeed(info.DownloadSpeed)
	fmt.Printf("Traffic: ↑ %s  ↓ %s", cyan(upSpeed), cyan(downSpeed))
	if info.TotalUpload > 0 || info.TotalDownload > 0 {
		fmt.Printf("  (Total: ↑ %s ↓ %s)", formatBytes(info.TotalUpload), formatBytes(info.TotalDownload))
	}
	fmt.Println()

	if info.Memory > 0 {
		fmt.Printf("Memory: %s\n", formatBytes(info.Memory))
	}

	if info.SubscriptionName != "" {
		fmt.Printf("Current subscription: %s", bold(info.SubscriptionName))
		fmt.Printf(" (ID: %d)\n", info.SubscriptionID)
	}
}
