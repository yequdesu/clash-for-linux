package config

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestLoadProfilesMissingFileReturnsEmptyMeta(t *testing.T) {
	meta, err := LoadProfiles(filepath.Join(t.TempDir(), "profiles.yaml"))
	if err != nil {
		t.Fatalf("LoadProfiles missing file error = %v", err)
	}
	if meta == nil || len(meta.Profiles) != 0 || meta.Use != 0 {
		t.Fatalf("meta = %+v, want empty", meta)
	}
}

func TestLoadProfilesReturnsReadError(t *testing.T) {
	path := filepath.Join(t.TempDir(), "profiles.yaml")
	if err := os.Mkdir(path, 0o755); err != nil {
		t.Fatal(err)
	}
	meta, err := LoadProfiles(path)
	if err == nil || !strings.Contains(err.Error(), "read profiles") {
		t.Fatalf("LoadProfiles read error = %v, want read profiles", err)
	}
	if meta == nil {
		t.Fatal("meta is nil, want empty meta with error")
	}
}

func TestLoadProfilesReturnsParseError(t *testing.T) {
	path := filepath.Join(t.TempDir(), "profiles.yaml")
	if err := os.WriteFile(path, []byte("profiles: [\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	_, err := LoadProfiles(path)
	if err == nil || !strings.Contains(err.Error(), "parse profiles") {
		t.Fatalf("LoadProfiles parse error = %v, want parse profiles", err)
	}
}

func TestLoadProfilesSupportsParameterizedSubscriptionMetadata(t *testing.T) {
	path := filepath.Join(t.TempDir(), "profiles.yaml")
	data := []byte(`
use: 1
profiles:
  - id: 1
    path: /tmp/1.yaml
    url: https://example.test/sub
    name: main
    interval: 12h
    update_enabled: true
    update_interval: 6h
    update_proxy: core
    user_agent: clashctl-test
    convert_mode: force
    tags: [home, stable]
    last_error: ""
    last_updated: "2026-07-02 10:00:00"
    next_update: "2026-07-02 16:00:00"
`)
	if err := os.WriteFile(path, data, 0o644); err != nil {
		t.Fatal(err)
	}
	meta, err := LoadProfiles(path)
	if err != nil {
		t.Fatal(err)
	}
	p := meta.Profiles[0]
	if p.UpdateEnabled == nil || !*p.UpdateEnabled {
		t.Fatalf("UpdateEnabled = %+v, want true", p.UpdateEnabled)
	}
	if p.UpdateInterval != "6h" || p.Interval != "12h" || p.UpdateProxy != "core" || p.UserAgent != "clashctl-test" || p.ConvertMode != "force" {
		t.Fatalf("profile metadata = %+v", p)
	}
	if strings.Join(p.Tags, ",") != "home,stable" || p.LastUpdated == "" || p.NextUpdate == "" {
		t.Fatalf("profile metadata = %+v", p)
	}
}
