package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var upgradeKernelCmd = &cobra.Command{
	Use:   "upgrade-kernel",
	Short: "Upgrade Mihomo kernel binary from GitHub",
	Long: `Download and install the latest Mihomo kernel binary.

This command:
  1. Checks the current kernel version via API
  2. Gets the latest release from GitHub (MetaCubeX/mihomo)
  3. Downloads the new kernel binary
  4. Stops the running kernel
  5. Replaces the binary
  6. Starts the kernel
`,
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		svc := kernel.NewServiceManager(cfg)

		info := readRuntimeInfo(cfg)
		apiPort := info.apiPort
		if apiPort == "" {
			apiPort = "9090"
		}

		currentVersion := ""
		if svc.IsRunning() {
			api := kernel.NewClient(fmt.Sprintf("http://127.0.0.1:%s", apiPort), info.secret)
			if ver, err := api.GetVersion(); err == nil {
				currentVersion = ver
				ilog.Info("current version: %s", currentVersion)
			}
		}

		latestVer, downloadURL := getLatestKernelRelease(cfg.KernelName, cfg.URLGhProxy)
		if latestVer == "" {
			ilog.Warn("could not determine latest version")
			return
		}

		ilog.Info("latest version: %s", latestVer)
		if latestVer == currentVersion && currentVersion != "" {
			ilog.Info("already running latest version")
			return
		}

		ilog.Info("downloading kernel %s ...", latestVer)
		tmpPath := filepath.Join(cfg.BinDir(), cfg.KernelName+".new")
		if err := downloadBinary(downloadURL, tmpPath); err != nil {
			ilog.Warn("download failed: %v", err)
			return
		}

		ilog.Info("stopping kernel before replacing binary...")
		svc.Stop()
		time.Sleep(1 * time.Second)

		if err := os.Rename(tmpPath, cfg.KernelBin()); err != nil {
			os.Remove(tmpPath)
			ilog.Warn("replace failed: %v", err)
			return
		}
		os.Chmod(cfg.KernelBin(), 0755)
		ensureSetcap(cfg.KernelBin())

		ilog.Info("starting new kernel...")
		if err := svc.Start(); err != nil {
			ilog.Warn("start failed: %v", err)
			return
		}
		ilog.Ok("kernel upgraded to %s", latestVer)
	},
}

func getLatestKernelRelease(kernelName, ghProxy string) (version, downloadURL string) {
	_ = kernelName
	apiURL := "https://api.github.com/repos/MetaCubeX/mihomo/releases/latest"
	client := &http.Client{Timeout: 15 * time.Second}
	req, _ := http.NewRequest("GET", apiURL, nil)
	req.Header.Set("Accept", "application/vnd.github.v3+json")
	resp, err := client.Do(req)
	if err != nil {
		return "", ""
	}
	defer resp.Body.Close()

	body, _ := io.ReadAll(io.LimitReader(resp.Body, 50000))
	var release struct {
		TagName string `json:"tag_name"`
		Assets  []struct {
			Name               string `json:"name"`
			BrowserDownloadURL string `json:"browser_download_url"`
		} `json:"assets"`
	}
	if err := json.Unmarshal(body, &release); err != nil {
		return "", ""
	}

	version = release.TagName
	for _, a := range release.Assets {
		name := strings.ToLower(a.Name)
		if strings.Contains(name, "linux") && strings.Contains(name, "amd64") && !strings.Contains(name, "compatible") {
			downloadURL = a.BrowserDownloadURL
			break
		}
	}
	if downloadURL == "" && len(release.Assets) > 0 {
		for _, a := range release.Assets {
			name := strings.ToLower(a.Name)
			if strings.Contains(name, "linux") && strings.Contains(name, "x86_64") {
				downloadURL = a.BrowserDownloadURL
				break
			}
		}
	}

	if downloadURL != "" && ghProxy != "" {
		downloadURL = strings.TrimRight(ghProxy, "/") + "/" + downloadURL
	}
	return version, downloadURL
}

func downloadBinary(url, dest string) error {
	resp, err := http.Get(url)
	if err != nil {
		return fmt.Errorf("GET: %w", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != 200 {
		return fmt.Errorf("HTTP %d", resp.StatusCode)
	}
	f, err := os.Create(dest)
	if err != nil {
		return err
	}
	defer f.Close()
	_, err = io.Copy(f, resp.Body)
	return err
}
