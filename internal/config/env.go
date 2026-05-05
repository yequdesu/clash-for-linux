package config

import (
	"bufio"
	"os"
	"path/filepath"
	"strings"
)

type EnvConfig struct {
	KernelName          string
	ClashBaseDir        string
	ClashConfigURL      string
	ClashSubUA          string
	InitType            string
	URLGhProxy          string
	VersionMihomo       string
	VersionYQ           string
	VersionSubConverter string
}

func LoadEnv() *EnvConfig {
	cfg := &EnvConfig{
		KernelName:          "mihomo",
		ClashBaseDir:        resolveBaseDir(),
		ClashSubUA:          "clash-verge/v2.4.0",
		URLGhProxy:          "https://gh-proxy.org",
		VersionMihomo:       "v1.19.17",
		VersionYQ:           "v4.49.2",
		VersionSubConverter: "v0.9.0",
	}

	envFile := filepath.Join(cfg.ClashBaseDir, ".env")
	parseEnvFile(envFile, cfg)
	return cfg
}

func resolveBaseDir() string {
	if dir := os.Getenv("CLASH_BASE_DIR"); dir != "" {
		return expandHome(dir)
	}

	if data, err := os.ReadFile("/etc/clashctl/install.env"); err == nil {
		for _, line := range strings.Split(string(data), "\n") {
			line = strings.TrimSpace(line)
			if strings.HasPrefix(line, "CLASH_BASE_DIR=") {
				return expandHome(strings.TrimPrefix(line, "CLASH_BASE_DIR="))
			}
		}
	}

	if sudoUser := os.Getenv("SUDO_USER"); sudoUser != "" && os.Getuid() == 0 {
		for _, homeBase := range []string{"/home", "/root"} {
			markerFile := filepath.Join(homeBase, sudoUser, ".config", "clashctl", "install.env")
			if f, err := os.Open(markerFile); err == nil {
				defer f.Close()
				sc := bufio.NewScanner(f)
				for sc.Scan() {
					line := strings.TrimSpace(sc.Text())
					if strings.HasPrefix(line, "CLASH_BASE_DIR=") {
						return expandHome(strings.TrimPrefix(line, "CLASH_BASE_DIR="))
					}
				}
			}
		}
	}

	home, _ := os.UserHomeDir()

	markerFile := filepath.Join(home, ".config", "clashctl", "install.env")
	if f, err := os.Open(markerFile); err == nil {
		defer f.Close()
		sc := bufio.NewScanner(f)
		for sc.Scan() {
			line := strings.TrimSpace(sc.Text())
			if strings.HasPrefix(line, "CLASH_BASE_DIR=") {
				return expandHome(strings.TrimPrefix(line, "CLASH_BASE_DIR="))
			}
		}
	}

	defaultDir := filepath.Join(home, "clashctl")
	if _, err := os.Stat(defaultDir); err == nil {
		return defaultDir
	}
	return filepath.Join(home, ".clashctl")
}

func expandHome(path string) string {
	if strings.HasPrefix(path, "~/") {
		home, _ := os.UserHomeDir()
		return filepath.Join(home, path[2:])
	}
	if path == "~" {
		home, _ := os.UserHomeDir()
		return home
	}
	return path
}

func parseEnvFile(path string, cfg *EnvConfig) {
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
		case "CLASH_BASE_DIR":
			cfg.ClashBaseDir = expandHome(val)
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
		case "VERSION_SUBCONVERTER":
			cfg.VersionSubConverter = val
		}
	}
}

func (c *EnvConfig) ResourcesDir() string    { return filepath.Join(c.ClashBaseDir, "resources") }
func (c *EnvConfig) ConfigPath() string      { return filepath.Join(c.ResourcesDir(), "config.yaml") }
func (c *EnvConfig) MixinPath() string       { return filepath.Join(c.ResourcesDir(), "mixin.yaml") }
func (c *EnvConfig) RuntimePath() string     { return filepath.Join(c.ResourcesDir(), "runtime.yaml") }
func (c *EnvConfig) TempPath() string        { return filepath.Join(c.ResourcesDir(), "temp.yaml") }
func (c *EnvConfig) BinDir() string          { return filepath.Join(c.ClashBaseDir, "bin") }
func (c *EnvConfig) KernelBin() string       { return filepath.Join(c.BinDir(), c.KernelName) }
func (c *EnvConfig) YQBin() string           { return filepath.Join(c.BinDir(), "yq") }
func (c *EnvConfig) SubconverterDir() string { return filepath.Join(c.BinDir(), "subconverter") }
func (c *EnvConfig) SubconverterBin() string { return filepath.Join(c.SubconverterDir(), "subconverter") }
func (c *EnvConfig) SubconverterConfig() string {
	return filepath.Join(c.SubconverterDir(), "pref.yml")
}
func (c *EnvConfig) ProfilesMeta() string { return filepath.Join(c.ResourcesDir(), "profiles.yaml") }
func (c *EnvConfig) ProfilesDir() string  { return filepath.Join(c.ResourcesDir(), "profiles") }
func (c *EnvConfig) ProfilesLog() string  { return filepath.Join(c.ResourcesDir(), "profiles.log") }
func (c *EnvConfig) ConfigsDir() string   { return filepath.Join(c.ResourcesDir(), "configs") }
func (c *EnvConfig) LogFile() string      { return "/var/log/" + c.KernelName + ".log" }
func (c *EnvConfig) PidFile() string      { return "/run/" + c.KernelName + ".pid" }
