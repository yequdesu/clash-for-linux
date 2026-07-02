package main

import (
	"errors"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestUpdateProfileMetadataPersistsParameterizedFields(t *testing.T) {
	cfg := testProfileEditConfig(t)

	err := updateProfileMetadata(cfg, 1, func(p *config.Profile) error {
		p.Name = "Renamed"
		p.URL = "https://example.test/new"
		p.UpdateEnabled = boolPtr(true)
		p.UpdateInterval = "6h"
		p.Interval = "6h"
		p.UpdateProxy = "core"
		p.UserAgent = "clashctl-test"
		p.ConvertMode = "force"
		p.Tags = []string{"home", "stable"}
		p.LastError = "previous failure"
		p.LastUpdated = "2026-07-02 10:00:00"
		p.NextUpdate = "2026-07-02 16:00:00"
		return nil
	})
	if err != nil {
		t.Fatalf("updateProfileMetadata: %v", err)
	}

	meta, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	got := meta.Profiles[0]
	if got.Name != "Renamed" || got.URL != "https://example.test/new" {
		t.Fatalf("profile identity not saved: %+v", got)
	}
	if got.UpdateEnabled == nil || !*got.UpdateEnabled {
		t.Fatalf("UpdateEnabled = %+v, want true", got.UpdateEnabled)
	}
	if got.UpdateInterval != "6h" || got.Interval != "6h" || got.UpdateProxy != "core" || got.UserAgent != "clashctl-test" || got.ConvertMode != "force" {
		t.Fatalf("profile policy not saved: %+v", got)
	}
	if strings.Join(got.Tags, ",") != "home,stable" || got.LastError == "" || got.LastUpdated == "" || got.NextUpdate == "" {
		t.Fatalf("profile metadata not saved: %+v", got)
	}
}

func TestUpdateProfileMetadataRejectsDuplicateSource(t *testing.T) {
	cfg := testProfileEditConfig(t)
	if err := addProfileForEditTest(cfg, 2, "https://example.test/two"); err != nil {
		t.Fatal(err)
	}

	err := updateProfileMetadata(cfg, 1, func(p *config.Profile) error {
		p.URL = "https://example.test/two"
		return nil
	})
	if err == nil || !strings.Contains(err.Error(), "subscription already exists") {
		t.Fatalf("updateProfileMetadata error = %v, want duplicate source", err)
	}
}

func TestUpdateProfileMetadataLeavesFileUnchangedWhenSaveFails(t *testing.T) {
	cfg := testProfileEditConfig(t)
	original, err := os.ReadFile(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	originalSaveProfiles := saveProfiles
	saveProfiles = func(string, *config.ProfilesMeta) error {
		return errors.New("forced save failure")
	}
	defer func() { saveProfiles = originalSaveProfiles }()

	err = updateProfileMetadata(cfg, 1, func(p *config.Profile) error {
		p.Name = "broken"
		return nil
	})
	if err == nil {
		t.Fatal("updateProfileMetadata succeeded, want error")
	}
	after, err := os.ReadFile(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	if string(after) != string(original) {
		t.Fatalf("profiles.yaml changed on save failure:\n%s", after)
	}
}

func TestScheduledUpdateRecordsLastErrorAndKeepsEnabled(t *testing.T) {
	cfg := testProfileEditConfig(t)
	oldCfg := cfgGlobalSwap(cfg)
	defer oldCfg()

	now := time.Date(2026, 7, 2, 12, 0, 0, 0, time.Local)
	oldTimeNow := timeNow
	timeNow = func() time.Time { return now }
	defer func() { timeNow = oldTimeNow }()

	err := updateProfileMetadata(cfg, 1, func(p *config.Profile) error {
		p.URL = "file://" + filepath.Join(cfg.ProfilesDir(), "missing.yaml")
		p.UpdateEnabled = boolPtr(true)
		p.UpdateInterval = "1h"
		p.Interval = "1h"
		p.Updated = "2026-07-02 10:00:00"
		p.NextUpdate = "2026-07-02 11:00:00"
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}

	err = updateScheduledSubscriptions()
	if err == nil || !strings.Contains(err.Error(), "scheduled update failed") {
		t.Fatalf("updateScheduledSubscriptions error = %v, want scheduled failure", err)
	}
	meta, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	got := meta.Profiles[0]
	if got.UpdateEnabled == nil || !*got.UpdateEnabled {
		t.Fatalf("UpdateEnabled = %+v, want still enabled", got.UpdateEnabled)
	}
	if got.LastError == "" || !strings.Contains(got.LastError, "open source") {
		t.Fatalf("LastError = %q, want download failure", got.LastError)
	}
	if got.NextUpdate != "2026-07-02 13:00:00" {
		t.Fatalf("NextUpdate = %q, want retry after interval", got.NextUpdate)
	}
}

func TestProfileDueForUpdateHonorsNextUpdate(t *testing.T) {
	p := config.Profile{
		URL:            "https://example.test/sub",
		UpdateEnabled:  boolPtr(true),
		UpdateInterval: "1h",
		Updated:        "2026-07-02 10:00:00",
		NextUpdate:     "2026-07-02 13:00:00",
	}
	now := time.Date(2026, 7, 2, 12, 0, 0, 0, time.Local)
	if profileDueForUpdate(p, now) {
		t.Fatal("profileDueForUpdate = true before NextUpdate")
	}
	if !profileDueForUpdate(p, now.Add(time.Hour)) {
		t.Fatal("profileDueForUpdate = false at NextUpdate")
	}
}

func testProfileEditConfig(t *testing.T) *config.EnvConfig {
	t.Helper()
	base := t.TempDir()
	cfg := &config.EnvConfig{
		ClashBaseDir: base,
		KernelName:   "missing-kernel",
		ClashSubUA:   "test",
	}
	if err := os.MkdirAll(cfg.ProfilesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := addProfileForEditTest(cfg, 1, "https://example.test/one"); err != nil {
		t.Fatal(err)
	}
	return cfg
}

func addProfileForEditTest(cfg *config.EnvConfig, id int, source string) error {
	path := filepath.Join(cfg.ProfilesDir(), strconv.Itoa(id)+".yaml")
	if err := os.WriteFile(path, []byte("proxies:\n  - name: old\n"), 0o644); err != nil {
		return err
	}
	meta, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		return err
	}
	if meta.Use == 0 {
		meta.Use = id
	}
	meta.Profiles = append(meta.Profiles, config.Profile{
		ID:      id,
		Path:    path,
		URL:     source,
		Name:    "Profile",
		Updated: "2026-07-02 10:00:00",
	})
	return config.SaveProfiles(cfg.ProfilesMeta(), meta)
}

func cfgGlobalSwap(next *config.EnvConfig) func() {
	old := cfg
	cfg = next
	return func() { cfg = old }
}
