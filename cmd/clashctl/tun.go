package main

import (
	"encoding/json"
	"fmt"
	"os"
	"os/exec"
	"strings"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	"github.com/yequdesu/linux-cli-tui-clash/internal/kernel"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var tunAllowSSHTunRisk bool

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
			if err := ensureSafeKernelStartFromSSH(cfg, tunAllowSSHTunRisk, true); err != nil {
				ilog.Fatal("%v", err)
			}
			if err := changeTunMode(cfg, true); err != nil {
				_ = cleanupSSHTunBypass(cfg)
				ilog.Fatal("Tun mode not enabled: %v", err)
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
			if err := changeTunMode(cfg, false); err != nil {
				ilog.Fatal("Tun mode not disabled: %v", err)
			}
			if err := cleanupSSHTunBypass(cfg); err != nil {
				ilog.Warn("SSH TUN bypass cleanup failed: %v", err)
			}
			ilog.Ok("Tun mode disabled")
		default:
			ilog.Fatal("usage: clashctl tun [on|off]")
		}
	},
}

func init() {
	tunCmd.Flags().BoolVar(&tunAllowSSHTunRisk, "allow-ssh-tun-risk", false, "allow enabling TUN auto-route from an SSH session")
}

type tunService interface {
	IsRunning() bool
	Stop() error
	Start() error
}

var newTunService = func(cfg *config.EnvConfig) tunService {
	return kernel.NewServiceManager(cfg)
}

var mergeTunConfig = config.MergeConfig

type fileSnapshot struct {
	path   string
	data   []byte
	exists bool
}

func changeTunMode(cfg *config.EnvConfig, enable bool) error {
	if enable {
		if err := checkTunPrerequisites(); err != nil {
			return err
		}
	}

	snapshots, err := snapshotFiles(cfg.MixinPath(), cfg.RuntimePath())
	if err != nil {
		return err
	}

	svc := newTunService(cfg)
	wasRunning := svc.IsRunning()

	if err := setTunEnabled(cfg, enable); err != nil {
		return fmt.Errorf("set tun: %w", err)
	}
	if err := mergeTunConfig(cfg, false); err != nil {
		restoreSnapshots(snapshots)
		return fmt.Errorf("merge: %w", err)
	}

	shouldStart := wasRunning || enable
	if !shouldStart {
		return nil
	}

	if err := svc.Stop(); err != nil {
		restoreSnapshots(snapshots)
		return fmt.Errorf("stop current kernel: %w", err)
	}
	if enable {
		ensureSetcap(cfg.KernelBin())
	}
	if err := svc.Start(); err != nil {
		restoreSnapshots(snapshots)
		if wasRunning {
			_ = svc.Start()
		}
		if strings.Contains(err.Error(), "exit status 5") || strings.Contains(err.Error(), "died") {
			return fmt.Errorf("kernel may lack capability or config is invalid; try: sudo setcap cap_net_admin,cap_net_raw+ep %s", cfg.KernelBin())
		}
		return fmt.Errorf("restart: %w", err)
	}

	if enable {
		if _, err := verifyTunDevice(); err != nil {
			restoreSnapshots(snapshots)
			if wasRunning {
				_ = svc.Stop()
				_ = svc.Start()
			} else {
				_ = svc.Stop()
			}
			return fmt.Errorf("TUN device not detected after start: %w", err)
		}
	}

	return nil
}

func checkTunPrerequisites() error {
	if _, err := os.Stat("/dev/net/tun"); err != nil {
		return fmt.Errorf("/dev/net/tun not available: %w", err)
	}
	if _, err := exec.LookPath("ip"); err != nil {
		return fmt.Errorf("'ip' command not found")
	}
	return nil
}

func setTunEnabled(cfg *config.EnvConfig, enable bool) error {
	return config.UpdateYAML(cfg.MixinPath(), map[string]any{"tun.enable": enable})
}

func snapshotFiles(paths ...string) ([]fileSnapshot, error) {
	snapshots := make([]fileSnapshot, 0, len(paths))
	for _, path := range paths {
		data, err := os.ReadFile(path)
		if err != nil {
			if os.IsNotExist(err) {
				snapshots = append(snapshots, fileSnapshot{path: path})
				continue
			}
			return nil, err
		}
		snapshots = append(snapshots, fileSnapshot{path: path, data: data, exists: true})
	}
	return snapshots, nil
}

func restoreSnapshots(snapshots []fileSnapshot) {
	for _, s := range snapshots {
		if s.exists {
			_ = config.AtomicWriteFile(s.path, s.data, 0644)
		} else {
			_ = os.Remove(s.path)
		}
	}
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
	info, err := config.ReadRuntimeInfo(cfg.RuntimePath())
	if err != nil {
		ilog.Info("Tun status: unknown")
		return
	}
	if info.TunEnabled {
		ilog.Ok("Tun status: enabled")
	} else {
		ilog.Info("Tun status: disabled")
	}
}

func verifyTunDevice() (string, error) {
	if out, err := runIPCommand("tuntap", "show"); err == nil {
		if dev := parseIPTunTapDevice(out); dev != "" {
			return dev, nil
		}
	}
	if out, err := runIPCommand("-d", "-j", "link", "show"); err == nil {
		if dev := parseIPLinkJSONDevice(out); dev != "" {
			return dev, nil
		}
	}
	out, err := runIPCommand("link", "show")
	if err != nil {
		return "", err
	}
	if dev := parseIPLinkTextDevice(out); dev != "" {
		return dev, nil
	}
	return "", fmt.Errorf("no TUN device found")
}

var runIPCommand = func(args ...string) ([]byte, error) {
	return exec.Command("ip", args...).Output()
}

func parseIPTunTapDevice(out []byte) string {
	for _, line := range strings.Split(string(out), "\n") {
		fields := strings.Fields(strings.TrimSpace(line))
		if len(fields) < 2 {
			continue
		}
		name := strings.TrimSuffix(fields[0], ":")
		for _, field := range fields[1:] {
			if field == "tun" {
				return name
			}
		}
	}
	return ""
}

type ipLinkJSONEntry struct {
	IfName   string   `json:"ifname"`
	Flags    []string `json:"flags"`
	LinkInfo struct {
		InfoKind string `json:"info_kind"`
	} `json:"linkinfo"`
}

func parseIPLinkJSONDevice(out []byte) string {
	var links []ipLinkJSONEntry
	if err := json.Unmarshal(out, &links); err != nil {
		return ""
	}
	for _, link := range links {
		if link.IfName == "" {
			continue
		}
		if link.LinkInfo.InfoKind == "tun" {
			return link.IfName
		}
		if likelyTunName(link.IfName) && hasAllFlags(link.Flags, "POINTOPOINT", "NOARP") {
			return link.IfName
		}
	}
	return ""
}

func parseIPLinkTextDevice(out []byte) string {
	for _, line := range strings.Split(string(out), "\n") {
		line = strings.TrimSpace(line)
		if line == "" || !strings.Contains(line, ":") {
			continue
		}
		fields := strings.Fields(line)
		if len(fields) < 2 {
			continue
		}
		name := strings.TrimSuffix(fields[1], ":")
		if likelyTunName(name) && strings.Contains(line, "POINTOPOINT") && strings.Contains(line, "NOARP") {
			return strings.TrimSuffix(strings.SplitN(name, "@", 2)[0], ":")
		}
	}
	return ""
}

func likelyTunName(name string) bool {
	name = strings.ToLower(strings.TrimSuffix(strings.SplitN(name, "@", 2)[0], ":"))
	return strings.Contains(name, "tun")
}

func hasAllFlags(flags []string, want ...string) bool {
	have := map[string]bool{}
	for _, flag := range flags {
		have[strings.ToUpper(flag)] = true
	}
	for _, flag := range want {
		if !have[strings.ToUpper(flag)] {
			return false
		}
	}
	return true
}
