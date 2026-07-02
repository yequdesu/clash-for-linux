package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestAddSubscriptionWithOptionsPersistsParameterizedFields(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{
		ClashBaseDir: base,
		KernelName:   "missing-kernel",
		ClashSubUA:   "default-ua",
	}
	if err := initConfig(cfg); err != nil {
		t.Fatal(err)
	}
	source := filepath.Join(base, "from-file.yaml")
	if err := os.WriteFile(source, []byte("proxies: []\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	now := time.Date(2026, 7, 2, 12, 0, 0, 0, time.Local)
	oldTimeNow := timeNow
	timeNow = func() time.Time { return now }
	defer func() { timeNow = oldTimeNow }()

	result, err := addSubscriptionWithOptions(cfg, source, addSubscriptionOptions{
		Name:           "Work",
		UpdateInterval: "6h",
		UpdateProxy:    "direct",
		UserAgent:      "custom-ua",
		ConvertMode:    "force",
		Tags:           []string{"work", "stable", " work "},
	})
	if err != nil {
		t.Fatalf("addSubscriptionWithOptions: %v", err)
	}
	if result.ID != 1 || !result.ActivateFirst {
		t.Fatalf("result = %+v, want first activated profile", result)
	}

	meta, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		t.Fatal(err)
	}
	if len(meta.Profiles) != 1 {
		t.Fatalf("profile count = %d, want 1", len(meta.Profiles))
	}
	got := meta.Profiles[0]
	if got.Name != "Work" || !strings.HasPrefix(got.URL, "file://") {
		t.Fatalf("profile identity = %+v", got)
	}
	if got.UpdateEnabled == nil || !*got.UpdateEnabled {
		t.Fatalf("UpdateEnabled = %+v, want true", got.UpdateEnabled)
	}
	if got.UpdateInterval != "6h" || got.Interval != "6h" || got.NextUpdate != "2026-07-02 18:00:00" {
		t.Fatalf("profile interval fields = %+v", got)
	}
	if got.UpdateProxy != "direct" || got.UserAgent != "custom-ua" || got.ConvertMode != "force" {
		t.Fatalf("profile policy fields = %+v", got)
	}
	if strings.Join(got.Tags, ",") != "work,stable" {
		t.Fatalf("tags = %+v, want deduplicated work/stable", got.Tags)
	}
}

func TestNormalizeAddSubscriptionOptionsRejectsInvalidValues(t *testing.T) {
	if _, err := normalizeAddSubscriptionOptions(addSubscriptionOptions{UpdateInterval: "soon"}); err == nil {
		t.Fatal("invalid interval accepted")
	}
	if _, err := normalizeAddSubscriptionOptions(addSubscriptionOptions{UpdateProxy: "vpn"}); err == nil {
		t.Fatal("invalid update proxy accepted")
	}
	if _, err := normalizeAddSubscriptionOptions(addSubscriptionOptions{ConvertMode: "maybe"}); err == nil {
		t.Fatal("invalid convert mode accepted")
	}

	options, err := normalizeAddSubscriptionOptions(addSubscriptionOptions{
		UpdateInterval: "off",
		UpdateProxy:    "AUTO",
		ConvertMode:    "Force",
		Tags:           []string{" daily ", "", "daily"},
	})
	if err != nil {
		t.Fatalf("normalizeAddSubscriptionOptions: %v", err)
	}
	if options.UpdateEnabled == nil || *options.UpdateEnabled {
		t.Fatalf("UpdateEnabled = %+v, want false for off", options.UpdateEnabled)
	}
	if options.UpdateInterval != "off" || options.UpdateProxy != "auto" || options.ConvertMode != "force" {
		t.Fatalf("normalized options = %+v", options)
	}
	if strings.Join(options.Tags, ",") != "daily" {
		t.Fatalf("tags = %+v, want daily", options.Tags)
	}
}
