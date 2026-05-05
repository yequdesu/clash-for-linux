package main

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	"github.com/yequdesu/linux-cli-tui-clash/internal/sub"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var subAddCmd = &cobra.Command{
	Use:   "add <url|path>",
	Short: "Add a subscription",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		url := ""
		if len(args) > 0 {
			url = args[0]
		} else {
			fmt.Print("Enter subscription URL: ")
			fmt.Scanln(&url)
			if url == "" {
				ilog.Fatal("subscription URL cannot be empty")
			}
		}

		meta, _ := config.LoadProfiles(cfg.ProfilesMeta())
		if meta.FindByURL(url) != nil {
			ilog.Warn("subscription already exists")
			return
		}

		if !hasProtocol(url) && fileExists(expandPath(url)) {
			url = "file://" + expandPath(url)
		}

		if err := sub.Download(url, cfg.TempPath(), cfg.ClashSubUA); err != nil {
			ilog.Warn("download failed: %v", err)
			ilog.Info("you can also drop .yaml files in %s and run 'clashctl sub import'", cfg.ConfigsDir())
			return
		}

		if err := validateConfigFile(cfg, cfg.TempPath()); err != nil {
			ilog.Info("direct validation failed, trying subscription conversion...")
			if err := sub.ConvertDownload(cfg, url, cfg.TempPath()); err != nil {
				ilog.Warn("conversion also failed: %v", err)
				ilog.Info("you can manually place a config file and use:")
				ilog.Info("  clashctl sub add file:///path/to/config.yaml")
				ilog.Info("  or drop it in %s and run 'clashctl sub import'", cfg.ConfigsDir())
				return
			}
			if err := validateConfigFile(cfg, cfg.TempPath()); err != nil {
				ilog.Warn("config validation failed")
				return
			}
		}

		id := meta.NextID()
		profilePath := filepath.Join(cfg.ProfilesDir(), fmt.Sprintf("%d.yaml", id))
		os.Rename(cfg.TempPath(), profilePath)
		meta.Profiles = append(meta.Profiles, config.Profile{
			ID: id, Path: profilePath, URL: url,
		})
		config.SaveProfiles(cfg.ProfilesMeta(), meta)
		logSub(fmt.Sprintf("added: [%d] %s", id, url))
		ilog.Ok("subscription added: [%d] %s", id, url)

		if meta.Use == 0 {
			ilog.Info("activating first subscription...")
			switchSubscription(cfg, id)
		}
	},
}

func hasProtocol(url string) bool {
	return len(url) >= 7 && (url[:7] == "http://" || url[:8] == "https://" || url[:7] == "file://")
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
