package main

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
	"gopkg.in/yaml.v3"
)

func TestSnapshotFilesRestoreSnapshots(t *testing.T) {
	dir := t.TempDir()
	existing := filepath.Join(dir, "mixin.yaml")
	missing := filepath.Join(dir, "runtime.yaml")
	if err := os.WriteFile(existing, []byte("original"), 0644); err != nil {
		t.Fatalf("write original: %v", err)
	}

	snapshots, err := snapshotFiles(existing, missing)
	if err != nil {
		t.Fatalf("snapshotFiles: %v", err)
	}

	if err := os.WriteFile(existing, []byte("changed"), 0644); err != nil {
		t.Fatalf("write changed: %v", err)
	}
	if err := os.WriteFile(missing, []byte("new"), 0644); err != nil {
		t.Fatalf("write missing replacement: %v", err)
	}

	restoreSnapshots(snapshots)

	data, err := os.ReadFile(existing)
	if err != nil {
		t.Fatalf("read restored existing: %v", err)
	}
	if string(data) != "original" {
		t.Fatalf("restored existing = %q, want original", data)
	}
	if _, err := os.Stat(missing); !os.IsNotExist(err) {
		t.Fatalf("missing file should have been removed, err=%v", err)
	}
}

func TestSetTunEnabledUpdatesNestedYAML(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: base}
	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.MixinPath(), []byte("mixed-port: 7890\ntun:\n  enable: false\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	if err := setTunEnabled(cfg, true); err != nil {
		t.Fatalf("setTunEnabled: %v", err)
	}
	if got := readTunEnabled(t, cfg.MixinPath()); !got {
		t.Fatal("tun.enable = false, want true")
	}
}

func TestSetTunEnabledCreatesNestedYAML(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: base}

	if err := setTunEnabled(cfg, false); err != nil {
		t.Fatalf("setTunEnabled: %v", err)
	}
	if got := readTunEnabled(t, cfg.MixinPath()); got {
		t.Fatal("tun.enable = true, want false")
	}
}

func TestParseIPTunTapDevice(t *testing.T) {
	out := []byte("tun0: tun one_queue pi off vnet_hdr off persist off\n")

	if got := parseIPTunTapDevice(out); got != "tun0" {
		t.Fatalf("parseIPTunTapDevice = %q, want tun0", got)
	}
}

func TestParseIPLinkJSONDeviceUsesStructuredKind(t *testing.T) {
	out := []byte(`[
  {"ifname":"eth0","flags":["BROADCAST","MULTICAST"],"linkinfo":{"info_kind":"veth"}},
  {"ifname":"mihomo0","flags":["POINTOPOINT","NOARP"],"linkinfo":{"info_kind":"tun"}}
]`)

	if got := parseIPLinkJSONDevice(out); got != "mihomo0" {
		t.Fatalf("parseIPLinkJSONDevice = %q, want mihomo0", got)
	}
}

func TestParseIPLinkJSONDeviceRequiresTunSignalsForNameFallback(t *testing.T) {
	out := []byte(`[
  {"ifname":"tun-history","flags":["BROADCAST","MULTICAST"]},
  {"ifname":"utun","flags":["POINTOPOINT","NOARP"]}
]`)

	if got := parseIPLinkJSONDevice(out); got != "utun" {
		t.Fatalf("parseIPLinkJSONDevice = %q, want utun", got)
	}
}

func TestParseIPLinkTextDeviceRequiresPointToPointTun(t *testing.T) {
	out := []byte(`2: eth0: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 state UP
9: utun: <POINTOPOINT,MULTICAST,NOARP,UP,LOWER_UP> mtu 9000 state UNKNOWN
`)

	if got := parseIPLinkTextDevice(out); got != "utun" {
		t.Fatalf("parseIPLinkTextDevice = %q, want utun", got)
	}
}

func TestParseIPLinkTextDeviceRejectsNameOnlyMatch(t *testing.T) {
	out := []byte(`7: tunnel-metrics: <BROADCAST,MULTICAST,UP,LOWER_UP> mtu 1500 state UP
`)

	if got := parseIPLinkTextDevice(out); got != "" {
		t.Fatalf("parseIPLinkTextDevice = %q, want empty", got)
	}
}

func TestChangeTunModeRestoresConfigWhenStopFails(t *testing.T) {
	base := t.TempDir()
	cfg := &config.EnvConfig{ClashBaseDir: base}
	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	originalMixin := []byte("mixed-port: 7890\ntun:\n  enable: true\n")
	originalRuntime := []byte("mixed-port: 7890\ntun:\n  enable: true\n")
	if err := os.WriteFile(cfg.MixinPath(), originalMixin, 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.RuntimePath(), originalRuntime, 0o644); err != nil {
		t.Fatal(err)
	}

	service := &fakeTunService{running: true, stopErr: errors.New("forced stop failure")}
	oldNewTunService := newTunService
	newTunService = func(*config.EnvConfig) tunService {
		return service
	}
	defer func() { newTunService = oldNewTunService }()

	oldMergeTunConfig := mergeTunConfig
	mergeTunConfig = func(cfg *config.EnvConfig, _ bool) error {
		return config.AtomicWriteFile(cfg.RuntimePath(), []byte("mixed-port: 7999\n"), 0o644)
	}
	defer func() { mergeTunConfig = oldMergeTunConfig }()

	err := changeTunMode(cfg, false)
	if err == nil || !strings.Contains(err.Error(), "stop current kernel") {
		t.Fatalf("changeTunMode error = %v, want stop current kernel", err)
	}
	if !service.stopCalled {
		t.Fatal("service Stop was not called")
	}
	if service.startCalled {
		t.Fatal("service Start should not be called after stop failure")
	}
	assertFileContent(t, cfg.MixinPath(), originalMixin)
	assertFileContent(t, cfg.RuntimePath(), originalRuntime)
}

func readTunEnabled(t *testing.T, path string) bool {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var parsed struct {
		Tun struct {
			Enable bool `yaml:"enable"`
		} `yaml:"tun"`
	}
	if err := yaml.Unmarshal(data, &parsed); err != nil {
		t.Fatal(err)
	}
	return parsed.Tun.Enable
}

type fakeTunService struct {
	running     bool
	stopErr     error
	stopCalled  bool
	startCalled bool
}

func (f *fakeTunService) IsRunning() bool {
	return f.running
}

func (f *fakeTunService) Stop() error {
	f.stopCalled = true
	return f.stopErr
}

func (f *fakeTunService) Start() error {
	f.startCalled = true
	return nil
}

func assertFileContent(t *testing.T, path string, want []byte) {
	t.Helper()
	got, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	if string(got) != string(want) {
		t.Fatalf("%s = %q, want %q", path, got, want)
	}
}
