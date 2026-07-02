package main

import (
	"errors"
	"os"
	"path/filepath"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestRemoveSubscriptionRemovesInactiveProfile(t *testing.T) {
	cfg, profilePath := testRemoveProfileConfig(t)

	removed, err := removeSubscription(cfg, 2)
	if err != nil {
		t.Fatalf("removeSubscription: %v", err)
	}
	if removed.ID != 2 {
		t.Fatalf("removed ID = %d, want 2", removed.ID)
	}
	if _, err := os.Stat(profilePath); !os.IsNotExist(err) {
		t.Fatalf("profile file still exists or stat failed unexpectedly: %v", err)
	}
	meta, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	if len(meta.Profiles) != 1 || meta.Profiles[0].ID != 1 {
		t.Fatalf("profiles = %+v, want only active profile 1", meta.Profiles)
	}
}

func TestRemoveSubscriptionRejectsActiveProfile(t *testing.T) {
	cfg, _ := testRemoveProfileConfig(t)

	if _, err := removeSubscription(cfg, 1); err == nil {
		t.Fatal("removeSubscription removed active profile, want error")
	}
}

func TestRemoveSubscriptionRestoresProfileWhenMetadataSaveFails(t *testing.T) {
	cfg, profilePath := testRemoveProfileConfig(t)
	originalProfile, err := os.ReadFile(profilePath)
	if err != nil {
		t.Fatal(err)
	}
	originalMeta, err := os.ReadFile(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	originalSaveProfiles := saveProfiles
	saveProfiles = func(string, *config.ProfilesMeta) error {
		return errors.New("forced metadata failure")
	}
	defer func() { saveProfiles = originalSaveProfiles }()

	if _, err := removeSubscription(cfg, 2); err == nil {
		t.Fatal("removeSubscription succeeded, want metadata save failure")
	}
	restoredProfile, err := os.ReadFile(profilePath)
	if err != nil {
		t.Fatal(err)
	}
	if string(restoredProfile) != string(originalProfile) {
		t.Fatalf("profile data = %q, want %q", restoredProfile, originalProfile)
	}
	metaData, err := os.ReadFile(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	if string(metaData) != string(originalMeta) {
		t.Fatalf("profiles.yaml changed on failure:\n%s", metaData)
	}
}

func testRemoveProfileConfig(t *testing.T) (*config.EnvConfig, string) {
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
	activePath := filepath.Join(cfg.ProfilesDir(), "1.yaml")
	if err := os.WriteFile(activePath, []byte("proxies:\n  - name: active\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	removePath := filepath.Join(cfg.ProfilesDir(), "2.yaml")
	if err := os.WriteFile(removePath, []byte("proxies:\n  - name: remove\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	meta := &config.ProfilesMeta{
		Use: 1,
		Profiles: []config.Profile{
			{ID: 1, Path: activePath, URL: "file://active.yaml"},
			{ID: 2, Path: removePath, URL: "file://remove.yaml"},
		},
	}
	if err := config.SaveProfiles(cfg.ProfilesMeta(), meta); err != nil {
		t.Fatal(err)
	}
	return cfg, removePath
}
