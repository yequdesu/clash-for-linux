package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestRuntimeInfoAPIHelpers(t *testing.T) {
	info := runtimeInfo{apiHost: "::1", apiPort: "9090"}
	if got := info.apiAddress(); got != "[::1]:9090" {
		t.Fatalf("apiAddress = %q, want [::1]:9090", got)
	}
	if got := info.apiBaseURL(); got != "http://[::1]:9090" {
		t.Fatalf("apiBaseURL = %q, want http://[::1]:9090", got)
	}
}

func TestRuntimeInfoAPIHelpersDefaultAndUnspecified(t *testing.T) {
	info := runtimeInfo{apiHost: "0.0.0.0"}
	if got := info.apiAddress(); got != "127.0.0.1:9090" {
		t.Fatalf("apiAddress = %q, want 127.0.0.1:9090", got)
	}
}

func TestStatusSubscriptionSummaryReportsProfilesError(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: base}
	if err := os.MkdirAll(cfg.ProfilesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.ProfilesMeta(), []byte("profiles: ["), 0o644); err != nil {
		t.Fatal(err)
	}

	if _, err := statusSubscriptionSummary(cfg); err == nil {
		t.Fatal("statusSubscriptionSummary error = nil, want parse error")
	}
}

func TestStatusSubscriptionSummaryActiveProfile(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: base}
	if err := os.MkdirAll(cfg.ProfilesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	profilePath := filepath.Join(cfg.ProfilesDir(), "1.yaml")
	meta := &config.ProfilesMeta{
		Use: 1,
		Profiles: []config.Profile{{
			ID:   1,
			Path: profilePath,
			URL:  "https://example.com/very-long-subscription.yaml",
		}},
	}
	if err := config.SaveProfiles(cfg.ProfilesMeta(), meta); err != nil {
		t.Fatal(err)
	}

	got, err := statusSubscriptionSummary(cfg)
	if err != nil {
		t.Fatalf("statusSubscriptionSummary: %v", err)
	}
	if !strings.Contains(got, "subscriptions: 1 active ([1]") {
		t.Fatalf("summary = %q, want active subscription", got)
	}
}
