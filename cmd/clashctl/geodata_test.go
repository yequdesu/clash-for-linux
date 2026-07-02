package main

import (
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestGeodataSourceBase(t *testing.T) {
	if got := geodataSourceBase(""); got != "https://github.com/MetaCubeX/meta-rules-dat/releases/latest/download" {
		t.Fatalf("geodataSourceBase(empty) = %q", got)
	}
	if got := geodataSourceBase("20250101"); got != "https://github.com/MetaCubeX/meta-rules-dat/releases/download/20250101" {
		t.Fatalf("geodataSourceBase(tag) = %q", got)
	}
}

func TestUpdateInstallStateGeodata(t *testing.T) {
	baseDir := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: baseDir}
	state := map[string]any{
		"schema_version": float64(1),
		"components": map[string]any{
			"mihomo": map[string]any{"path": "/tmp/mihomo"},
		},
	}
	data, err := json.Marshal(state)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(baseDir, "install-state.json"), data, 0o644); err != nil {
		t.Fatal(err)
	}

	if err := updateInstallStateGeodata(cfg, "latest", "https://example.test/latest"); err != nil {
		t.Fatal(err)
	}
	out, err := os.ReadFile(cfg.InstallState())
	if err != nil {
		t.Fatal(err)
	}
	var parsed map[string]any
	if err := json.Unmarshal(out, &parsed); err != nil {
		t.Fatal(err)
	}
	components := parsed["components"].(map[string]any)
	geodata := components["geodata"].(map[string]any)
	if geodata["version"] != "latest" {
		t.Fatalf("geodata version = %v", geodata["version"])
	}
	if geodata["path"] != cfg.ResourcesDir() {
		t.Fatalf("geodata path = %v, want %s", geodata["path"], cfg.ResourcesDir())
	}
}

func TestUpdateGeodataFromBaseDoesNotPartiallyUpdateOnDownloadFailure(t *testing.T) {
	baseDir := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: baseDir}
	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := seedInstallState(cfg); err != nil {
		t.Fatal(err)
	}
	for _, name := range geodataFileNames {
		if err := os.WriteFile(filepath.Join(cfg.ResourcesDir(), name), []byte("old-"+name), 0o644); err != nil {
			t.Fatal(err)
		}
	}

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		switch filepath.Base(r.URL.Path) {
		case "Country.mmdb":
			_, _ = w.Write([]byte("new-country"))
		case "geosite.dat":
			http.Error(w, "geosite unavailable", http.StatusInternalServerError)
		default:
			t.Fatalf("unexpected request: %s", r.URL.Path)
		}
	}))
	defer server.Close()

	err := updateGeodataFromBase(cfg, "test", server.URL)
	if err == nil {
		t.Fatal("expected geodata update failure")
	}
	for _, name := range geodataFileNames {
		got, readErr := os.ReadFile(filepath.Join(cfg.ResourcesDir(), name))
		if readErr != nil {
			t.Fatal(readErr)
		}
		if string(got) != "old-"+name {
			t.Fatalf("%s = %q, want old content", name, got)
		}
	}
}

func TestUpdateGeodataFromBaseReplacesAllFilesOnSuccess(t *testing.T) {
	baseDir := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: baseDir}
	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := seedInstallState(cfg); err != nil {
		t.Fatal(err)
	}

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		name := filepath.Base(r.URL.Path)
		for _, allowed := range geodataFileNames {
			if name == allowed {
				_, _ = fmt.Fprintf(w, "new-%s", name)
				return
			}
		}
		t.Fatalf("unexpected request: %s", r.URL.Path)
	}))
	defer server.Close()

	if err := updateGeodataFromBase(cfg, "test", server.URL); err != nil {
		t.Fatal(err)
	}
	for _, name := range geodataFileNames {
		got, err := os.ReadFile(filepath.Join(cfg.ResourcesDir(), name))
		if err != nil {
			t.Fatal(err)
		}
		if string(got) != "new-"+name {
			t.Fatalf("%s = %q, want new content", name, got)
		}
	}
}

func TestUpdateGeodataFromBaseRollsBackWhenInstallStateFails(t *testing.T) {
	baseDir := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: baseDir}
	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	for _, name := range geodataFileNames {
		if err := os.WriteFile(filepath.Join(cfg.ResourcesDir(), name), []byte("old-"+name), 0o644); err != nil {
			t.Fatal(err)
		}
	}

	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		name := filepath.Base(r.URL.Path)
		for _, allowed := range geodataFileNames {
			if name == allowed {
				_, _ = fmt.Fprintf(w, "new-%s", name)
				return
			}
		}
		t.Fatalf("unexpected request: %s", r.URL.Path)
	}))
	defer server.Close()

	err := updateGeodataFromBase(cfg, "test", server.URL)
	if err == nil {
		t.Fatal("expected install-state failure")
	}
	for _, name := range geodataFileNames {
		got, readErr := os.ReadFile(filepath.Join(cfg.ResourcesDir(), name))
		if readErr != nil {
			t.Fatal(readErr)
		}
		if string(got) != "old-"+name {
			t.Fatalf("%s = %q, want old content after rollback", name, got)
		}
	}
}

func seedInstallState(cfg *config.EnvConfig) error {
	state := map[string]any{"schema_version": 1, "components": map[string]any{}}
	data, err := json.Marshal(state)
	if err != nil {
		return err
	}
	return os.WriteFile(cfg.InstallState(), data, 0o644)
}
