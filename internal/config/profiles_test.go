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
