package main

import (
	"errors"
	"os"
	"strings"
	"testing"

	"github.com/yequdesu/linux-cli-tui-clash/internal/config"
)

func TestSetAPISecretRestoresFilesWhenMergeFails(t *testing.T) {
	base := t.TempDir()
	oldCfg := cfg
	cfg = &config.EnvConfig{ClashBaseDir: base, KernelName: "missing-kernel"}
	defer func() { cfg = oldCfg }()

	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	originalMixin := []byte("secret: old-secret\nmixed-port: 7890\n")
	originalRuntime := []byte("secret: old-secret\nmixed-port: 7890\n")
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

	err := setAPISecret("new-secret")
	if err == nil || !strings.Contains(err.Error(), "restored previous config") {
		t.Fatalf("setAPISecret error = %v, want restored previous config", err)
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

func TestSetAPISecretUpdatesSecretWhenMergeSucceeds(t *testing.T) {
	base := t.TempDir()
	oldCfg := cfg
	cfg = &config.EnvConfig{ClashBaseDir: base, KernelName: "missing-kernel"}
	defer func() { cfg = oldCfg }()

	if err := os.MkdirAll(cfg.ResourcesDir(), 0o755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(cfg.MixinPath(), []byte("secret: old-secret\nmixed-port: 7890\n"), 0o644); err != nil {
		t.Fatal(err)
	}

	originalMerge := mergeMixinConfig
	mergeMixinConfig = func(cfg *config.EnvConfig, _ bool) error {
		return config.AtomicWriteFile(cfg.RuntimePath(), []byte("secret: new-secret\nmixed-port: 7890\n"), 0o644)
	}
	defer func() { mergeMixinConfig = originalMerge }()

	if err := setAPISecret("new-secret"); err != nil {
		t.Fatalf("setAPISecret: %v", err)
	}
	mixinData, err := os.ReadFile(cfg.MixinPath())
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(mixinData), "secret: new-secret") {
		t.Fatalf("mixin was not updated:\n%s", mixinData)
	}
	runtimeData, err := os.ReadFile(cfg.RuntimePath())
	if err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(string(runtimeData), "secret: new-secret") {
		t.Fatalf("runtime was not updated:\n%s", runtimeData)
	}
}

func TestSetAPISecretRejectsEmptySecret(t *testing.T) {
	if err := setAPISecret(""); err == nil || !strings.Contains(err.Error(), "secret cannot be empty") {
		t.Fatalf("setAPISecret empty error = %v", err)
	}
}
