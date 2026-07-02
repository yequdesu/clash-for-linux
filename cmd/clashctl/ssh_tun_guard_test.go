package main

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestTunStartCanCaptureRoutes(t *testing.T) {
	if !tunStartCanCaptureRoutes(config.RuntimeInfo{TunEnabled: true, TunAutoRoute: true}, false) {
		t.Fatal("auto-route TUN should be treated as route-capturing")
	}
	if !tunStartCanCaptureRoutes(config.RuntimeInfo{TunEnabled: true, TunStrictRoute: true}, false) {
		t.Fatal("strict-route TUN should be treated as route-capturing")
	}
	if tunStartCanCaptureRoutes(config.RuntimeInfo{TunEnabled: true}, false) {
		t.Fatal("TUN without auto-route/strict-route should not be treated as route-capturing")
	}
	if !tunStartCanCaptureRoutes(config.RuntimeInfo{}, true) {
		t.Fatal("forced TUN enable should be treated as route-capturing")
	}
}

func TestEnsureSafeKernelStartFromSSHBlocksRiskyTun(t *testing.T) {
	base := t.TempDir()
	resources := filepath.Join(base, "resources")
	if err := os.MkdirAll(resources, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(resources, "runtime.yaml"), []byte(`
mixed-port: 7897
tun:
  enable: true
  auto-route: true
`), 0o644); err != nil {
		t.Fatal(err)
	}

	t.Setenv("SSH_TTY", "/dev/pts/1")
	err := ensureSafeKernelStartFromSSH(&config.EnvConfig{ClashBaseDir: base}, false, false)
	if err == nil {
		t.Fatal("expected SSH TUN guard error")
	}
	if !strings.Contains(err.Error(), "refusing to start TUN auto-route") {
		t.Fatalf("unexpected error: %v", err)
	}
}

func TestEnsureSafeKernelStartFromSSHAllowsExplicitOverride(t *testing.T) {
	base := t.TempDir()
	resources := filepath.Join(base, "resources")
	if err := os.MkdirAll(resources, 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(filepath.Join(resources, "runtime.yaml"), []byte(`
tun:
  enable: true
  strict-route: true
`), 0o644); err != nil {
		t.Fatal(err)
	}

	t.Setenv("SSH_TTY", "/dev/pts/1")
	if err := ensureSafeKernelStartFromSSH(&config.EnvConfig{ClashBaseDir: base}, true, false); err != nil {
		t.Fatalf("override should allow risky start: %v", err)
	}
}
