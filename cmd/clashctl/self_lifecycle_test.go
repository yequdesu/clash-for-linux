package main

import (
	"os"
	"path/filepath"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestNormalizeShellDetectsSupportedShells(t *testing.T) {
	t.Setenv("SHELL", "/usr/bin/zsh")
	if got := normalizeShell(""); got != "zsh" {
		t.Fatalf("normalizeShell from env = %q, want zsh", got)
	}
	if got := normalizeShell("pwsh"); got != "powershell" {
		t.Fatalf("normalizeShell pwsh = %q, want powershell", got)
	}
	if got := normalizeShell("cmd.exe"); got != "" {
		t.Fatalf("unsupported shell = %q, want empty", got)
	}
}

func TestRemoveSafePathRejectsDangerousPaths(t *testing.T) {
	for _, path := range []string{"/", ".", userHomeDir()} {
		if err := removeSafePath(path); err == nil {
			t.Fatalf("removeSafePath(%q) succeeded, want refusal", path)
		}
	}
}

func TestRemoveSafePathAllowsOwnedTempPath(t *testing.T) {
	dir := filepath.Join(t.TempDir(), "clashctl-data")
	if err := os.MkdirAll(dir, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := removeSafePath(dir); err != nil {
		t.Fatalf("removeSafePath temp dir: %v", err)
	}
	if _, err := os.Stat(dir); !os.IsNotExist(err) {
		t.Fatalf("temp dir still exists or unexpected error: %v", err)
	}
}

func TestReleaseDownloadURLsHonorsMirrorOnlyMode(t *testing.T) {
	t.Setenv("CLASHCTL_GITHUB_DIRECT", "false")
	t.Setenv("CLASHCTL_GITHUB_MIRRORS", "https://mirror.example")
	oldCfg := cfg
	cfg = &config.EnvConfig{}
	defer func() { cfg = oldCfg }()
	cfg.URLGhProxy = ""
	got := releaseDownloadURLs("https://github.com/owner/repo/releases/latest/download", "asset.tar.gz")
	want := "https://mirror.example/https://github.com/owner/repo/releases/latest/download/asset.tar.gz"
	if len(got) != 1 || got[0] != want {
		t.Fatalf("releaseDownloadURLs = %#v, want [%q]", got, want)
	}
}
