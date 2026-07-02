package main

import (
	"errors"
	"os"
	"path/filepath"
	"strings"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestConfigCommandHelpers(t *testing.T) {
	if port, err := parsePortArg("7890"); err != nil || port != 7890 {
		t.Fatalf("parsePortArg = %d, %v", port, err)
	}
	if _, err := parsePortArg("70000"); err == nil {
		t.Fatal("parsePortArg accepted out-of-range port")
	}
	if err := validateController("127.0.0.1:9090", false); err != nil {
		t.Fatalf("validateController loopback: %v", err)
	}
	if err := validateController("0.0.0.0:9090", false); err == nil {
		t.Fatal("validateController accepted unsafe controller")
	}
	if err := validateController("0.0.0.0:9090", true); err != nil {
		t.Fatalf("validateController allow unsafe: %v", err)
	}
	if on, err := parseOnOff("on"); err != nil || !on {
		t.Fatalf("parseOnOff(on) = %v, %v", on, err)
	}
	if off, err := parseOnOff("disabled"); err != nil || off {
		t.Fatalf("parseOnOff(disabled) = %v, %v", off, err)
	}
	if !hasCommonFakeIPFilters(defaultFakeIPFilter()) {
		t.Fatal("defaultFakeIPFilter lacks common LAN filters")
	}
}

func TestApplyMixinUpdatesRestoresFilesWhenMergeFails(t *testing.T) {
	base := t.TempDir()
	oldCfg := cfg
	cfg = &config.EnvConfig{ClashBaseDir: base, KernelName: "missing-kernel"}
	defer func() { cfg = oldCfg }()
	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	originalMixin := []byte("mixed-port: 7890\n")
	originalRuntime := []byte("mixed-port: 7890\n")
	if err := os.WriteFile(cfg.MixinPath(), originalMixin, 0o644); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.RuntimePath(), originalRuntime, 0o644); err != nil {
		t.Fatal(err)
	}

	originalMerge := mergeMixinConfig
	mergeMixinConfig = func(*config.EnvConfig, bool) error {
		return errors.New("forced merge failure")
	}
	defer func() { mergeMixinConfig = originalMerge }()

	err := applyMixinUpdates(map[string]any{"mixed-port": 7999})
	if err == nil || !strings.Contains(err.Error(), "restored previous config") {
		t.Fatalf("applyMixinUpdates error = %v, want restored previous config", err)
	}
	mixinData, err := os.ReadFile(cfg.MixinPath())
	if err != nil {
		t.Fatal(err)
	}
	if string(mixinData) != string(originalMixin) {
		t.Fatalf("mixin = %q, want %q", mixinData, originalMixin)
	}
	runtimeData, err := os.ReadFile(cfg.RuntimePath())
	if err != nil {
		t.Fatal(err)
	}
	if string(runtimeData) != string(originalRuntime) {
		t.Fatalf("runtime = %q, want %q", runtimeData, originalRuntime)
	}
}

func TestApplyMixinUpdatesKeepsChangesWhenMergeSucceeds(t *testing.T) {
	base := t.TempDir()
	oldCfg := cfg
	cfg = &config.EnvConfig{ClashBaseDir: base, KernelName: "missing-kernel"}
	defer func() { cfg = oldCfg }()
	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.MixinPath(), []byte("mixed-port: 7890\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	originalMerge := mergeMixinConfig
	mergeMixinConfig = func(cfg *config.EnvConfig, _ bool) error {
		return config.AtomicWriteFile(cfg.RuntimePath(), []byte("mixed-port: 7999\n"), 0o644)
	}
	defer func() { mergeMixinConfig = originalMerge }()

	if err := applyMixinUpdates(map[string]any{"mixed-port": 7999}); err != nil {
		t.Fatalf("applyMixinUpdates: %v", err)
	}
	mixinData, err := os.ReadFile(cfg.MixinPath())
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(mixinData), "mixed-port: 7999") {
		t.Fatalf("mixin was not updated:\n%s", mixinData)
	}
	if _, err := os.Stat(filepath.Join(cfg.ResourcesDir(), "runtime.yaml")); err != nil {
		t.Fatalf("runtime was not written: %v", err)
	}
}
