package main

import (
	"fmt"
	urlpkg "net/url"
	"os"
	"path/filepath"
	"strings"
	"time"

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

		result, err := addSubscription(cfg, source)
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

type addSubscriptionResult struct {
	ID            int
	Source        string
	ActivateFirst bool
}

var saveProfiles = config.SaveProfiles

func addSubscription(cfg *config.EnvConfig, rawSource string) (addSubscriptionResult, error) {
	result := addSubscriptionResult{Source: normalizeSubscriptionSource(rawSource)}
	if result.Source == "" {
		return result, fmt.Errorf("subscription URL cannot be empty")
	}

	tempPath, err := newTempConfigPath(cfg)
	if err != nil {
		return result, fmt.Errorf("create temp config failed: %w", err)
	}
	defer os.Remove(tempPath)

	if err := sub.Download(result.Source, tempPath, cfg.ClashSubUA); err != nil {
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
		meta.Profiles = append(meta.Profiles, config.Profile{
			ID:      result.ID,
			Path:    profilePath,
			URL:     result.Source,
			Name:    profileNameFromSource(result.Source),
			Updated: time.Now().Format("2006-01-02 15:04:05"),
		})
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
