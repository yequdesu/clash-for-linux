package main

import (
	"archive/tar"
	"compress/gzip"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"time"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var selfUpdateWithTUI bool
var selfUpdateSkipTUI bool
var selfUpdateAllowNoChecksum bool
var selfUpdateBaseURL string
var selfUpdateTag string

var selfUninstallYes bool
var selfUninstallDryRun bool
var selfUninstallKeepConfig bool
var selfUninstallRemoveConfig bool
var selfUninstallKeepKernel bool
var selfUninstallRemoveKernel bool

const defaultReleaseBaseURL = "https://github.com/yequdesu/clash-for-linux/releases/latest/download"

var selfCmd = &cobra.Command{
	Use:   "self",
	Short: "Manage clashctl installation",
	Run: func(cmd *cobra.Command, args []string) {
		showHelpAndExit(cmd)
	},
}

var selfUpdateCmd = &cobra.Command{
	Use:   "update",
	Short: "Update clashctl and clash-tui from release artifacts",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if err := runSelfUpdate(); err != nil {
			ilog.Fatal("self update failed: %v", err)
		}
	},
}

var selfUninstallCmd = &cobra.Command{
	Use:   "uninstall",
	Short: "Uninstall clashctl from this machine",
	Run: func(cmd *cobra.Command, args []string) {
		if err := runSelfUninstall(); err != nil {
			ilog.Fatal("self uninstall failed: %v", err)
		}
	},
}

func init() {
	selfUpdateCmd.Flags().BoolVar(&selfUpdateWithTUI, "with-tui", false, "install or update clash-tui even if it is not currently installed")
	selfUpdateCmd.Flags().BoolVar(&selfUpdateSkipTUI, "skip-tui", false, "do not update clash-tui")
	selfUpdateCmd.Flags().BoolVar(&selfUpdateAllowNoChecksum, "allow-no-checksum", false, "allow update when SHA256SUMS cannot be downloaded")
	selfUpdateCmd.Flags().StringVar(&selfUpdateBaseURL, "base-url", "", "release base URL")
	selfUpdateCmd.Flags().StringVar(&selfUpdateTag, "tag", "", "release tag")

	selfUninstallCmd.Flags().BoolVar(&selfUninstallYes, "yes", false, "run without prompts and remove application data")
	selfUninstallCmd.Flags().BoolVar(&selfUninstallDryRun, "dry-run", false, "print actions without changing the system")
	selfUninstallCmd.Flags().BoolVar(&selfUninstallKeepConfig, "keep-config", false, "keep base directory, subscriptions, traffic, and TUI settings")
	selfUninstallCmd.Flags().BoolVar(&selfUninstallRemoveConfig, "remove-config", false, "remove base directory, subscriptions, traffic, and TUI settings")
	selfUninstallCmd.Flags().BoolVar(&selfUninstallKeepKernel, "keep-kernel", false, "keep mihomo and yq binaries in /usr/local/bin")
	selfUninstallCmd.Flags().BoolVar(&selfUninstallRemoveKernel, "remove-kernel", false, "remove mihomo and yq binaries")

	selfCmd.AddCommand(selfUpdateCmd, selfUninstallCmd)
}

func runSelfUpdate() error {
	if runningAsRoot() == false {
		rerunWithSudoOrFatal("self update")
	}
	arch, err := releaseArch()
	if err != nil {
		return err
	}
	base := selfReleaseBaseURL()
	artifact := fmt.Sprintf("clash-for-linux-%s.tar.gz", arch)
	tmpDir, err := os.MkdirTemp("", "clashctl-self-update-*")
	if err != nil {
		return err
	}
	defer os.RemoveAll(tmpDir)

	artifactPath := filepath.Join(tmpDir, artifact)
	if err := downloadReleaseFile(base, artifact, artifactPath); err != nil {
		if usingDefaultLatestRelease() {
			return fmt.Errorf("%w. GitHub latest may not point to release candidates; use --tag vX.Y.Z or --base-url for RC builds", err)
		}
		return err
	}
	checksumsPath := filepath.Join(tmpDir, "SHA256SUMS")
	if err := downloadReleaseFile(base, "SHA256SUMS", checksumsPath); err != nil {
		if !selfUpdateAllowNoChecksum {
			return fmt.Errorf("download checksums: %w", err)
		}
		ilog.Warn("checksum unavailable; continuing because --allow-no-checksum was set")
	} else if err := verifySHA256File(checksumsPath, artifactPath, artifact); err != nil {
		return err
	}

	extractDir := filepath.Join(tmpDir, "extract")
	if err := os.MkdirAll(extractDir, 0o755); err != nil {
		return err
	}
	if err := extractTarGz(artifactPath, extractDir); err != nil {
		return err
	}

	svc := kernel.NewServiceManager(cfg)
	wasRunning := svc.IsRunning()
	if wasRunning {
		ilog.Info("stopping kernel before update...")
		if err := svc.Stop(); err != nil {
			return fmt.Errorf("stop kernel: %w", err)
		}
	}

	if err := installReleaseBinary(filepath.Join(extractDir, "clashctl-linux-"+arch), "/usr/local/bin/clashctl"); err != nil {
		return err
	}
	tuiInstalled := fileExists("/usr/local/bin/clash-tui")
	if selfUpdateWithTUI || (tuiInstalled && !selfUpdateSkipTUI) {
		if err := installReleaseBinary(filepath.Join(extractDir, "clash-tui-linux-"+arch), "/usr/local/bin/clash-tui"); err != nil {
			return err
		}
	}
	if err := updateSelfInstallState(base+"/"+artifact, selfUpdateWithTUI || (tuiInstalled && !selfUpdateSkipTUI)); err != nil {
		ilog.Warn("install state update failed: %v", err)
	}

	if wasRunning {
		ilog.Info("starting kernel after update...")
		if err := svc.Start(); err != nil {
			return fmt.Errorf("restart kernel: %w", err)
		}
	}
	ilog.Ok("clashctl updated from %s", base)
	return nil
}

func runSelfUninstall() error {
	if !selfUninstallDryRun && !selfUninstallYes && !stdinIsTerminal() {
		return fmt.Errorf("interactive uninstall requires a terminal; rerun with --yes or --dry-run")
	}
	if runningAsRoot() == false && !selfUninstallDryRun {
		rerunWithSudoOrFatal("self uninstall")
	}
	opts, err := resolveUninstallOptions()
	if err != nil {
		return err
	}
	return performSelfUninstall(opts)
}

type uninstallOptions struct {
	yes              bool
	dryRun           bool
	keepConfig       bool
	keepKernel       bool
	removeCompletion bool
}

func resolveUninstallOptions() (uninstallOptions, error) {
	opts := uninstallOptions{
		yes:              selfUninstallYes,
		dryRun:           selfUninstallDryRun,
		keepConfig:       selfUninstallKeepConfig,
		keepKernel:       selfUninstallKeepKernel,
		removeCompletion: true,
	}
	if selfUninstallKeepConfig && selfUninstallRemoveConfig {
		return opts, fmt.Errorf("--keep-config and --remove-config conflict")
	}
	if selfUninstallKeepKernel && selfUninstallRemoveKernel {
		return opts, fmt.Errorf("--keep-kernel and --remove-kernel conflict")
	}
	if selfUninstallYes {
		opts.keepConfig = selfUninstallKeepConfig
		opts.keepKernel = selfUninstallKeepKernel
		return opts, nil
	}
	if !confirm(fmt.Sprintf("Uninstall clashctl from %s?", cfg.ClashBaseDir), false) {
		return opts, fmt.Errorf("cancelled")
	}
	if !selfUninstallKeepConfig && !selfUninstallRemoveConfig {
		opts.keepConfig = confirm("Keep configuration, subscriptions, traffic data, and TUI settings?", true)
	} else {
		opts.keepConfig = selfUninstallKeepConfig
	}
	if !selfUninstallKeepKernel && !selfUninstallRemoveKernel {
		opts.keepKernel = confirm("Keep mihomo and yq binaries in /usr/local/bin for future use?", true)
	} else {
		opts.keepKernel = selfUninstallKeepKernel
	}
	opts.removeCompletion = confirm("Remove installed shell completions?", true)
	return opts, nil
}

func performSelfUninstall(opts uninstallOptions) error {
	run := func(desc string, fn func() error) error {
		if opts.dryRun {
			ilog.Info("[dry-run] %s", desc)
			return nil
		}
		if err := fn(); err != nil {
			return fmt.Errorf("%s: %w", desc, err)
		}
		ilog.Ok("%s", desc)
		return nil
	}

	svc := kernel.NewServiceManager(cfg)
	if err := run("stop kernel", func() error { return svc.Stop() }); err != nil {
		ilog.Warn("%v", err)
	}
	if err := run("remove systemd service", removeSystemdService); err != nil {
		ilog.Warn("%v", err)
	}
	if opts.removeCompletion {
		if err := run("remove shell completions", removeAllCompletions); err != nil {
			ilog.Warn("%v", err)
		}
	}
	if err := run("remove shell rc snippets", removeShellSnippets); err != nil {
		ilog.Warn("%v", err)
	}
	if err := run("remove cron entries", removeCronEntries); err != nil {
		ilog.Warn("%v", err)
	}
	if opts.keepKernel {
		if err := run("preserve mihomo and yq binaries", preserveKernelTools); err != nil {
			ilog.Warn("%v", err)
		}
	}
	if !opts.keepConfig {
		if err := run("remove application data", removeApplicationData); err != nil {
			return err
		}
	} else {
		ilog.Info("kept application data: %s", cfg.ClashBaseDir)
	}
	if err := run("remove install markers", removeInstallMarkers); err != nil {
		ilog.Warn("%v", err)
	}
	if !opts.keepKernel {
		if err := run("remove mihomo and yq system binaries", removeKernelTools); err != nil {
			ilog.Warn("%v", err)
		}
	}
	if err := run("remove clashctl binaries", removeClashctlBinaries); err != nil {
		return err
	}
	ilog.Ok("uninstall complete")
	return nil
}

func selfReleaseBaseURL() string {
	if selfUpdateBaseURL != "" {
		return strings.TrimRight(selfUpdateBaseURL, "/")
	}
	if env := os.Getenv("CLASHCTL_RELEASE_BASE_URL"); env != "" {
		return strings.TrimRight(env, "/")
	}
	tag := selfUpdateTag
	if tag == "" {
		tag = os.Getenv("CLASHCTL_RELEASE_TAG")
	}
	if tag != "" {
		return "https://github.com/yequdesu/clash-for-linux/releases/download/" + tag
	}
	return defaultReleaseBaseURL
}

func usingDefaultLatestRelease() bool {
	return selfUpdateBaseURL == "" &&
		os.Getenv("CLASHCTL_RELEASE_BASE_URL") == "" &&
		selfUpdateTag == "" &&
		os.Getenv("CLASHCTL_RELEASE_TAG") == ""
}

func releaseArch() (string, error) {
	switch runtime.GOARCH {
	case "amd64":
		return "amd64", nil
	case "arm64":
		return "arm64", nil
	default:
		return "", fmt.Errorf("no release artifact for %s", runtime.GOARCH)
	}
}

func downloadReleaseFile(base, name, dest string) error {
	urls := releaseDownloadURLs(base, name)
	var errs []string
	client := &http.Client{Timeout: 120 * time.Second}
	for _, rawURL := range urls {
		ilog.Info("downloading %s", rawURL)
		req, _ := http.NewRequest("GET", rawURL, nil)
		req.Header.Set("User-Agent", cfg.ClashSubUA)
		resp, err := client.Do(req)
		if err != nil {
			errs = append(errs, err.Error())
			continue
		}
		if resp.StatusCode != http.StatusOK {
			errs = append(errs, fmt.Sprintf("HTTP %d", resp.StatusCode))
			resp.Body.Close()
			continue
		}
		tmp := dest + ".tmp"
		out, err := os.Create(tmp)
		if err != nil {
			resp.Body.Close()
			return err
		}
		_, copyErr := io.Copy(out, resp.Body)
		closeErr := out.Close()
		resp.Body.Close()
		if copyErr != nil {
			_ = os.Remove(tmp)
			errs = append(errs, copyErr.Error())
			continue
		}
		if closeErr != nil {
			_ = os.Remove(tmp)
			return closeErr
		}
		return os.Rename(tmp, dest)
	}
	return fmt.Errorf("all download attempts failed: %s", strings.Join(errs, "; "))
}

func releaseDownloadURLs(base, name string) []string {
	raw := strings.TrimRight(base, "/") + "/" + name
	if strings.EqualFold(os.Getenv("CLASHCTL_GITHUB_DIRECT"), "false") {
		return mirroredURLs(raw)
	}
	urls := []string{raw}
	return append(urls, mirroredURLs(raw)...)
}

func mirroredURLs(raw string) []string {
	var mirrors []string
	if env := os.Getenv("CLASHCTL_GITHUB_MIRRORS"); env != "" {
		for _, mirror := range strings.Split(env, ",") {
			mirror = strings.TrimSpace(mirror)
			if mirror != "" {
				mirrors = append(mirrors, strings.TrimRight(mirror, "/")+"/"+raw)
			}
		}
	}
	if cfg.URLGhProxy != "" {
		mirrors = append(mirrors, strings.TrimRight(cfg.URLGhProxy, "/")+"/"+raw)
	}
	return mirrors
}

func verifySHA256File(sumPath, artifactPath, artifactName string) error {
	content, err := os.ReadFile(sumPath)
	if err != nil {
		return err
	}
	expected, err := checksumForFile(string(content), artifactName)
	if err != nil {
		return err
	}
	data, err := os.ReadFile(artifactPath)
	if err != nil {
		return err
	}
	sum := sha256.Sum256(data)
	actual := hex.EncodeToString(sum[:])
	if !strings.EqualFold(actual, expected) {
		return fmt.Errorf("sha256 mismatch for %s: got %s want %s", artifactName, actual, expected)
	}
	ilog.Ok("checksum verified: %s", artifactName)
	return nil
}

func extractTarGz(src, dest string) error {
	f, err := os.Open(src)
	if err != nil {
		return err
	}
	defer f.Close()
	gz, err := gzip.NewReader(f)
	if err != nil {
		return err
	}
	defer gz.Close()
	tr := tar.NewReader(gz)
	for {
		h, err := tr.Next()
		if err == io.EOF {
			return nil
		}
		if err != nil {
			return err
		}
		if h.Typeflag != tar.TypeReg {
			continue
		}
		name := filepath.Base(h.Name)
		if name == "." || name == string(filepath.Separator) {
			continue
		}
		outPath := filepath.Join(dest, name)
		out, err := os.OpenFile(outPath, os.O_CREATE|os.O_WRONLY|os.O_TRUNC, 0o755)
		if err != nil {
			return err
		}
		if _, err := io.Copy(out, tr); err != nil {
			out.Close()
			return err
		}
		if err := out.Close(); err != nil {
			return err
		}
	}
}

func installReleaseBinary(src, dest string) error {
	if !fileExists(src) {
		return fmt.Errorf("release binary missing: %s", src)
	}
	tmp := dest + ".new"
	if err := copyFilePath(src, tmp); err != nil {
		return err
	}
	if err := os.Chmod(tmp, 0o755); err != nil {
		return err
	}
	if err := os.Rename(tmp, dest); err != nil {
		_ = os.Remove(tmp)
		return err
	}
	ilog.Ok("updated %s", dest)
	return nil
}

func updateSelfInstallState(sourceURL string, tui bool) error {
	path := cfg.InstallState()
	data, err := os.ReadFile(path)
	if err != nil {
		return err
	}
	var state map[string]any
	if err := json.Unmarshal(data, &state); err != nil {
		return err
	}
	components, _ := state["components"].(map[string]any)
	if components == nil {
		components = map[string]any{}
		state["components"] = components
	}
	components["clashctl"] = map[string]any{
		"installed":  true,
		"version":    appVersion,
		"source_url": sourceURL,
		"path":       "/usr/local/bin/clashctl",
		"updated_at": time.Now().UTC().Format(time.RFC3339),
	}
	if tui {
		components["clash_tui"] = map[string]any{
			"installed":  true,
			"source_url": sourceURL,
			"path":       "/usr/local/bin/clash-tui",
			"updated_at": time.Now().UTC().Format(time.RFC3339),
		}
	}
	out, err := json.MarshalIndent(state, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(path, append(out, '\n'), 0o644)
}

func removeSystemdService() error {
	for _, name := range uniqueStrings([]string{cfg.ServiceName, "clashctl"}) {
		if name == "" {
			continue
		}
		unit := "/etc/systemd/system/" + name + ".service"
		if fileExists(unit) {
			_ = exec.Command("systemctl", "stop", name).Run()
			_ = exec.Command("systemctl", "disable", name).Run()
			if err := os.Remove(unit); err != nil && !os.IsNotExist(err) {
				return err
			}
		}
	}
	_ = exec.Command("systemctl", "daemon-reload").Run()
	return nil
}

func removeAllCompletions() error {
	paths := []string{
		"/etc/bash_completion.d/clashctl",
		"/usr/local/share/zsh/site-functions/_clashctl",
		"/usr/share/fish/vendor_completions.d/clashctl.fish",
		filepath.Join(userHomeDir(), ".local", "share", "bash-completion", "completions", "clashctl"),
		filepath.Join(userHomeDir(), ".zsh", "completions", "_clashctl"),
		filepath.Join(userHomeDir(), ".config", "fish", "completions", "clashctl.fish"),
		filepath.Join(userHomeDir(), ".config", "powershell", "clashctl-completion.ps1"),
	}
	for _, path := range paths {
		if err := os.Remove(path); err != nil && !os.IsNotExist(err) {
			return err
		}
	}
	_ = removeManagedBlock(filepath.Join(userHomeDir(), ".zshrc"), "clashctl completion")
	return nil
}

func removeShellSnippets() error {
	for _, path := range []string{filepath.Join(userHomeDir(), ".zshrc"), filepath.Join(userHomeDir(), ".bashrc")} {
		if err := removeManagedBlock(path, "clashctl"); err != nil {
			return err
		}
		if err := removeManagedBlock(path, "clashctl completion"); err != nil {
			return err
		}
	}
	_ = os.Remove(filepath.Join(userHomeDir(), ".config", "fish", "conf.d", "clashctl.fish"))
	return nil
}

func removeCronEntries() error {
	if _, err := exec.LookPath("crontab"); err != nil {
		return nil
	}
	cmd := exec.Command("crontab", "-l")
	out, err := cmd.Output()
	if err != nil {
		return nil
	}
	var kept []string
	for _, line := range strings.Split(string(out), "\n") {
		if strings.Contains(line, "clashctl sub update") {
			continue
		}
		kept = append(kept, line)
	}
	install := exec.Command("crontab", "-")
	install.Stdin = strings.NewReader(strings.Join(kept, "\n"))
	return install.Run()
}

func preserveKernelTools() error {
	for _, name := range []string{cfg.KernelName, "yq"} {
		src := filepath.Join(cfg.BinDir(), name)
		if fileExists(src) {
			if err := copyFilePath(src, filepath.Join("/usr/local/bin", name)); err != nil {
				return err
			}
		}
	}
	return nil
}

func removeKernelTools() error {
	for _, name := range []string{cfg.KernelName, "yq"} {
		if err := os.Remove(filepath.Join("/usr/local/bin", name)); err != nil && !os.IsNotExist(err) {
			return err
		}
	}
	return nil
}

func removeApplicationData() error {
	for _, path := range []string{
		cfg.ClashBaseDir,
		filepath.Join(userHomeDir(), ".config", "clash-tui"),
	} {
		if err := removeSafePath(path); err != nil {
			return err
		}
	}
	return nil
}

func removeInstallMarkers() error {
	for _, path := range []string{
		"/etc/clashctl",
		filepath.Join(userHomeDir(), ".config", "clashctl"),
	} {
		if err := removeSafePath(path); err != nil {
			return err
		}
	}
	return nil
}

func removeClashctlBinaries() error {
	for _, path := range []string{"/usr/local/bin/clash-tui", "/usr/local/bin/clashctl"} {
		if err := os.Remove(path); err != nil && !os.IsNotExist(err) {
			return err
		}
	}
	return nil
}
