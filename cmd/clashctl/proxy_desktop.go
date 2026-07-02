package main

import (
	"errors"
	"fmt"
	"os/exec"
	"strconv"
	"strings"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

type desktopProxyCommand struct {
	name string
	args []string
}

func handleDesktopProxy(args []string) {
	if len(args) != 1 {
		ilog.Fatal("usage: clashctl proxy desktop [on|off|status]")
	}
	switch args[0] {
	case "on":
		if err := setDesktopProxy(cfg); err != nil {
			ilog.Fatal("%v", err)
		}
		ilog.Ok("desktop proxy enabled")
	case "off":
		if err := unsetDesktopProxy(); err != nil {
			ilog.Fatal("%v", err)
		}
		ilog.Ok("desktop proxy disabled")
	case "status":
		showDesktopProxyStatus()
	default:
		ilog.Fatal("usage: clashctl proxy desktop [on|off|status]")
	}
}

func setDesktopProxy(cfg *config.EnvConfig) error {
	ports := desktopProxySettingsFromConfig(cfg)
	var applied []string
	var errs []string

	if commandAvailable("gsettings") {
		if err := runDesktopCommands(gnomeProxyOnCommands(ports)); err != nil {
			errs = append(errs, "GNOME: "+err.Error())
		} else {
			applied = append(applied, "GNOME")
		}
	}
	if kdeConfigTool() != "" {
		if err := runDesktopCommands(kdeProxyOnCommands(kdeConfigTool(), ports)); err != nil {
			errs = append(errs, "KDE: "+err.Error())
		} else {
			applied = append(applied, "KDE")
		}
	}
	if len(applied) > 0 {
		ilog.Info("applied: %s", strings.Join(applied, ", "))
		return nil
	}
	if len(errs) > 0 {
		return errors.New(strings.Join(errs, "; "))
	}
	return fmt.Errorf("no supported desktop proxy tool found (need gsettings for GNOME or kwriteconfig6/kwriteconfig5 for KDE)")
}

func unsetDesktopProxy() error {
	var applied []string
	var errs []string

	if commandAvailable("gsettings") {
		if err := runDesktopCommands(gnomeProxyOffCommands()); err != nil {
			errs = append(errs, "GNOME: "+err.Error())
		} else {
			applied = append(applied, "GNOME")
		}
	}
	if kdeConfigTool() != "" {
		if err := runDesktopCommands(kdeProxyOffCommands(kdeConfigTool())); err != nil {
			errs = append(errs, "KDE: "+err.Error())
		} else {
			applied = append(applied, "KDE")
		}
	}
	if len(applied) > 0 {
		ilog.Info("applied: %s", strings.Join(applied, ", "))
		return nil
	}
	if len(errs) > 0 {
		return errors.New(strings.Join(errs, "; "))
	}
	return fmt.Errorf("no supported desktop proxy tool found (need gsettings for GNOME or kwriteconfig6/kwriteconfig5 for KDE)")
}

func showDesktopProxyStatus() {
	found := false
	if commandAvailable("gsettings") {
		found = true
		mode := commandOutput("gsettings", "get", "org.gnome.system.proxy", "mode")
		httpHost := commandOutput("gsettings", "get", "org.gnome.system.proxy.http", "host")
		httpPort := commandOutput("gsettings", "get", "org.gnome.system.proxy.http", "port")
		ilog.Info("GNOME proxy: mode=%s http=%s:%s", trimDesktopValue(mode), trimDesktopValue(httpHost), trimDesktopValue(httpPort))
	}
	if tool := kdeConfigTool(); tool != "" {
		found = true
		ilog.Info("KDE proxy: configurable via %s (kioslaverc)", tool)
	}
	if !found {
		ilog.Warn("desktop proxy: unsupported desktop or missing tools")
	}
}

type desktopProxySettings struct {
	host      string
	httpPort  string
	socksPort string
	noProxy   string
}

func desktopProxySettingsFromConfig(cfg *config.EnvConfig) desktopProxySettings {
	info := config.LoadRuntimeInfo(cfg)
	httpPort := info.ProxyPort
	if httpPort == "" {
		httpPort = "7890"
	}
	socksPort := info.SocksPort
	if socksPort == "" {
		socksPort = httpPort
	}
	return desktopProxySettings{
		host:      "127.0.0.1",
		httpPort:  httpPort,
		socksPort: socksPort,
		noProxy:   "localhost,127.0.0.1,::1",
	}
}

func gnomeProxyOnCommands(p desktopProxySettings) []desktopProxyCommand {
	return []desktopProxyCommand{
		{name: "gsettings", args: []string{"set", "org.gnome.system.proxy", "mode", "manual"}},
		{name: "gsettings", args: []string{"set", "org.gnome.system.proxy", "ignore-hosts", "['localhost', '127.0.0.1', '::1']"}},
		{name: "gsettings", args: []string{"set", "org.gnome.system.proxy.http", "host", p.host}},
		{name: "gsettings", args: []string{"set", "org.gnome.system.proxy.http", "port", p.httpPort}},
		{name: "gsettings", args: []string{"set", "org.gnome.system.proxy.https", "host", p.host}},
		{name: "gsettings", args: []string{"set", "org.gnome.system.proxy.https", "port", p.httpPort}},
		{name: "gsettings", args: []string{"set", "org.gnome.system.proxy.ftp", "host", p.host}},
		{name: "gsettings", args: []string{"set", "org.gnome.system.proxy.ftp", "port", p.httpPort}},
		{name: "gsettings", args: []string{"set", "org.gnome.system.proxy.socks", "host", p.host}},
		{name: "gsettings", args: []string{"set", "org.gnome.system.proxy.socks", "port", p.socksPort}},
	}
}

func gnomeProxyOffCommands() []desktopProxyCommand {
	return []desktopProxyCommand{
		{name: "gsettings", args: []string{"set", "org.gnome.system.proxy", "mode", "none"}},
	}
}

func kdeProxyOnCommands(tool string, p desktopProxySettings) []desktopProxyCommand {
	return []desktopProxyCommand{
		{name: tool, args: []string{"--file", "kioslaverc", "--group", "Proxy Settings", "--key", "ProxyType", "1"}},
		{name: tool, args: []string{"--file", "kioslaverc", "--group", "Proxy Settings", "--key", "httpProxy", fmt.Sprintf("http://%s %s", p.host, p.httpPort)}},
		{name: tool, args: []string{"--file", "kioslaverc", "--group", "Proxy Settings", "--key", "httpsProxy", fmt.Sprintf("http://%s %s", p.host, p.httpPort)}},
		{name: tool, args: []string{"--file", "kioslaverc", "--group", "Proxy Settings", "--key", "ftpProxy", fmt.Sprintf("http://%s %s", p.host, p.httpPort)}},
		{name: tool, args: []string{"--file", "kioslaverc", "--group", "Proxy Settings", "--key", "socksProxy", fmt.Sprintf("socks://%s %s", p.host, p.socksPort)}},
		{name: tool, args: []string{"--file", "kioslaverc", "--group", "Proxy Settings", "--key", "NoProxyFor", p.noProxy}},
	}
}

func kdeProxyOffCommands(tool string) []desktopProxyCommand {
	return []desktopProxyCommand{
		{name: tool, args: []string{"--file", "kioslaverc", "--group", "Proxy Settings", "--key", "ProxyType", "0"}},
	}
}

func runDesktopCommands(commands []desktopProxyCommand) error {
	for _, command := range commands {
		if err := exec.Command(command.name, command.args...).Run(); err != nil {
			return fmt.Errorf("%s %s: %w", command.name, strings.Join(command.args, " "), err)
		}
	}
	return nil
}

func commandAvailable(name string) bool {
	_, err := exec.LookPath(name)
	return err == nil
}

func kdeConfigTool() string {
	for _, tool := range []string{"kwriteconfig6", "kwriteconfig5"} {
		if commandAvailable(tool) {
			return tool
		}
	}
	return ""
}

func commandOutput(name string, args ...string) string {
	out, err := exec.Command(name, args...).Output()
	if err != nil {
		return "unknown"
	}
	return strings.TrimSpace(string(out))
}

func trimDesktopValue(value string) string {
	value = strings.TrimSpace(value)
	value = strings.Trim(value, "'\"")
	if _, err := strconv.Atoi(value); err == nil {
		return value
	}
	if value == "" {
		return "unknown"
	}
	return value
}
