package main

import (
	"fmt"
	urlpkg "net/url"
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
	"github.com/yequdesu/linux-cli-tui-clash/internal/sub"
)

var subAddCmd = &cobra.Command{
	Use:   "add <url|path>",
	Short: "Add a subscription",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		source := ""
		if len(args) > 0 {
			source = args[0]
		} else {
			fmt.Print("Enter subscription URL: ")
			fmt.Scanln(&source)
			if source == "" {
				ilog.Fatal("subscription URL cannot be empty")
			}
		}

		options, err := addSubscriptionOptionsFromFlags()
		if err != nil {
			ilog.Fatal("%v", err)
		}

		result, err := addSubscriptionWithOptions(cfg, source, options)
		if err != nil {
			ilog.Info("you can also drop .yaml files in %s and run 'clashctl sub import'", cfg.ConfigsDir())
			ilog.Fatal("%v", err)
		}

		logSub(fmt.Sprintf("added: [%d] %s", result.ID, result.Source))
		ilog.Ok("subscription added: [%d] %s", result.ID, result.Source)

		if result.ActivateFirst {
			ilog.Info("activating first subscription...")
			if err := switchSubscription(cfg, result.ID); err != nil {
				ilog.Fatal("%v", err)
			}
		}
	},
}

type addSubscriptionOptions struct {
	Name           string
	UpdateInterval string
	UpdateEnabled  *bool
	UpdateProxy    string
	UserAgent      string
	ConvertMode    string
	Tags           []string
}

type addSubscriptionResult struct {
	ID            int
	Source        string
	ActivateFirst bool
}

var (
	subAddName        string
	subAddInterval    string
	subAddUpdateProxy string
	subAddUserAgent   string
	subAddConvertMode string
	subAddTags        []string
)

var saveProfiles = config.SaveProfiles

func addSubscription(cfg *config.EnvConfig, rawSource string) (addSubscriptionResult, error) {
	return addSubscriptionWithOptions(cfg, rawSource, addSubscriptionOptions{})
}

func addSubscriptionWithOptions(cfg *config.EnvConfig, rawSource string, options addSubscriptionOptions) (addSubscriptionResult, error) {
	result := addSubscriptionResult{Source: normalizeSubscriptionSource(rawSource)}
	if result.Source == "" {
		return result, fmt.Errorf("subscription URL cannot be empty")
	}
	options, err := normalizeAddSubscriptionOptions(options)
	if err != nil {
		return result, err
	}

	tempPath, err := newTempConfigPath(cfg)
	if err != nil {
		return result, fmt.Errorf("create temp config failed: %w", err)
	}
	defer os.Remove(tempPath)

	userAgent := cfg.ClashSubUA
	if options.UserAgent != "" {
		userAgent = options.UserAgent
	}
	downloadOpts := sub.DownloadOptions{ProxyMode: sub.ProxyModeSystem}
	if options.UpdateProxy != "" {
		var label string
		downloadOpts, label, err = subscriptionDownloadOptions(cfg, config.Profile{UpdateProxy: options.UpdateProxy})
		if err != nil {
			return result, err
		}
		ilog.Info("subscription download path: %s", label)
	}

	if err := sub.DownloadWithOptions(result.Source, tempPath, userAgent, downloadOpts); err != nil {
		return result, fmt.Errorf("download failed: %w", err)
	}

	if err := validateConfigFile(cfg, tempPath); err != nil {
		ilog.Info("direct validation failed, trying subscription conversion...")
		if err := sub.ConvertDownload(cfg, result.Source, tempPath); err != nil {
			return result, fmt.Errorf("conversion also failed: %w", err)
		}
		if err := validateConfigFile(cfg, tempPath); err != nil {
			return result, fmt.Errorf("config validation failed: %w", err)
		}
	}

	data, err := os.ReadFile(tempPath)
	if err != nil {
		return result, fmt.Errorf("cannot read downloaded profile: %w", err)
	}

	profilePath := ""
	if err := config.WithProfilesLock(cfg, func() error {
		meta, err := config.LoadProfiles(cfg.ProfilesMeta())
		if err != nil {
			return err
		}
		if meta.FindByURL(result.Source) != nil {
			return fmt.Errorf("subscription already exists")
		}

		result.ID = meta.NextID()
		profilePath = filepath.Join(cfg.ProfilesDir(), fmt.Sprintf("%d.yaml", result.ID))
		if err := config.AtomicWriteFile(profilePath, data, 0644); err != nil {
			return fmt.Errorf("cannot save profile: %w", err)
		}
		profile := config.Profile{
			ID:      result.ID,
			Path:    profilePath,
			URL:     result.Source,
			Name:    profileNameFromSource(result.Source),
			Updated: timeNow().Format(profileTimeFormat),
		}
		if options.Name != "" {
			profile.Name = options.Name
		}
		if options.UpdateEnabled != nil {
			profile.UpdateEnabled = options.UpdateEnabled
			profile.UpdateInterval = options.UpdateInterval
			profile.Interval = options.UpdateInterval
			profile.NextUpdate = nextProfileUpdateString(profile, timeNow())
		}
		profile.UpdateProxy = options.UpdateProxy
		profile.UserAgent = options.UserAgent
		profile.ConvertMode = options.ConvertMode
		profile.Tags = options.Tags
		meta.Profiles = append(meta.Profiles, profile)
		result.ActivateFirst = meta.Use == 0
		return saveProfiles(cfg.ProfilesMeta(), meta)
	}); err != nil {
		if profilePath != "" {
			_ = os.Remove(profilePath)
		}
		return result, err
	}
	return result, nil
}

func init() {
	subAddCmd.Flags().StringVar(&subAddName, "name", "", "subscription display name")
	subAddCmd.Flags().StringVar(&subAddInterval, "interval", "", "scheduled update interval, for example 12h or off")
	subAddCmd.Flags().StringVar(&subAddUpdateProxy, "update-proxy", "", "subscription update network path: direct|system|core|auto")
	subAddCmd.Flags().StringVar(&subAddUserAgent, "user-agent", "", "per-profile User-Agent for subscription updates")
	subAddCmd.Flags().StringVar(&subAddConvertMode, "convert", "", "subscription conversion mode: auto|off|force")
	subAddCmd.Flags().StringArrayVar(&subAddTags, "tag", nil, "subscription tag; repeatable")
}

func addSubscriptionOptionsFromFlags() (addSubscriptionOptions, error) {
	return normalizeAddSubscriptionOptions(addSubscriptionOptions{
		Name:           subAddName,
		UpdateInterval: subAddInterval,
		UpdateProxy:    subAddUpdateProxy,
		UserAgent:      subAddUserAgent,
		ConvertMode:    subAddConvertMode,
		Tags:           append([]string(nil), subAddTags...),
	})
}

func normalizeAddSubscriptionOptions(options addSubscriptionOptions) (addSubscriptionOptions, error) {
	options.Name = strings.TrimSpace(options.Name)
	options.UpdateInterval = strings.TrimSpace(options.UpdateInterval)
	options.UpdateProxy = strings.TrimSpace(options.UpdateProxy)
	options.UserAgent = strings.TrimSpace(options.UserAgent)
	options.ConvertMode = strings.TrimSpace(options.ConvertMode)
	if options.UpdateInterval != "" {
		interval, enabled, err := normalizeUpdateInterval(options.UpdateInterval)
		if err != nil {
			return options, err
		}
		options.UpdateInterval = interval
		options.UpdateEnabled = boolPtr(enabled)
	}
	if options.UpdateProxy != "" {
		value, err := normalizeProfileEnum(options.UpdateProxy, "update proxy", []string{"direct", "system", "core", "auto"})
		if err != nil {
			return options, err
		}
		options.UpdateProxy = value
	}
	if options.ConvertMode != "" {
		value, err := normalizeProfileEnum(options.ConvertMode, "convert mode", []string{"auto", "off", "force"})
		if err != nil {
			return options, err
		}
		options.ConvertMode = value
	}
	tags := make([]string, 0, len(options.Tags))
	for _, tag := range options.Tags {
		tag = strings.TrimSpace(tag)
		if tag != "" {
			tags = addUniqueTag(tags, tag)
		}
	}
	options.Tags = tags
	return options, nil
}

func normalizeSubscriptionSource(source string) string {
	source = strings.TrimSpace(source)
	if strings.HasPrefix(source, "file://") {
		return canonicalFileSource(strings.TrimPrefix(source, "file://"))
	}
	if !hasProtocol(source) && fileExists(expandPath(source)) {
		return canonicalFileSource(source)
	}
	return source
}

func canonicalFileSource(path string) string {
	path = expandPath(strings.TrimSpace(path))
	if abs, err := filepath.Abs(path); err == nil {
		path = abs
	}
	if resolved, err := filepath.EvalSymlinks(path); err == nil {
		path = resolved
	}
	return "file://" + filepath.Clean(path)
}

func profileNameFromSource(source string) string {
	source = strings.TrimSpace(source)
	if strings.HasPrefix(source, "file://") {
		return trimConfigExt(filepath.Base(strings.TrimPrefix(source, "file://")))
	}
	if u, err := urlpkg.Parse(source); err == nil && u.Host != "" {
		name := filepath.Base(strings.TrimRight(u.Path, "/"))
		if name == "." || name == "/" || name == "" {
			return u.Host
		}
		return trimConfigExt(name)
	}
	return trimConfigExt(filepath.Base(source))
}

func trimConfigExt(name string) string {
	name = strings.TrimSpace(name)
	ext := strings.ToLower(filepath.Ext(name))
	if ext == ".yaml" || ext == ".yml" {
		return strings.TrimSuffix(name, filepath.Ext(name))
	}
	return name
}

func hasProtocol(url string) bool {
	return strings.HasPrefix(url, "http://") ||
		strings.HasPrefix(url, "https://") ||
		strings.HasPrefix(url, "file://")
}

func fileExists(path string) bool {
	fi, err := os.Stat(path)
	return err == nil && !fi.IsDir()
}

func expandPath(path string) string {
	if len(path) > 0 && path[0] == '~' {
		home, _ := os.UserHomeDir()
		return filepath.Join(home, path[1:])
	}
	return path
}
