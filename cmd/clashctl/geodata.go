package main

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/spf13/cobra"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	ilog "github.com/yequdesu/linux-cli-tui-clash/internal/log"
)

var geodataVersion string

var geodataCmd = &cobra.Command{
	Use:   "geodata <update>",
	Short: "Manage geosite/geoip/mmdb databases",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		showHelpAndExit(cmd)
	},
}

var geodataUpdateCmd = &cobra.Command{
	Use:   "update",
	Short: "Update Country.mmdb, geosite.dat, and geoip.dat",
	Run: func(cmd *cobra.Command, args []string) {
		requireInstall()
		version := geodataVersion
		if version == "" {
			version = cfg.VersionGeodata
		}
		if err := updateGeodata(cfg, version); err != nil {
			ilog.Fatal("geodata update failed: %v", err)
		}
		ilog.Ok("geodata updated")
	},
}

func init() {
	geodataUpdateCmd.Flags().StringVar(&geodataVersion, "version", "", "meta-rules-dat release tag, or latest")
	geodataCmd.AddCommand(geodataUpdateCmd)
}

func updateGeodata(cfg *config.EnvConfig, version string) error {
	base := geodataSourceBase(version)
	return updateGeodataFromBase(cfg, version, base)
}

var geodataFileNames = []string{"Country.mmdb", "geosite.dat", "geoip.dat"}

func updateGeodataFromBase(cfg *config.EnvConfig, version, base string) error {
	if err := os.MkdirAll(cfg.ResourcesDir(), 0755); err != nil {
		return err
	}
	staging, err := os.MkdirTemp(cfg.ResourcesDir(), ".geodata-*")
	if err != nil {
		return err
	}
	defer os.RemoveAll(staging)

	for _, name := range geodataFileNames {
		url := base + "/" + name
		ilog.Info("downloading %s", name)
		if err := downloadFileAtomic(url, filepath.Join(staging, name), cfg.ClashSubUA, cfg.URLGhProxy); err != nil {
			return fmt.Errorf("%s: %w", name, err)
		}
	}
	rollback, err := replaceGeodataFiles(cfg.ResourcesDir(), staging, geodataFileNames)
	if err != nil {
		return err
	}
	if err := updateInstallStateGeodata(cfg, version, base); err != nil {
		rollback()
		return fmt.Errorf("install state not updated: %w", err)
	}
	return nil
}

func geodataSourceBase(version string) string {
	version = strings.TrimSpace(version)
	if version == "" || version == "latest" {
		return "https://github.com/MetaCubeX/meta-rules-dat/releases/latest/download"
	}
	return "https://github.com/MetaCubeX/meta-rules-dat/releases/download/" + version
}

func downloadFileAtomic(rawURL, dest, userAgent, proxyPrefix string) error {
	tmp, err := os.CreateTemp(filepath.Dir(dest), filepath.Base(dest)+".*.tmp")
	if err != nil {
		return err
	}
	tmpPath := tmp.Name()
	_ = tmp.Close()
	defer os.Remove(tmpPath)

	if err := downloadToPath(rawURL, tmpPath, userAgent); err != nil {
		if proxyPrefix == "" {
			return err
		}
		proxyURL := strings.TrimRight(proxyPrefix, "/") + "/" + rawURL
		if proxyErr := downloadToPath(proxyURL, tmpPath, userAgent); proxyErr != nil {
			return fmt.Errorf("%w; proxy fallback: %v", err, proxyErr)
		}
	}
	fi, err := os.Stat(tmpPath)
	if err != nil {
		return err
	}
	if fi.Size() == 0 {
		return fmt.Errorf("downloaded file is empty")
	}
	return os.Rename(tmpPath, dest)
}

type geodataReplacement struct {
	dest      string
	backup    string
	hadBackup bool
	installed bool
}

func replaceGeodataFiles(resourcesDir, stagingDir string, names []string) (func(), error) {
	replacements := make([]geodataReplacement, 0, len(names))
	for _, name := range names {
		staged := filepath.Join(stagingDir, name)
		dest := filepath.Join(resourcesDir, name)
		replacement := geodataReplacement{dest: dest}

		if _, err := os.Stat(dest); err == nil {
			replacement.backup = filepath.Join(stagingDir, name+".old")
			if err := os.Rename(dest, replacement.backup); err != nil {
				rollbackGeodataReplacements(replacements)
				return func() {}, fmt.Errorf("backup %s: %w", name, err)
			}
			replacement.hadBackup = true
		} else if !os.IsNotExist(err) {
			rollbackGeodataReplacements(replacements)
			return func() {}, fmt.Errorf("stat %s: %w", name, err)
		}

		replacements = append(replacements, replacement)
		if err := os.Rename(staged, dest); err != nil {
			rollbackGeodataReplacements(replacements)
			return func() {}, fmt.Errorf("replace %s: %w", name, err)
		}
		replacements[len(replacements)-1].installed = true
	}
	return func() {
		rollbackGeodataReplacements(replacements)
	}, nil
}

func rollbackGeodataReplacements(replacements []geodataReplacement) {
	for i := len(replacements) - 1; i >= 0; i-- {
		replacement := replacements[i]
		if replacement.installed {
			_ = os.Remove(replacement.dest)
		}
		if replacement.hadBackup {
			_ = os.Rename(replacement.backup, replacement.dest)
		}
	}
}

func downloadToPath(rawURL, dest, userAgent string) error {
	client := &http.Client{Timeout: 30 * time.Second}
	req, err := http.NewRequest("GET", rawURL, nil)
	if err != nil {
		return err
	}
	if userAgent != "" {
		req.Header.Set("User-Agent", userAgent)
	}
	resp, err := client.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return fmt.Errorf("HTTP %d", resp.StatusCode)
	}
	out, err := os.Create(dest)
	if err != nil {
		return err
	}
	defer out.Close()
	if _, err := io.Copy(out, resp.Body); err != nil {
		return err
	}
	return nil
}

func updateInstallStateGeodata(cfg *config.EnvConfig, version, sourceBase string) error {
	data, err := os.ReadFile(cfg.InstallState())
	if err != nil {
		return err
	}
	var state map[string]any
	if err := json.Unmarshal(data, &state); err != nil {
		return err
	}
	components, ok := state["components"].(map[string]any)
	if !ok {
		components = map[string]any{}
		state["components"] = components
	}
	components["geodata"] = map[string]any{
		"version":    version,
		"source_url": sourceBase,
		"path":       cfg.ResourcesDir(),
		"updated_at": time.Now().UTC().Format(time.RFC3339),
	}
	out, err := json.MarshalIndent(state, "", "  ")
	if err != nil {
		return err
	}
	return config.AtomicWriteFile(cfg.InstallState(), append(out, '\n'), 0644)
}
