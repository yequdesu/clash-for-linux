package main

import (
	"os"
	"path/filepath"
	"strings"
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

func TestUsingDefaultLatestReleaseRequiresNoExplicitSource(t *testing.T) {
	oldBase := selfUpdateBaseURL
	oldTag := selfUpdateTag
	defer func() {
		selfUpdateBaseURL = oldBase
		selfUpdateTag = oldTag
	}()
	selfUpdateBaseURL = ""
	selfUpdateTag = ""
	t.Setenv("CLASHCTL_RELEASE_BASE_URL", "")
	t.Setenv("CLASHCTL_RELEASE_TAG", "")
	if !usingDefaultLatestRelease() {
		t.Fatal("default latest was not detected")
	}
	selfUpdateTag = "v0.2.0-rc.12"
	if usingDefaultLatestRelease() {
		t.Fatal("explicit tag should disable default latest mode")
	}
}

func TestActiveShellProxyEnvNamesIncludesSavedSudoMarker(t *testing.T) {
	for _, key := range []string{"http_proxy", "HTTP_PROXY", "https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY", "no_proxy", "NO_PROXY", "CLASHCTL_UNINSTALL_PROXY_ENV_NAMES"} {
		t.Setenv(key, "")
	}
	t.Setenv("https_proxy", "http://127.0.0.1:7897")
	t.Setenv("CLASHCTL_UNINSTALL_PROXY_ENV_NAMES", "http_proxy,https_proxy")
	got := activeShellProxyEnvNames()
	for _, want := range []string{"https_proxy", "http_proxy"} {
		found := false
		for _, name := range got {
			if name == want {
				found = true
				break
			}
		}
		if !found {
			t.Fatalf("activeShellProxyEnvNames = %#v, missing %s", got, want)
		}
	}
}

func TestProxyUnsetCommandContainsAllProxyVariables(t *testing.T) {
	cmd := proxyUnsetCommand()
	for _, key := range []string{"http_proxy", "HTTP_PROXY", "https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY", "no_proxy", "NO_PROXY"} {
		if !strings.Contains(cmd, key) {
			t.Fatalf("proxyUnsetCommand missing %s: %s", key, cmd)
		}
	}
}
