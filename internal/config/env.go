package config

import (
	"bufio"
	"os"
	osuser "os/user"
	"path/filepath"
	"strings"
)

type EnvConfig struct {
	KernelName          string
	ServiceName         string
	ClashBaseDir        string
	ClashConfigURL      string
	ClashSubUA          string
	InitType            string
	URLGhProxy          string
	VersionMihomo       string
	VersionYQ           string
	VersionGeodata      string
	VersionSubConverter string
}

func LoadEnv() *EnvConfig {
	cfg := &EnvConfig{
		KernelName:          "mihomo",
		ServiceName:         "clashctl",
		ClashBaseDir:        resolveBaseDir(),
		ClashSubUA:          "clash-verge/v2.4.0",
		URLGhProxy:          "https://gh-proxy.org",
		VersionMihomo:       "v1.19.17",
		VersionYQ:           "v4.49.2",
		VersionGeodata:      "latest",
		VersionSubConverter: "v0.9.0",
	}

	parseEnvFile("/etc/clashctl/install.env", cfg)
	parseUserInstallMarker(cfg)
	parseEnvFile(filepath.Join(cfg.ClashBaseDir, ".env"), cfg)
	applyProcessEnv(cfg)
	return cfg
}

func resolveBaseDir() string {
	if dir := os.Getenv("CLASH_BASE_DIR"); dir != "" {
		return expandHome(dir)
	}

	if dir, ok := readBaseDirFromEnvFile("/etc/clashctl/install.env", ""); ok {
		return dir
	}

	if sudoUser := os.Getenv("SUDO_USER"); sudoUser != "" && os.Getuid() == 0 {
		if dir := resolveSudoUserBaseDir(sudoUser); dir != "" {
			return dir
		}
	}

	home, _ := os.UserHomeDir()

	if dir, ok := readBaseDirFromEnvFile(filepath.Join(home, ".config", "clashctl", "install.env"), home); ok {
		return dir
	}

	return defaultBaseDir(home)
}

func resolveSudoUserBaseDir(sudoUser string) string {
	home := lookupUserHome(sudoUser)
	if home == "" {
		return ""
	}
	if dir, ok := readBaseDirFromEnvFile(filepath.Join(home, ".config", "clashctl", "install.env"), home); ok {
		return dir
	}
	return defaultBaseDir(home)
}

func readBaseDirFromEnvFile(path, homeOverride string) (string, bool) {
	f, err := os.Open(path)
	if err != nil {
		return "", false
	}
	defer f.Close()
	sc := bufio.NewScanner(f)
	for sc.Scan() {
		line := strings.TrimSpace(sc.Text())
		if strings.HasPrefix(line, "CLASH_BASE_DIR=") {
			return expandHomeFor(strings.TrimSpace(strings.TrimPrefix(line, "CLASH_BASE_DIR=")), homeOverride), true
		}
	}
	return "", false
}

func defaultBaseDir(home string) string {
	defaultDir := filepath.Join(home, "clashctl")
	if _, err := os.Stat(defaultDir); err == nil {
		return defaultDir
	}
	hiddenDir := filepath.Join(home, ".clashctl")
	if _, err := os.Stat(hiddenDir); err == nil {
		return hiddenDir
	}
	return hiddenDir
}

func lookupUserHome(name string) string {
	u, err := osuser.Lookup(name)
	if err == nil && u.HomeDir != "" {
		return u.HomeDir
	}
	if name == "root" {
		return "/root"
	}
	candidate := filepath.Join("/home", name)
	if _, err := os.Stat(candidate); err == nil {
		return candidate
	}
	return ""
}

func expandHome(path string) string {
	return expandHomeFor(path, "")
}

func expandHomeFor(path, homeOverride string) string {
	if strings.HasPrefix(path, "~/") {
		home := homeOverride
		if home == "" {
			home, _ = os.UserHomeDir()
		}
		return filepath.Join(home, path[2:])
	}
	if path == "~" {
		home := homeOverride
		if home == "" {
			home, _ = os.UserHomeDir()
		}
		return home
	}
	return path
}

func parseEnvFile(path string, cfg *EnvConfig) {
	parseEnvFileWithHome(path, cfg, "")
}

func parseEnvFileWithHome(path string, cfg *EnvConfig, homeOverride string) {
	f, err := os.Open(path)
	if err != nil {
		return
	}
	defer f.Close()
	sc := bufio.NewScanner(f)
	for sc.Scan() {
		line := strings.TrimSpace(sc.Text())
		if line == "" || strings.HasPrefix(line, "#") {
			continue
		}
		parts := strings.SplitN(line, "=", 2)
		if len(parts) != 2 {
			continue
		}
		key := strings.TrimSpace(parts[0])
		val := strings.TrimSpace(parts[1])
		switch key {
		case "KERNEL_NAME":
			cfg.KernelName = val
		case "SERVICE_NAME", "CLASH_SERVICE_NAME":
			cfg.ServiceName = val
		case "CLASH_BASE_DIR":
			cfg.ClashBaseDir = expandHomeFor(val, homeOverride)
		case "CLASH_CONFIG_URL":
			cfg.ClashConfigURL = val
		case "CLASH_SUB_UA":
			cfg.ClashSubUA = val
		case "INIT_TYPE":
			cfg.InitType = val
		case "URL_GH_PROXY":
			cfg.URLGhProxy = val
		case "VERSION_MIHOMO":
			cfg.VersionMihomo = val
		case "VERSION_YQ":
			cfg.VersionYQ = val
		case "VERSION_GEODATA":
			cfg.VersionGeodata = val
		case "VERSION_SUBCONVERTER":
			cfg.VersionSubConverter = val
		}
	}
}

func parseUserInstallMarker(cfg *EnvConfig) {
	if sudoUser := os.Getenv("SUDO_USER"); sudoUser != "" && os.Getuid() == 0 {
		if home := lookupUserHome(sudoUser); home != "" {
			parseEnvFileWithHome(filepath.Join(home, ".config", "clashctl", "install.env"), cfg, home)
			return
		}
	}
	home, err := os.UserHomeDir()
	if err != nil || home == "" {
		return
	}
	parseEnvFileWithHome(filepath.Join(home, ".config", "clashctl", "install.env"), cfg, home)
}

func applyProcessEnv(cfg *EnvConfig) {
	if val := os.Getenv("KERNEL_NAME"); val != "" {
		cfg.KernelName = val
	}
	if val := os.Getenv("SERVICE_NAME"); val != "" {
		cfg.ServiceName = val
	} else if val := os.Getenv("CLASH_SERVICE_NAME"); val != "" {
		cfg.ServiceName = val
	}
	if val := os.Getenv("CLASH_BASE_DIR"); val != "" {
		cfg.ClashBaseDir = expandHome(val)
	}
	if val := os.Getenv("CLASH_CONFIG_URL"); val != "" {
		cfg.ClashConfigURL = val
	}
	if val := os.Getenv("CLASH_SUB_UA"); val != "" {
		cfg.ClashSubUA = val
	}
	if val := os.Getenv("INIT_TYPE"); val != "" {
		cfg.InitType = val
	}
	if val := os.Getenv("URL_GH_PROXY"); val != "" {
		cfg.URLGhProxy = val
	}
	if val := os.Getenv("VERSION_MIHOMO"); val != "" {
		cfg.VersionMihomo = val
	}
	if val := os.Getenv("VERSION_YQ"); val != "" {
		cfg.VersionYQ = val
	}
	if val := os.Getenv("VERSION_GEODATA"); val != "" {
		cfg.VersionGeodata = val
	}
	if val := os.Getenv("VERSION_SUBCONVERTER"); val != "" {
		cfg.VersionSubConverter = val
	}
}

func (c *EnvConfig) ResourcesDir() string    { return filepath.Join(c.ClashBaseDir, "resources") }
func (c *EnvConfig) ConfigPath() string      { return filepath.Join(c.ResourcesDir(), "config.yaml") }
func (c *EnvConfig) MixinPath() string       { return filepath.Join(c.ResourcesDir(), "mixin.yaml") }
func (c *EnvConfig) RuntimePath() string     { return filepath.Join(c.ResourcesDir(), "runtime.yaml") }
func (c *EnvConfig) TempPath() string        { return filepath.Join(c.ResourcesDir(), "temp.yaml") }
func (c *EnvConfig) BinDir() string          { return filepath.Join(c.ClashBaseDir, "bin") }
func (c *EnvConfig) LogDir() string          { return filepath.Join(c.ClashBaseDir, "logs") }
func (c *EnvConfig) TrafficDir() string      { return filepath.Join(c.ClashBaseDir, "traffic") }
func (c *EnvConfig) KernelBin() string       { return filepath.Join(c.BinDir(), c.KernelName) }
func (c *EnvConfig) YQBin() string           { return filepath.Join(c.BinDir(), "yq") }
func (c *EnvConfig) SubconverterDir() string { return filepath.Join(c.BinDir(), "subconverter") }
func (c *EnvConfig) SubconverterBin() string {
	return filepath.Join(c.SubconverterDir(), "subconverter")
}
func (c *EnvConfig) SubconverterConfig() string {
	return filepath.Join(c.SubconverterDir(), "pref.yml")
}
func (c *EnvConfig) ProfilesMeta() string { return filepath.Join(c.ResourcesDir(), "profiles.yaml") }
func (c *EnvConfig) ProfilesDir() string  { return filepath.Join(c.ResourcesDir(), "profiles") }
func (c *EnvConfig) ProfilesLog() string  { return filepath.Join(c.ResourcesDir(), "profiles.log") }
func (c *EnvConfig) ConfigsDir() string   { return filepath.Join(c.ResourcesDir(), "configs") }
func (c *EnvConfig) LogFile() string      { return filepath.Join(c.LogDir(), c.KernelName+".log") }
func (c *EnvConfig) InstallState() string { return filepath.Join(c.ClashBaseDir, "install-state.json") }
func (c *EnvConfig) PidFile() string {
	return filepath.Join(c.ClashBaseDir, "runtime", c.KernelName+".pid")
}
