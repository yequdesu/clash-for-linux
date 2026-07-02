package config

import (
	"fmt"
	"net"
	"os"
	"strconv"
	"strings"

	"gopkg.in/yaml.v3"
)

type RuntimeInfo struct {
	MixedPort          string
	Port               string
	SocksPort          string
	ProxyPort          string
	ExternalController string
	APIHost            string
	APIPort            string
	Secret             string
	DNSEnabled         bool
	DNSEnhancedMode    string
	FakeIPFilter       []string
	TunEnabled         bool
	TunStack           string
	TunAutoRoute       bool
	TunStrictRoute     bool
	TunDevice          string
}

func LoadRuntimeInfo(cfg *EnvConfig) RuntimeInfo {
	info, _ := ReadRuntimeInfo(cfg.RuntimePath())
	return info
}

func ReadRuntimeInfo(path string) (RuntimeInfo, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return RuntimeInfo{}, err
	}
	return ParseRuntimeInfo(data)
}

func ParseRuntimeInfo(data []byte) (RuntimeInfo, error) {
	var raw struct {
		MixedPort          any    `yaml:"mixed-port"`
		Port               any    `yaml:"port"`
		SocksPort          any    `yaml:"socks-port"`
		ExternalController string `yaml:"external-controller"`
		Secret             string `yaml:"secret"`
		DNS                struct {
			Enable       bool     `yaml:"enable"`
			EnhancedMode string   `yaml:"enhanced-mode"`
			FakeIPFilter []string `yaml:"fake-ip-filter"`
		} `yaml:"dns"`
		Tun struct {
			Enable      bool   `yaml:"enable"`
			Stack       string `yaml:"stack"`
			AutoRoute   bool   `yaml:"auto-route"`
			StrictRoute bool   `yaml:"strict-route"`
			Device      string `yaml:"device"`
		} `yaml:"tun"`
	}
	if err := yaml.Unmarshal(data, &raw); err != nil {
		return RuntimeInfo{}, fmt.Errorf("parse runtime yaml: %w", err)
	}

	info := RuntimeInfo{
		MixedPort:          scalarString(raw.MixedPort),
		Port:               scalarString(raw.Port),
		SocksPort:          scalarString(raw.SocksPort),
		ExternalController: strings.TrimSpace(raw.ExternalController),
		Secret:             raw.Secret,
		DNSEnabled:         raw.DNS.Enable,
		DNSEnhancedMode:    strings.TrimSpace(raw.DNS.EnhancedMode),
		FakeIPFilter:       raw.DNS.FakeIPFilter,
		TunEnabled:         raw.Tun.Enable,
		TunStack:           strings.TrimSpace(raw.Tun.Stack),
		TunAutoRoute:       raw.Tun.AutoRoute,
		TunStrictRoute:     raw.Tun.StrictRoute,
		TunDevice:          strings.TrimSpace(raw.Tun.Device),
	}
	if info.MixedPort != "" {
		info.ProxyPort = info.MixedPort
	} else {
		info.ProxyPort = info.Port
	}
	info.APIHost, info.APIPort = splitController(info.ExternalController)
	return info, nil
}

func (r RuntimeInfo) APIBaseURL(defaultPort string) string {
	return "http://" + r.APIAddress(defaultPort)
}

func (r RuntimeInfo) APIAddress(defaultPort string) string {
	return net.JoinHostPort(r.APIDialHost(), r.APIPortOrDefault(defaultPort))
}

func (r RuntimeInfo) APIPortOrDefault(defaultPort string) string {
	port := strings.TrimSpace(r.APIPort)
	if port == "" {
		return defaultPort
	}
	return port
}

func (r RuntimeInfo) APIDialHost() string {
	host := strings.Trim(strings.TrimSpace(r.APIHost), "[]")
	switch strings.ToLower(host) {
	case "", "0.0.0.0":
		return "127.0.0.1"
	case "::":
		return "::1"
	case "localhost":
		return "localhost"
	}
	if ip := net.ParseIP(host); ip != nil && ip.IsUnspecified() {
		if ip.To4() == nil {
			return "::1"
		}
		return "127.0.0.1"
	}
	return host
}

func scalarString(v any) string {
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

func splitController(controller string) (host, port string) {
	controller = strings.TrimSpace(controller)
	if controller == "" {
		return "", ""
	}
	if h, p, err := net.SplitHostPort(controller); err == nil {
		return strings.Trim(h, "[]"), p
	}
	if strings.Count(controller, ":") == 1 {
		parts := strings.SplitN(controller, ":", 2)
		return strings.Trim(parts[0], "[]"), parts[1]
	}
	return strings.Trim(controller, "[]"), ""
}
