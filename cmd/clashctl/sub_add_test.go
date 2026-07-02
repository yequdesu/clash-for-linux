package main

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestHasProtocol(t *testing.T) {
	tests := []struct {
		name string
		in   string
		want bool
	}{
		{name: "empty", in: "", want: false},
		{name: "short", in: "https:/", want: false},
		{name: "http", in: "http://example.com/sub", want: true},
		{name: "https", in: "https://example.com/sub", want: true},
		{name: "file", in: "file:///tmp/config.yaml", want: true},
		{name: "path", in: "/tmp/config.yaml", want: false},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := hasProtocol(tt.in); got != tt.want {
				t.Fatalf("hasProtocol(%q) = %v, want %v", tt.in, got, tt.want)
			}
		})
	}
}

func TestAddSubscriptionWritesProfileMetadata(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{
		ClashBaseDir: base,
		KernelName:   "missing-kernel",
		ClashSubUA:   "test",
	}
	sourcePath := filepath.Join(base, "configs", "my-sub.yaml")
	if err := os.MkdirAll(filepath.Dir(sourcePath), 0o755); err != nil {
		t.Fatal(err)
	}
	sourceData := []byte("proxies: []\n")
	if err := os.WriteFile(sourcePath, sourceData, 0o644); err != nil {
		t.Fatal(err)
	}

	result, err := addSubscription(cfg, sourcePath)
	if err != nil {
		t.Fatalf("addSubscription: %v", err)
	}
	if result.ID != 1 {
		t.Fatalf("ID = %d, want 1", result.ID)
	}
	if !result.ActivateFirst {
		t.Fatal("ActivateFirst = false, want true")
	}
	if result.Source != "file://"+sourcePath {
		t.Fatalf("Source = %q, want file://%s", result.Source, sourcePath)
	}

	meta, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		t.Fatalf("LoadProfiles: %v", err)
	}
	if len(meta.Profiles) != 1 {
		t.Fatalf("profiles length = %d, want 1", len(meta.Profiles))
	}
	profile := meta.Profiles[0]
	if profile.Name != "my-sub" {
		t.Fatalf("Name = %q, want my-sub", profile.Name)
	}
	if profile.Updated == "" {
		t.Fatal("Updated is empty")
	}
	written, err := os.ReadFile(profile.Path)
	if err != nil {
		t.Fatalf("read profile: %v", err)
	}
	if string(written) != string(sourceData) {
		t.Fatalf("profile content = %q, want %q", written, sourceData)
	}
}

func TestAddSubscriptionRejectsDuplicateSource(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{
		ClashBaseDir: base,
		KernelName:   "missing-kernel",
		ClashSubUA:   "test",
	}
	sourcePath := filepath.Join(base, "dup.yaml")
	if err := os.WriteFile(sourcePath, []byte("proxies: []\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	if _, err := addSubscription(cfg, sourcePath); err != nil {
		t.Fatalf("first addSubscription: %v", err)
	}
	if _, err := addSubscription(cfg, sourcePath); err == nil {
		t.Fatal("second addSubscription succeeded, want duplicate error")
	}
	meta, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		t.Fatalf("LoadProfiles: %v", err)
	}
	if len(meta.Profiles) != 1 {
		t.Fatalf("profiles length = %d, want 1", len(meta.Profiles))
	}
}

func TestAddSubscriptionRejectsDuplicateRelativeAndAbsolutePath(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{
		ClashBaseDir: base,
		KernelName:   "missing-kernel",
		ClashSubUA:   "test",
	}
	sourcePath := filepath.Join(base, "relative-dup.yaml")
	if err := os.WriteFile(sourcePath, []byte("proxies: []\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	oldWD, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	if err := os.Chdir(base); err != nil {
		t.Fatal(err)
	}
	defer func() {
		if err := os.Chdir(oldWD); err != nil {
			t.Fatal(err)
		}
	}()

	if _, err := addSubscription(cfg, "relative-dup.yaml"); err != nil {
		t.Fatalf("relative addSubscription: %v", err)
	}
	if _, err := addSubscription(cfg, sourcePath); err == nil {
		t.Fatal("absolute addSubscription succeeded, want duplicate error")
	}
	meta, err := config.LoadProfiles(cfg.ProfilesMeta())
	if err != nil {
		t.Fatalf("LoadProfiles: %v", err)
	}
	if len(meta.Profiles) != 1 {
		t.Fatalf("profiles length = %d, want 1", len(meta.Profiles))
	}
	if !strings.HasPrefix(meta.Profiles[0].URL, "file://") || !filepath.IsAbs(strings.TrimPrefix(meta.Profiles[0].URL, "file://")) {
		t.Fatalf("profile URL is not canonical absolute file URL: %q", meta.Profiles[0].URL)
	}
}

func TestAddSubscriptionCleansProfileFileWhenMetadataSaveFails(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{
		ClashBaseDir: base,
		KernelName:   "missing-kernel",
		ClashSubUA:   "test",
	}
	sourcePath := filepath.Join(base, "bad-meta.yaml")
	if err := os.WriteFile(sourcePath, []byte("proxies: []\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	originalSaveProfiles := saveProfiles
	saveProfiles = func(string, *config.ProfilesMeta) error {
		return errors.New("forced metadata failure")
	}
	defer func() { saveProfiles = originalSaveProfiles }()

	if _, err := addSubscription(cfg, sourcePath); err == nil {
		t.Fatal("addSubscription succeeded, want metadata save failure")
	}
	if _, err := os.Stat(filepath.Join(cfg.ProfilesDir(), "1.yaml")); !os.IsNotExist(err) {
		t.Fatalf("orphan profile file still exists or stat failed unexpectedly: %v", err)
	}
}

func TestProfileNameFromSource(t *testing.T) {
	tests := map[string]string{
		"file:///tmp/HK.yaml":                    "HK",
		"https://example.com/subscriptions/main": "main",
		"https://example.com/":                   "example.com",
		"plain.yml":                              "plain",
	}
	for source, want := range tests {
		if got := profileNameFromSource(source); got != want {
			t.Fatalf("profileNameFromSource(%q) = %q, want %q", source, got, want)
		}
	}
}

func TestNormalizeSubscriptionSourceCanonicalizesFileURL(t *testing.T) {
	base := t.TempDir()
	sourcePath := filepath.Join(base, "canonical.yaml")
	if err := os.WriteFile(sourcePath, []byte("proxies: []\n"), 0o644); err != nil {
		t.Fatal(err)
	}
	got := normalizeSubscriptionSource("file://" + filepath.Join(base, ".", "canonical.yaml"))
	want := "file://" + filepath.Clean(sourcePath)
	if got != want {
		t.Fatalf("normalizeSubscriptionSource = %q, want %q", got, want)
	}
}
