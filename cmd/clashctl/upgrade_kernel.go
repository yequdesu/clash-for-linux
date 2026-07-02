package main

import (
	"compress/gzip"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os"
	pathpkg "path"
	"path/filepath"
	"runtime"
	"strings"
	"time"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
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

		currentVersion := ""
		if svc.IsRunning() {
			api := kernel.NewClient(info.apiBaseURL(), info.secret)
			if ver, err := api.GetVersion(); err == nil {
				currentVersion = ver
				ilog.Info("current version: %s", currentVersion)
			}
		}

		latestVer, downloadURL, checksumURL := getLatestKernelRelease(cfg.KernelName, cfg.URLGhProxy)
		if latestVer == "" {
			ilog.Fatal("could not determine latest version")
		}
		if downloadURL == "" {
			ilog.Fatal("could not find a linux kernel asset for %s", runtime.GOARCH)
		}

		ilog.Info("latest version: %s", latestVer)
		if latestVer == currentVersion && currentVersion != "" {
			ilog.Info("already running latest version")
			return
		}

		if err := ensureSafeKernelStartFromSSH(cfg, upgradeKernelAllowSSHTunRisk, false); err != nil {
			ilog.Fatal("%v", err)
		}

		ilog.Info("downloading kernel %s ...", latestVer)
		tmpPath := filepath.Join(cfg.BinDir(), cfg.KernelName+".new")
		checksumVerified, err := downloadKernelBinary(downloadURL, checksumURL, tmpPath, upgradeKernelAllowUnsigned)
		if err != nil {
			ilog.Fatal("download failed: %v", err)
		}

		ilog.Info("stopping kernel before replacing binary...")
		if err := svc.Stop(); err != nil {
			os.Remove(tmpPath)
			ilog.Fatal("stop failed: %v", err)
		}
		time.Sleep(1 * time.Second)

		backupPath := filepath.Join(cfg.BinDir(), fmt.Sprintf("%s.bak.%s", cfg.KernelName, time.Now().Format("20060102150405")))
		if err := copyFilePath(cfg.KernelBin(), backupPath); err != nil {
			os.Remove(tmpPath)
			ilog.Fatal("backup failed: %v", err)
		}

		if err := os.Rename(tmpPath, cfg.KernelBin()); err != nil {
			os.Remove(tmpPath)
			ilog.Fatal("replace failed: %v", err)
		}
		os.Chmod(cfg.KernelBin(), 0755)
		ensureSetcap(cfg.KernelBin())

		ilog.Info("starting new kernel...")
		if err := svc.Start(); err != nil {
			ilog.Warn("start failed: %v", err)
			ilog.Info("rolling back kernel binary...")
			_ = os.Remove(cfg.KernelBin())
			if restoreErr := os.Rename(backupPath, cfg.KernelBin()); restoreErr != nil {
				ilog.Fatal("rollback failed: %v", restoreErr)
			}
			os.Chmod(cfg.KernelBin(), 0755)
			ensureSetcap(cfg.KernelBin())
			if restartErr := svc.Start(); restartErr != nil {
				ilog.Fatal("rollback restart failed: %v", restartErr)
			}
			ilog.Fatal("kernel upgrade failed; rolled back to previous kernel")
		}
		if err := updateInstallStateKernel(cfg, latestVer, downloadURL, checksumURL, checksumVerified, upgradeKernelAllowUnsigned); err != nil {
			ilog.Warn("install state update failed: %v", err)
			ilog.Info("rolling back kernel binary...")
			if stopErr := svc.Stop(); stopErr != nil {
				ilog.Fatal("rollback stop failed: %v", stopErr)
			}
			_ = os.Remove(cfg.KernelBin())
			if restoreErr := os.Rename(backupPath, cfg.KernelBin()); restoreErr != nil {
				ilog.Fatal("rollback failed: %v", restoreErr)
			}
			os.Chmod(cfg.KernelBin(), 0755)
			ensureSetcap(cfg.KernelBin())
			if restartErr := svc.Start(); restartErr != nil {
				ilog.Fatal("rollback restart failed: %v", restartErr)
			}
			ilog.Fatal("install state update failed; rolled back to previous kernel")
		}
		ilog.Ok("kernel upgraded to %s", latestVer)
	},
}

var upgradeKernelAllowUnsigned bool
var upgradeKernelAllowSSHTunRisk bool

func init() {
	upgradeKernelCmd.Flags().BoolVar(&upgradeKernelAllowUnsigned, "allow-unsigned", false, "allow upgrade when no SHA256 checksum asset is available")
	upgradeKernelCmd.Flags().BoolVar(&upgradeKernelAllowSSHTunRisk, "allow-ssh-tun-risk", false, "allow restarting TUN auto-route from an SSH session")
}

type releaseAsset struct {
	Name               string `json:"name"`
	BrowserDownloadURL string `json:"browser_download_url"`
}

func getLatestKernelRelease(kernelName, ghProxy string) (version, downloadURL, checksumURL string) {
	_ = kernelName
	apiURL := "https://api.github.com/repos/MetaCubeX/mihomo/releases/latest"
	client := &http.Client{Timeout: 15 * time.Second}
	req, _ := http.NewRequest("GET", apiURL, nil)
	req.Header.Set("Accept", "application/vnd.github.v3+json")
	resp, err := client.Do(req)
	if err != nil {
		return "", "", ""
	}
	defer resp.Body.Close()

	body, _ := io.ReadAll(io.LimitReader(resp.Body, 50000))
	var release struct {
		TagName string         `json:"tag_name"`
		Assets  []releaseAsset `json:"assets"`
	}
	if err := json.Unmarshal(body, &release); err != nil {
		return "", "", ""
	}

	version = release.TagName
	archTokens := kernelAssetArchTokens()
	for _, a := range release.Assets {
		name := strings.ToLower(a.Name)
		if strings.Contains(name, "linux") && containsAny(name, archTokens) && !strings.Contains(name, "compatible") {
			downloadURL = a.BrowserDownloadURL
			break
		}
	}
	if downloadURL != "" {
		checksumURL = findChecksumAsset(release.Assets)
	}

	if downloadURL != "" && ghProxy != "" {
		downloadURL = strings.TrimRight(ghProxy, "/") + "/" + downloadURL
	}
	if checksumURL != "" && ghProxy != "" {
		checksumURL = strings.TrimRight(ghProxy, "/") + "/" + checksumURL
	}
	return version, downloadURL, checksumURL
}

func kernelAssetArchTokens() []string {
	switch runtime.GOARCH {
	case "amd64":
		return []string{"amd64", "x86_64"}
	case "arm64":
		return []string{"arm64", "aarch64"}
	case "arm":
		return []string{"armv7", "armv7l"}
	default:
		return []string{runtime.GOARCH}
	}
}

func containsAny(s string, tokens []string) bool {
	for _, token := range tokens {
		if strings.Contains(s, token) {
			return true
		}
	}
	return false
}

func findChecksumAsset(assets []releaseAsset) string {
	for _, a := range assets {
		name := strings.ToLower(a.Name)
		if strings.Contains(name, "sha256") || strings.Contains(name, "checksum") || strings.Contains(name, "checksums") {
			return a.BrowserDownloadURL
		}
	}
	return ""
}

func downloadKernelBinary(downloadURL, checksumURL, dest string, allowUnsigned bool) (bool, error) {
	rawPath := dest + ".download"
	defer os.Remove(rawPath)

	if err := downloadBinary(downloadURL, rawPath); err != nil {
		return false, err
	}
	checksumVerified := false
	if checksumURL == "" {
		if !allowUnsigned {
			return false, fmt.Errorf("checksum asset not found; rerun with --allow-unsigned only if you trust the download source")
		}
		ilog.Warn("checksum asset not found; continuing because --allow-unsigned was set")
	} else if err := verifyDownloadChecksum(checksumURL, downloadURL, rawPath); err != nil {
		return false, fmt.Errorf("checksum verification failed: %w", err)
	} else {
		checksumVerified = true
		ilog.Ok("checksum verified")
	}

	if strings.HasSuffix(strings.ToLower(downloadURL), ".gz") {
		return checksumVerified, gunzipFile(rawPath, dest)
	}
	return checksumVerified, moveFile(rawPath, dest)
}

func downloadBinary(url, dest string) error {
	client := &http.Client{Timeout: 60 * time.Second}
	resp, err := client.Get(url)
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

func gunzipFile(src, dest string) error {
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer in.Close()

	gz, err := gzip.NewReader(in)
	if err != nil {
		return fmt.Errorf("gzip: %w", err)
	}
	defer gz.Close()

	out, err := os.Create(dest)
	if err != nil {
		return err
	}
	defer out.Close()

	if _, err := io.Copy(out, gz); err != nil {
		return err
	}
	return out.Sync()
}

func moveFile(src, dest string) error {
	if err := os.Rename(src, dest); err == nil {
		return nil
	}
	if err := copyFilePath(src, dest); err != nil {
		return err
	}
	return os.Remove(src)
}

func copyFilePath(src, dst string) error {
	in, err := os.Open(src)
	if err != nil {
		return err
	}
	defer in.Close()

	out, err := os.OpenFile(dst, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, 0755)
	if err != nil {
		return err
	}
	defer out.Close()

	if _, err := io.Copy(out, in); err != nil {
		return err
	}
	return out.Sync()
}

func verifyDownloadChecksum(checksumURL, downloadURL, filePath string) error {
	content, err := downloadText(checksumURL)
	if err != nil {
		return err
	}

	filename := downloadFileName(downloadURL)
	expected, err := checksumForFile(content, filename)
	if err != nil {
		return err
	}

	data, err := os.ReadFile(filePath)
	if err != nil {
		return err
	}
	sum := sha256.Sum256(data)
	actual := hex.EncodeToString(sum[:])
	if !strings.EqualFold(actual, expected) {
		return fmt.Errorf("sha256 mismatch for %s: got %s want %s", filename, actual, expected)
	}
	return nil
}

func downloadText(rawURL string) (string, error) {
	client := &http.Client{Timeout: 30 * time.Second}
	resp, err := client.Get(rawURL)
	if err != nil {
		return "", fmt.Errorf("GET checksum: %w", err)
	}
	defer resp.Body.Close()
	if resp.StatusCode != 200 {
		return "", fmt.Errorf("checksum HTTP %d", resp.StatusCode)
	}
	body, err := io.ReadAll(io.LimitReader(resp.Body, 2<<20))
	if err != nil {
		return "", err
	}
	return string(body), nil
}

func downloadFileName(rawURL string) string {
	parsed, err := url.Parse(rawURL)
	if err != nil || parsed.Path == "" {
		return pathpkg.Base(rawURL)
	}
	return pathpkg.Base(parsed.Path)
}

func checksumForFile(content, filename string) (string, error) {
	base := pathpkg.Base(filename)
	for _, line := range strings.Split(content, "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}

		if strings.HasPrefix(line, "SHA256 (") {
			if gotName, gotSum, ok := parseBSDChecksumLine(line); ok && gotName == base {
				return gotSum, nil
			}
			continue
		}

		fields := strings.Fields(line)
		if len(fields) < 2 || !isSHA256Hex(fields[0]) {
			continue
		}
		gotName := strings.TrimPrefix(fields[len(fields)-1], "*")
		gotName = pathpkg.Base(gotName)
		if gotName == base {
			return strings.ToLower(fields[0]), nil
		}
	}
	return "", fmt.Errorf("sha256 entry for %s not found", base)
}

func parseBSDChecksumLine(line string) (filename, checksum string, ok bool) {
	const prefix = "SHA256 ("
	if !strings.HasPrefix(line, prefix) {
		return "", "", false
	}
	closeIdx := strings.Index(line[len(prefix):], ")")
	if closeIdx < 0 {
		return "", "", false
	}
	filename = line[len(prefix) : len(prefix)+closeIdx]
	suffix := strings.TrimSpace(line[len(prefix)+closeIdx+1:])
	if !strings.HasPrefix(suffix, "=") {
		return "", "", false
	}
	checksum = strings.TrimSpace(strings.TrimPrefix(suffix, "="))
	if !isSHA256Hex(checksum) {
		return "", "", false
	}
	return pathpkg.Base(filename), strings.ToLower(checksum), true
}

func isSHA256Hex(s string) bool {
	if len(s) != 64 {
		return false
	}
	for _, r := range s {
		if !((r >= '0' && r <= '9') || (r >= 'a' && r <= 'f') || (r >= 'A' && r <= 'F')) {
			return false
		}
	}
	return true
}

func updateInstallStateKernel(cfg *config.EnvConfig, version, sourceURL, checksumURL string, checksumVerified, allowUnsigned bool) error {
	data, err := os.ReadFile(cfg.InstallState())
	if err != nil {
		return err
	}
	var state map[string]any
	if err := json.Unmarshal(data, &state); err != nil {
		return err
	}
	components, ok := state["components"].(map[string]any)
	if !ok {
		components = map[string]any{}
		state["components"] = components
	}
	components["mihomo"] = map[string]any{
		"version":           version,
		"source_url":        sourceURL,
		"checksum_url":      checksumURL,
		"checksum_verified": checksumVerified,
		"allow_unsigned":    allowUnsigned,
		"path":              cfg.KernelBin(),
		"updated_at":        time.Now().UTC().Format(time.RFC3339),
	}
	out, err := json.MarshalIndent(state, "", "  ")
	if err != nil {
		return err
	}
	return config.AtomicWriteFile(cfg.InstallState(), append(out, '\n'), 0644)
}
