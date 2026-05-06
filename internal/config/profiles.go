package config

import (
	"fmt"
	"os"
	"sort"
	"strings"
	"time"

	"gopkg.in/yaml.v3"
)

// ProfilesConfig holds the subscription index
type ProfilesConfig struct {
	Use      int       `yaml:"use"`
	Profiles []Profile `yaml:"profiles"`
}

// Profile represents a subscription entry
type Profile struct {
	ID       int          `yaml:"id"`
	Path     string       `yaml:"path"`
	URL      string       `yaml:"url"`
	Name     string       `yaml:"name"`
	Updated  int64        `yaml:"updated"`
	Interval int          `yaml:"interval"`
	UserAgent string      `yaml:"user_agent,omitempty"`
	Extra    ProfileExtra `yaml:"extra,omitempty"`
}

// ProfileExtra holds subscription usage info
type ProfileExtra struct {
	Upload   int64 `yaml:"upload"`
	Download int64 `yaml:"download"`
	Total    int64 `yaml:"total"`
	Expire   int64 `yaml:"expire"`
}

// LoadProfiles reads the profiles.yaml file
func LoadProfiles(path string) (*ProfilesConfig, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		if os.IsNotExist(err) {
			return &ProfilesConfig{Use: 0, Profiles: []Profile{}}, nil
		}
		return nil, err
	}

	var cfg ProfilesConfig
	if err := yaml.Unmarshal(data, &cfg); err != nil {
		return nil, err
	}

	return &cfg, nil
}

// SaveProfiles writes the profiles configuration back to file
func SaveProfiles(path string, cfg *ProfilesConfig) error {
	// Sort profiles by ID
	sort.Slice(cfg.Profiles, func(i, j int) bool {
		return cfg.Profiles[i].ID < cfg.Profiles[j].ID
	})

	data, err := yaml.Marshal(cfg)
	if err != nil {
		return err
	}

	return os.WriteFile(path, data, 0644)
}

// GetActiveProfile returns the currently active profile
func GetActiveProfile(cfg *ProfilesConfig) *Profile {
	for i := range cfg.Profiles {
		if cfg.Profiles[i].ID == cfg.Use {
			return &cfg.Profiles[i]
		}
	}
	if len(cfg.Profiles) > 0 {
		return &cfg.Profiles[0]
	}
	return nil
}

// Now returns current Unix timestamp
func Now() int64 {
	return time.Now().Unix()
}

// InjectShellRC adds proxy environment variables to shell RC file
func InjectShellRC(rcPath string, port int) error {
	data, err := os.ReadFile(rcPath)
	if err != nil {
		return err
	}

	content := string(data)

	// Check if already injected
	if strings.Contains(content, "# clashctl START") {
		return nil
	}

	proxyBlock := fmt.Sprintf(`
# clashctl START
export http_proxy=http://127.0.0.1:%d
export HTTP_PROXY=http://127.0.0.1:%d
export https_proxy=http://127.0.0.1:%d
export HTTPS_PROXY=http://127.0.0.1:%d
export all_proxy=socks5h://127.0.0.1:%d
export ALL_PROXY=socks5h://127.0.0.1:%d
export no_proxy=localhost,127.0.0.0/8,::1
export NO_PROXY=localhost,127.0.0.0/8,::1
# clashctl END
`, port, port, port, port, port, port)

	content = content + proxyBlock
	return os.WriteFile(rcPath, []byte(content), 0644)
}

// RemoveShellRC removes proxy environment variables from shell RC file
func RemoveShellRC(rcPath string) error {
	data, err := os.ReadFile(rcPath)
	if err != nil {
		return err
	}

	content := string(data)

	start := "# clashctl START"
	end := "# clashctl END"

	for {
		startIdx := strings.Index(content, start)
		if startIdx == -1 {
			break
		}
		endIdx := strings.Index(content[startIdx:], end)
		if endIdx == -1 {
			break
		}
		endIdx += startIdx + len(end)
		content = content[:startIdx] + content[endIdx:]
	}

	return os.WriteFile(rcPath, []byte(content), 0644)
}
