package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"strings"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
)

type doctorLevel int

const (
	doctorOK doctorLevel = iota
	doctorWarn
	doctorFatal
)

type doctorResult struct {
	level   doctorLevel
	subject string
	message string
}

type installStateFile struct {
	SchemaVersion int                         `json:"schema_version"`
	BaseDir       string                      `json:"base_dir"`
	ServiceName   string                      `json:"service_name"`
	KernelName    string                      `json:"kernel_name"`
	Components    map[string]installComponent `json:"components"`
}

type installComponent struct {
	Version   string `json:"version"`
	SourceURL string `json:"source_url"`
	Path      string `json:"path"`
}

var doctorCmd = &cobra.Command{
	Use:   "doctor",
	Short: "Diagnose installation, config, service, and security issues",
	Run: func(cmd *cobra.Command, args []string) {
		code := runDoctor()
		if code != 0 {
			exitProcess(code)
		}
	},
}

func runDoctor() int {
	return printDoctorResults(collectDoctorResults(cfg))
}

func printDoctorResults(results []doctorResult) int {
	exitCode := 0
	for _, r := range results {
		switch r.level {
		case doctorFatal:
			exitCode = 2
			fmt.Printf("[x] %-18s %s\n", r.subject, r.message)
		case doctorWarn:
			if exitCode < 1 {
				exitCode = 1
			}
			fmt.Printf("[!] %-18s %s\n", r.subject, r.message)
		default:
			fmt.Printf("[+] %-18s %s\n", r.subject, r.message)
		}
	}
	return exitCode
}

func collectDoctorResults(cfg *config.EnvConfig) []doctorResult {
	var results []doctorResult
	add := func(level doctorLevel, subject, message string) {
		results = append(results, doctorResult{level: level, subject: subject, message: message})
	}

	checkDir := func(subject, path string, required bool) {
		if fi, err := os.Stat(path); err == nil && fi.IsDir() {
			add(doctorOK, subject, path)
		} else if required {
			add(doctorFatal, subject, "missing: "+path)
		} else {
			add(doctorWarn, subject, "missing: "+path)
		}
	}
	checkFile := func(subject, path string, required bool) {
		if fi, err := os.Stat(path); err == nil && !fi.IsDir() {
			add(doctorOK, subject, path)
		} else if required {
			add(doctorFatal, subject, "missing: "+path)
		} else {
			add(doctorWarn, subject, "missing: "+path)
		}
	}
	checkExecutable := func(subject, path string, required bool) {
		if fi, err := os.Stat(path); err == nil && !fi.IsDir() {
			if fi.Mode()&0111 == 0 {
				if required {
					add(doctorFatal, subject, "not executable: "+path)
				} else {
					add(doctorWarn, subject, "not executable: "+path)
				}
				return
			}
			add(doctorOK, subject, path)
		} else if required {
			add(doctorFatal, subject, "missing: "+path)
		} else {
			add(doctorWarn, subject, "missing: "+path)
		}
	}

	checkDir("base dir", cfg.ClashBaseDir, true)
	checkDir("resources", cfg.ResourcesDir(), true)
	checkDir("profiles dir", cfg.ProfilesDir(), false)
	checkDir("logs dir", cfg.LogDir(), false)
	if err := checkWritableDir(cfg.ClashBaseDir); err != nil {
		add(doctorFatal, "base access", err.Error())
	} else {
		add(doctorOK, "base access", "writable")
	}
	if message, err := validateInstallState(cfg.InstallState(), cfg); err != nil {
		if errors.Is(err, os.ErrNotExist) {
			add(doctorWarn, "install state", "missing: "+cfg.InstallState())
		} else {
			add(doctorFatal, "install state", err.Error())
		}
	} else {
		add(doctorOK, "install state", message)
	}
	checkExecutable("kernel", cfg.KernelBin(), true)
	checkExecutable("yq", cfg.YQBin(), true)
	checkExecutable("subconverter", cfg.SubconverterBin(), false)
	checkFile("runtime", cfg.RuntimePath(), false)

	profiles, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		add(doctorFatal, "profiles", err.Error())
	} else {
		add(doctorOK, "profiles", cfg.ProfilesMeta())
		checkCronAutoUpdate(add, len(profiles.Profiles))
	}

	if _, err := os.Stat(cfg.RuntimePath()); err == nil {
		if err := validateConfigFile(cfg, cfg.RuntimePath()); err != nil {
			add(doctorFatal, "config test", err.Error())
		} else {
			add(doctorOK, "config test", "runtime.yaml is valid")
		}

		runtimeInfo, err := config.ReadRuntimeInfo(cfg.RuntimePath())
		if err != nil {
			add(doctorFatal, "runtime parse", err.Error())
			return results
		}

		controller := runtimeInfo.ExternalController
		if controller == "" {
			add(doctorWarn, "api listen", "external-controller not found")
		} else if !isSafeController(controller) {
			add(doctorFatal, "api listen", "unsafe external-controller: "+controller)
		} else {
			add(doctorOK, "api listen", controller)
		}

		if runtimeInfo.Secret == "" {
			add(doctorFatal, "api secret", "empty secret")
		} else {
			add(doctorOK, "api secret", "set")
		}

		if runtimeInfo.APIPort != "" && tcpOpen(runtimeInfo.APIAddress("9090")) {
			api := kernel.NewClient(runtimeInfo.APIBaseURL("9090"), runtimeInfo.Secret)
			if ver, err := api.GetVersion(); err != nil {
				add(doctorFatal, "api auth", err.Error())
			} else {
				add(doctorOK, "api auth", "version "+ver)
			}
		}

		if runtimeInfo.ProxyPort != "" {
			if portOpen(runtimeInfo.ProxyPort) {
				add(doctorOK, "proxy port", runtimeInfo.ProxyPort+" listening")
			} else {
				add(doctorWarn, "proxy port", runtimeInfo.ProxyPort+" not listening")
			}
		}

		checkRuntimeDNS(add, runtimeInfo)
		checkRuntimeTun(add, runtimeInfo)
	}

	svc := kernel.NewServiceManager(cfg)
	if svc.InitType() == "systemd" {
		if serviceUnitExists(cfg.ServiceName) {
			add(doctorOK, "systemd unit", cfg.ServiceName+".service")
		} else {
			add(doctorWarn, "systemd unit", cfg.ServiceName+".service not installed")
		}
	}

	for _, name := range []string{"Country.mmdb", "geosite.dat", "geoip.dat"} {
		path := filepath.Join(cfg.ResourcesDir(), name)
		if _, err := os.Stat(path); err == nil {
			add(doctorOK, "geodata", name)
		} else {
			add(doctorWarn, "geodata", name+" missing")
		}
	}

	return results
}

func checkWritableDir(path string) error {
	f, err := os.CreateTemp(path, ".clashctl-doctor-*")
	if err != nil {
		return fmt.Errorf("not writable: %s: %w", path, err)
	}
	name := f.Name()
	_ = f.Close()
	_ = os.Remove(name)
	return nil
}

func checkCronAutoUpdate(add func(doctorLevel, string, string), profileCount int) {
	if profileCount == 0 {
		add(doctorOK, "cron update", "no subscriptions")
		return
	}
	if _, err := exec.LookPath("crontab"); err != nil {
		add(doctorWarn, "cron update", "crontab not installed")
		return
	}
	out, err := exec.Command("crontab", "-l").Output()
	if err != nil {
		add(doctorWarn, "cron update", "not enabled")
		return
	}
	if hasCronAutoUpdate(string(out)) {
		add(doctorOK, "cron update", "enabled")
	} else {
		add(doctorWarn, "cron update", "not enabled; run 'clashctl sub update --auto'")
	}
}

func hasCronAutoUpdate(crontab string) bool {
	for _, line := range strings.Split(crontab, "\n") {
		line = strings.TrimSpace(line)
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		if strings.Contains(line, "clashctl sub update") && strings.Contains(line, "--cron") {
			return true
		}
	}
	return false
}

func checkRuntimeDNS(add func(doctorLevel, string, string), info config.RuntimeInfo) {
	if !info.DNSEnabled {
		add(doctorWarn, "dns", "disabled")
		return
	}
	switch info.DNSEnhancedMode {
	case "fake-ip":
		if hasCommonFakeIPFilters(info.FakeIPFilter) {
			add(doctorOK, "dns", "fake-ip with LAN filters")
		} else {
			add(doctorWarn, "dns", "fake-ip without common LAN filters")
		}
	case "redir-host":
		add(doctorOK, "dns", "redir-host")
	default:
		add(doctorWarn, "dns", "unknown enhanced-mode: "+info.DNSEnhancedMode)
	}
}

func checkRuntimeTun(add func(doctorLevel, string, string), info config.RuntimeInfo) {
	if !info.TunEnabled {
		add(doctorOK, "tun", "disabled")
		return
	}
	if _, err := os.Stat("/dev/net/tun"); err != nil {
		add(doctorFatal, "tun", "/dev/net/tun unavailable")
		return
	}
	if _, err := exec.LookPath("ip"); err != nil {
		add(doctorWarn, "tun", "ip command not found")
		return
	}
	add(doctorOK, "tun", "enabled")
}

func validateInstallState(path string, cfg *config.EnvConfig) (string, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return "", err
	}

	var state installStateFile
	if err := json.Unmarshal(data, &state); err != nil {
		return "", fmt.Errorf("invalid JSON in %s: %w", path, err)
	}
	if state.SchemaVersion != 1 {
		return "", fmt.Errorf("unsupported schema_version %d in %s", state.SchemaVersion, path)
	}
	if filepath.Clean(state.BaseDir) != filepath.Clean(cfg.ClashBaseDir) {
		return "", fmt.Errorf("base_dir mismatch: state=%s current=%s", state.BaseDir, cfg.ClashBaseDir)
	}
	if state.ServiceName != cfg.ServiceName {
		return "", fmt.Errorf("service_name mismatch: state=%s current=%s", state.ServiceName, cfg.ServiceName)
	}
	if state.KernelName != cfg.KernelName {
		return "", fmt.Errorf("kernel_name mismatch: state=%s current=%s", state.KernelName, cfg.KernelName)
	}
	for _, name := range []string{"mihomo", "yq", "geodata", "clashctl"} {
		component, ok := state.Components[name]
		if !ok {
			return "", fmt.Errorf("component %s missing in %s", name, path)
		}
		if strings.TrimSpace(component.Path) == "" {
			return "", fmt.Errorf("component %s has empty path in %s", name, path)
		}
	}
	for _, name := range []string{"mihomo", "yq", "geodata"} {
		component := state.Components[name]
		if strings.TrimSpace(component.Version) == "" {
			return "", fmt.Errorf("component %s has empty version in %s", name, path)
		}
		if strings.TrimSpace(component.SourceURL) == "" {
			return "", fmt.Errorf("component %s has empty source_url in %s", name, path)
		}
	}
	return fmt.Sprintf("schema %d, %d components", state.SchemaVersion, len(state.Components)), nil
}

func serviceUnitExists(serviceName string) bool {
	for _, dir := range []string{"/etc/systemd/system", "/lib/systemd/system", "/usr/lib/systemd/system"} {
		if _, err := os.Stat(filepath.Join(dir, serviceName+".service")); err == nil {
			return true
		}
	}
	return false
}

func isSafeController(controller string) bool {
	host := controller
	if h, _, err := net.SplitHostPort(controller); err == nil {
		host = h
	} else if idx := strings.LastIndex(controller, ":"); idx >= 0 {
		host = controller[:idx]
	}
	host = strings.Trim(host, "[]")
	if host == "" {
		return false
	}
	if strings.EqualFold(host, "localhost") {
		return true
	}
	ip := net.ParseIP(host)
	return ip != nil && ip.IsLoopback()
}
