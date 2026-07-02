package main

import (
	"fmt"
	"net"
	"os"
	"strconv"
	"strings"

	"github.com/spf13/cobra"
	"gopkg.in/yaml.v3"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var (
	configMergeAutofix      bool
	configSetHTTPPort       int
	configSetSocksPort      int
	configSetAPISecret      string
	configSetAPIAllowUnsafe bool
)

var configCmd = &cobra.Command{
	Use:   "config",
	Short: "Manage configuration",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		showHelpAndExit(cmd)
	},
}

var configEditCmd = &cobra.Command{
	Use:   "edit",
	Short: "Edit mixin.yaml and merge runtime config",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if err := launchEditor(cfg.MixinPath()); err != nil {
			ilog.Fatal("editor failed: %v", err)
		}
		if err := config.MergeConfig(cfg, false); err != nil {
			ilog.Fatal("merge failed: %v", err)
		}
		ilog.Ok("config updated (restart to apply)")
	},
}

var configViewCmd = &cobra.Command{
	Use:   "view",
	Short: "Print runtime.yaml",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		data, err := readFileString(cfg.RuntimePath())
		if err != nil {
			ilog.Fatal("cannot read runtime config: %v", err)
		}
		fmt.Print(data)
	},
}

var configRawCmd = &cobra.Command{
	Use:   "raw",
	Short: "Print subscription config.yaml",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		data, err := readFileString(cfg.ConfigPath())
		if err != nil {
			ilog.Fatal("cannot read subscription config: %v", err)
		}
		fmt.Print(data)
	},
}

var configMergeCmd = &cobra.Command{
	Use:   "merge",
	Short: "Merge subscription config and mixin.yaml",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		if err := config.MergeConfig(cfg, configMergeAutofix); err != nil {
			ilog.Fatal("merge failed: %v", err)
		}
		ilog.Ok("config merged")
	},
}

var configSetPortCmd = &cobra.Command{
	Use:   "set-port <mixed-port>",
	Short: "Set proxy listening ports without editing YAML",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		mixed, err := parsePortArg(args[0])
		if err != nil {
			ilog.Fatal("%v", err)
		}
		updates := map[string]any{"mixed-port": mixed}
		if configSetHTTPPort > 0 {
			if err := validatePort(configSetHTTPPort); err != nil {
				ilog.Fatal("http port: %v", err)
			}
			updates["port"] = configSetHTTPPort
		}
		if configSetSocksPort > 0 {
			if err := validatePort(configSetSocksPort); err != nil {
				ilog.Fatal("socks port: %v", err)
			}
			updates["socks-port"] = configSetSocksPort
		}
		if err := applyMixinUpdates(updates); err != nil {
			ilog.Fatal("%v", err)
		}
		ilog.Ok("proxy ports updated")
	},
}

var configSetAPICmd = &cobra.Command{
	Use:   "set-api <host:port>",
	Short: "Set external-controller safely",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		controller := strings.TrimSpace(args[0])
		if err := validateController(controller, configSetAPIAllowUnsafe); err != nil {
			ilog.Fatal("%v", err)
		}
		updates := map[string]any{"external-controller": controller}
		if configSetAPISecret != "" {
			updates["secret"] = configSetAPISecret
		}
		if err := applyMixinUpdates(updates); err != nil {
			ilog.Fatal("%v", err)
		}
		ilog.Ok("API controller updated")
	},
}

var configSetDNSModeCmd = &cobra.Command{
	Use:   "set-dns-mode <fake-ip|redir-host|off>",
	Short: "Set DNS enhanced mode without editing YAML",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		mode := strings.ToLower(strings.TrimSpace(args[0]))
		updates := map[string]any{}
		switch mode {
		case "off", "disabled", "disable":
			updates["dns.enable"] = false
		case "fake-ip":
			updates["dns.enable"] = true
			updates["dns.enhanced-mode"] = "fake-ip"
			updates["dns.fake-ip-filter"] = defaultFakeIPFilter()
		case "redir-host":
			updates["dns.enable"] = true
			updates["dns.enhanced-mode"] = "redir-host"
		default:
			ilog.Fatal("unsupported DNS mode %q (use fake-ip, redir-host, or off)", mode)
		}
		if err := applyMixinUpdates(updates); err != nil {
			ilog.Fatal("%v", err)
		}
		ilog.Ok("DNS mode updated")
	},
}

var configSetLANCmd = &cobra.Command{
	Use:   "set-lan <on|off>",
	Short: "Enable or disable LAN proxy access",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		value, err := parseOnOff(args[0])
		if err != nil {
			ilog.Fatal("%v", err)
		}
		if err := applyMixinUpdates(map[string]any{"allow-lan": value}); err != nil {
			ilog.Fatal("%v", err)
		}
		if value {
			ilog.Warn("LAN proxy access enabled; API controller remains separate and should stay loopback-only")
		}
		ilog.Ok("LAN setting updated")
	},
}

var configDoctorCmd = &cobra.Command{
	Use:   "doctor",
	Short: "Diagnose mixin/runtime configuration",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		code := printDoctorResults(collectConfigDoctorResults(cfg))
		if code != 0 {
			exitProcess(code)
		}
	},
}

func init() {
	configMergeCmd.Flags().BoolVar(&configMergeAutofix, "autofix", false, "auto-fix proxy group reference mismatches")
	configSetPortCmd.Flags().IntVar(&configSetHTTPPort, "http", 0, "set HTTP proxy port")
	configSetPortCmd.Flags().IntVar(&configSetSocksPort, "socks", 0, "set SOCKS proxy port")
	configSetAPICmd.Flags().StringVar(&configSetAPISecret, "secret", "", "set API secret together with controller")
	configSetAPICmd.Flags().BoolVar(&configSetAPIAllowUnsafe, "allow-unsafe", false, "allow non-loopback API controller")
	configCmd.AddCommand(
		configEditCmd,
		configViewCmd,
		configRawCmd,
		configMergeCmd,
		configSetPortCmd,
		configSetAPICmd,
		configSetDNSModeCmd,
		configSetLANCmd,
		configDoctorCmd,
	)
}

func applyMixinUpdates(updates map[string]any) error {
	snapshots, err := snapshotFiles(cfg.MixinPath(), cfg.RuntimePath())
	if err != nil {
		return fmt.Errorf("snapshot config: %w", err)
	}
	if err := config.UpdateYAML(cfg.MixinPath(), updates); err != nil {
		return fmt.Errorf("update mixin: %w", err)
	}
	if err := mergeMixinConfig(cfg, false); err != nil {
		restoreSnapshots(snapshots)
		return fmt.Errorf("merge failed after mixin update, restored previous config: %w", err)
	}
	return nil
}

var mergeMixinConfig = config.MergeConfig

func parsePortArg(raw string) (int, error) {
	port, err := strconv.Atoi(strings.TrimSpace(raw))
	if err != nil {
		return 0, fmt.Errorf("invalid port %q", raw)
	}
	return port, validatePort(port)
}

func validatePort(port int) error {
	if port < 1 || port > 65535 {
		return fmt.Errorf("port must be 1-65535")
	}
	return nil
}

func validateController(controller string, allowUnsafe bool) error {
	host, port, err := net.SplitHostPort(controller)
	if err != nil {
		return fmt.Errorf("external-controller must be host:port: %w", err)
	}
	p, err := strconv.Atoi(port)
	if err != nil {
		return fmt.Errorf("invalid API port %q", port)
	}
	if err := validatePort(p); err != nil {
		return err
	}
	if allowUnsafe {
		return nil
	}
	host = strings.Trim(host, "[]")
	if strings.EqualFold(host, "localhost") {
		return nil
	}
	ip := net.ParseIP(host)
	if ip == nil || !ip.IsLoopback() {
		return fmt.Errorf("refusing unsafe API listen address %q; use --allow-unsafe only if you understand the risk", controller)
	}
	return nil
}

func parseOnOff(raw string) (bool, error) {
	switch strings.ToLower(strings.TrimSpace(raw)) {
	case "on", "true", "yes", "1", "enable", "enabled":
		return true, nil
	case "off", "false", "no", "0", "disable", "disabled":
		return false, nil
	default:
		return false, fmt.Errorf("expected on or off")
	}
}

func defaultFakeIPFilter() []string {
	return []string{
		"localhost",
		"*.local",
		"*.lan",
		"*.home.arpa",
		"router.asus.com",
		"routerlogin.net",
		"*.msftconnecttest.com",
		"*.msftncsi.com",
		"time.*.com",
		"time.*.gov",
		"time.*.edu.cn",
		"time.*.apple.com",
	}
}

type mixinDoctorConfig struct {
	MixedPort          any    `yaml:"mixed-port"`
	Port               any    `yaml:"port"`
	SocksPort          any    `yaml:"socks-port"`
	AllowLAN           bool   `yaml:"allow-lan"`
	ExternalController string `yaml:"external-controller"`
	Secret             string `yaml:"secret"`
	DNS                struct {
		Enable       bool     `yaml:"enable"`
		EnhancedMode string   `yaml:"enhanced-mode"`
		FakeIPFilter []string `yaml:"fake-ip-filter"`
	} `yaml:"dns"`
	Tun struct {
		Enable bool `yaml:"enable"`
	} `yaml:"tun"`
}

func collectConfigDoctorResults(cfg *config.EnvConfig) []doctorResult {
	var results []doctorResult
	add := func(level doctorLevel, subject, message string) {
		results = append(results, doctorResult{level: level, subject: subject, message: message})
	}

	data, err := os.ReadFile(cfg.MixinPath())
	if err != nil {
		add(doctorFatal, "mixin", "missing: "+cfg.MixinPath())
		return results
	}
	var mixin mixinDoctorConfig
	if err := yaml.Unmarshal(data, &mixin); err != nil {
		add(doctorFatal, "mixin", err.Error())
		return results
	}
	add(doctorOK, "mixin", cfg.MixinPath())

	checkPort := func(subject string, value any) string {
		port := configScalarString(value)
		if port == "" {
			add(doctorWarn, subject, "not set")
			return ""
		}
		p, err := strconv.Atoi(port)
		if err != nil || validatePort(p) != nil {
			add(doctorFatal, subject, "invalid port: "+port)
			return ""
		}
		add(doctorOK, subject, port)
		return port
	}
	ports := map[string]string{}
	for _, p := range []string{
		checkPort("mixed-port", mixin.MixedPort),
		checkPort("http port", mixin.Port),
		checkPort("socks port", mixin.SocksPort),
	} {
		if p == "" {
			continue
		}
		if prev := ports[p]; prev != "" {
			add(doctorWarn, "ports", fmt.Sprintf("duplicate port %s also used by %s", p, prev))
		}
		ports[p] = "proxy"
	}

	if mixin.ExternalController == "" {
		add(doctorWarn, "api listen", "external-controller not set")
	} else if !isSafeController(mixin.ExternalController) {
		add(doctorFatal, "api listen", "unsafe external-controller: "+mixin.ExternalController)
	} else {
		add(doctorOK, "api listen", mixin.ExternalController)
	}
	if mixin.Secret == "" {
		add(doctorWarn, "api secret", "empty in mixin; install should generate one before runtime use")
	} else {
		add(doctorOK, "api secret", "set")
	}

	if !mixin.DNS.Enable {
		add(doctorWarn, "dns", "disabled")
	} else {
		switch mixin.DNS.EnhancedMode {
		case "fake-ip":
			if hasCommonFakeIPFilters(mixin.DNS.FakeIPFilter) {
				add(doctorOK, "dns", "fake-ip with LAN filters")
			} else {
				add(doctorWarn, "dns", "fake-ip without common LAN filters; run 'clashctl config set-dns-mode fake-ip'")
			}
		case "redir-host":
			add(doctorOK, "dns", "redir-host")
		default:
			add(doctorWarn, "dns", "unknown enhanced-mode: "+mixin.DNS.EnhancedMode)
		}
	}

	if mixin.AllowLAN {
		add(doctorWarn, "allow-lan", "enabled")
	} else {
		add(doctorOK, "allow-lan", "disabled")
	}
	if mixin.Tun.Enable {
		if _, err := os.Stat("/dev/net/tun"); err != nil {
			add(doctorWarn, "tun", "/dev/net/tun unavailable")
		} else {
			add(doctorOK, "tun", "enabled")
		}
	} else {
		add(doctorOK, "tun", "disabled")
	}

	if _, err := os.Stat(cfg.RuntimePath()); err == nil {
		if err := validateConfigFile(cfg, cfg.RuntimePath()); err != nil {
			add(doctorFatal, "runtime", err.Error())
		} else {
			add(doctorOK, "runtime", "runtime.yaml is valid")
		}
	} else {
		add(doctorWarn, "runtime", "missing; run 'clashctl config merge'")
	}

	return results
}

func configScalarString(v any) string {
	switch t := v.(type) {
	case nil:
		return ""
	case string:
		return strings.TrimSpace(t)
	case int:
		return strconv.Itoa(t)
	case int64:
		return strconv.FormatInt(t, 10)
	case uint64:
		return strconv.FormatUint(t, 10)
	case float64:
		if t == float64(int64(t)) {
			return strconv.FormatInt(int64(t), 10)
		}
		return strconv.FormatFloat(t, 'f', -1, 64)
	default:
		return fmt.Sprint(t)
	}
}

func hasCommonFakeIPFilters(filters []string) bool {
	haveLocal := false
	haveLAN := false
	for _, filter := range filters {
		switch strings.ToLower(strings.TrimSpace(filter)) {
		case "localhost", "*.local":
			haveLocal = true
		case "*.lan", "*.home.arpa":
			haveLAN = true
		}
	}
	return haveLocal && haveLAN
}
